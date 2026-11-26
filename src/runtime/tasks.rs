//! Runtime background task management.

use std::time::Duration;

use actix_web::web;
use chrono::Utc;
use tokio_util::sync::CancellationToken;

use crate::config::operations;
use crate::runtime::{AppState, SharedRuntimeState};
use crate::services::ops::task::{RuntimeTaskRunOutcome, SystemRuntimeTaskKind};
use aster_forge_metrics::SharedMetricsRecorder;
use aster_forge_tasks::BackgroundTasks;

const MAINTENANCE_CLEANUP_JITTER_CAP: Duration = Duration::from_secs(30);
const BACKGROUND_TASK_RUNTIME_LEASE_ID: &str = "aster_yggdrasil.background_tasks";
const BACKGROUND_TASK_RUNTIME_LEASE_TTL: Duration = Duration::from_secs(30);
const BACKGROUND_TASK_RUNTIME_LEASE_RENEW_INTERVAL: Duration = Duration::from_secs(10);
const BACKGROUND_TASK_RUNTIME_LEASE_STANDBY_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const SCHEDULED_TASK_CLAIM_TTL: Duration = Duration::from_secs(120);

/// Independent component identity for the per-instance queue consumer group.
///
/// Process-owned maintenance (metrics updates, config reload) keeps the default
/// [`aster_forge_tasks::BACKGROUND_TASKS_COMPONENT`] name, so shutdown graphs can
/// order the consumer group against it explicitly.
pub const TASK_CONSUMERS_COMPONENT: &str = "task_consumers";

/// Independent component identity for the leased scheduled maintenance group.
///
/// Unlike [`TASK_CONSUMERS_COMPONENT`], only the instance holding the runtime
/// lease runs these tasks; the component exists so health reports and shutdown
/// graphs can address the group separately from queue consumers.
pub const SCHEDULED_TASKS_COMPONENT: &str = "scheduled_tasks";

type BackgroundTasksComponent<F> =
    aster_forge_tasks::BackgroundTaskRuntimeDefinitionsComponentFromShutdown<
        SystemRuntimeTaskKind,
        crate::services::ops::task::types::TaskPresentationCode,
        F,
    >;

/// Creates the process-owned maintenance component (metrics updates, runtime
/// config reload). Every serve role runs it because both API and worker
/// processes depend on fresh runtime configuration and system metrics.
pub fn maintenance_component(
    state: web::Data<AppState>,
) -> aster_forge_tasks::BackgroundTaskRuntimeComponentFromShutdown<
    impl FnOnce(CancellationToken) -> BackgroundTasks,
> {
    aster_forge_tasks::background_task_component_from_shutdown(move |shutdown_token| {
        spawn_maintenance_tasks(state, shutdown_token)
    })
}

/// Creates the per-instance queue consumer component. Every consuming role
/// instance runs it in parallel; claims are fenced by the claim transaction,
/// so throughput scales with instance count. API-only processes leave tasks
/// queued for workers.
pub fn task_consumers_component(
    state: web::Data<AppState>,
    consumer_statuses: Vec<aster_forge_tasks::TaskRuntimeStatus>,
) -> aster_forge_tasks::BackgroundTaskRuntimeComponentFromShutdown<
    impl FnOnce(CancellationToken) -> BackgroundTasks,
> {
    aster_forge_tasks::background_task_component_from_shutdown(move |shutdown_token| {
        spawn_queue_consumers(state, consumer_statuses, shutdown_token)
    })
    .named(TASK_CONSUMERS_COMPONENT)
}

/// Creates the leased scheduled maintenance component (the 10 maintenance
/// tasks). Only the runtime-lease holder executes them; standbys retry until
/// the lease is handed over.
pub fn scheduled_tasks_component(
    state: web::Data<AppState>,
    scheduled_status: aster_forge_tasks::TaskRuntimeStatus,
) -> BackgroundTasksComponent<impl FnOnce(CancellationToken) -> BackgroundTasks> {
    aster_forge_tasks::background_task_component_with_definitions_from_shutdown(
        crate::services::ops::task::registered_system_runtime_tasks(),
        move |shutdown_token| {
            spawn_scheduled_runtime_tasks(state, scheduled_status, shutdown_token)
        },
    )
    .named(SCHEDULED_TASKS_COMPONENT)
}

fn spawn_maintenance_tasks(
    state: web::Data<AppState>,
    shutdown_token: CancellationToken,
) -> BackgroundTasks {
    let budget = state.get_ref().config.worker.consumer_config();
    let shutdown_grace =
        budget.drain_timeout + budget.cancellation_timeout + Duration::from_secs(5);
    tracing::info!(
        shutdown_grace_secs = shutdown_grace.as_secs(),
        "starting process maintenance tasks"
    );
    let mut tasks = build_background_tasks_base(state.metrics(), shutdown_token, shutdown_grace);
    let shutdown_token = tasks.shutdown_token();

    if state.get_ref().config_sync().enabled() {
        tasks.push(spawn_config_reload_subscription(
            shutdown_token.clone(),
            state.clone(),
        ));
    }

    tasks
}

fn spawn_queue_consumers(
    state: web::Data<AppState>,
    consumer_statuses: Vec<aster_forge_tasks::TaskRuntimeStatus>,
    shutdown_token: CancellationToken,
) -> BackgroundTasks {
    let budget = state.get_ref().config.worker.consumer_config();
    let shutdown_grace =
        budget.drain_timeout + budget.cancellation_timeout + Duration::from_secs(5);
    tracing::info!(
        shutdown_grace_secs = shutdown_grace.as_secs(),
        lanes = consumer_statuses.len(),
        "starting queue task consumers"
    );
    let mut tasks = BackgroundTasks::with_shutdown_token_and_grace(shutdown_token, shutdown_grace);
    crate::services::ops::task::spawn_consumers(
        state,
        consumer_statuses,
        tasks.shutdown_token(),
        &mut tasks,
    );
    tasks
}

fn spawn_scheduled_runtime_tasks(
    state: web::Data<AppState>,
    scheduled_status: aster_forge_tasks::TaskRuntimeStatus,
    shutdown_token: CancellationToken,
) -> BackgroundTasks {
    let budget = state.get_ref().config.worker.consumer_config();
    let shutdown_grace =
        budget.drain_timeout + budget.cancellation_timeout + Duration::from_secs(5);
    tracing::info!(
        shutdown_grace_secs = shutdown_grace.as_secs(),
        "starting leased scheduled runtime tasks"
    );
    let mut tasks = BackgroundTasks::with_shutdown_token_and_grace(shutdown_token, shutdown_grace);
    tasks.push(run_scheduled_runtime_group(
        tasks.shutdown_token(),
        state,
        scheduled_status,
    ));
    tasks
}

async fn run_scheduled_runtime_group(
    shutdown_token: CancellationToken,
    state: web::Data<AppState>,
    scheduled_status: aster_forge_tasks::TaskRuntimeStatus,
) {
    let config = aster_forge_tasks::LeasedScheduledRuntimeConfig::new(
        "aster_yggdrasil",
        BACKGROUND_TASK_RUNTIME_LEASE_ID,
        aster_forge_db::RuntimeLeaseDbStore::new(state.writer_db().clone()),
        aster_forge_db::ScheduledTaskDbStore::new(state.writer_db().clone()),
        state,
        |panic_message| {
            RuntimeTaskRunOutcome::failed(Some("Task panicked".to_string()), panic_message)
        },
        record_scheduled_task_outcome,
    )
    .status(scheduled_status)
    .claim_ttl(SCHEDULED_TASK_CLAIM_TTL)
    .lease_ttl(BACKGROUND_TASK_RUNTIME_LEASE_TTL)
    .lease_renew_interval(BACKGROUND_TASK_RUNTIME_LEASE_RENEW_INTERVAL)
    .lease_standby_retry_interval(BACKGROUND_TASK_RUNTIME_LEASE_STANDBY_RETRY_INTERVAL);

    config
        .run(shutdown_token, |runtime| {
            runtime.scheduled(
                SystemRuntimeTaskKind::MailOutboxDispatch,
                mail_outbox_dispatch_interval_for_web_state,
                None,
                run_mail_outbox_dispatch,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::SystemHealthCheck,
                maintenance_cleanup_interval_for_web_state,
                None,
                run_system_health_check,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::AuthSessionCleanup,
                maintenance_cleanup_interval_for_web_state,
                Some(MAINTENANCE_CLEANUP_JITTER_CAP),
                run_auth_session_cleanup,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::ExternalAuthFlowCleanup,
                maintenance_cleanup_interval_for_web_state,
                Some(MAINTENANCE_CLEANUP_JITTER_CAP),
                run_external_auth_flow_cleanup,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::YggdrasilTokenCleanup,
                maintenance_cleanup_interval_for_web_state,
                Some(MAINTENANCE_CLEANUP_JITTER_CAP),
                run_yggdrasil_token_cleanup,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::AuditCleanup,
                maintenance_cleanup_interval_for_web_state,
                Some(MAINTENANCE_CLEANUP_JITTER_CAP),
                run_audit_cleanup,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::TaskCleanup,
                maintenance_cleanup_interval_for_web_state,
                Some(MAINTENANCE_CLEANUP_JITTER_CAP),
                run_task_cleanup,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::YggdrasilStorageConsistencyCheck,
                maintenance_cleanup_interval_for_web_state,
                Some(MAINTENANCE_CLEANUP_JITTER_CAP),
                run_yggdrasil_storage_consistency_check,
            );
            runtime.scheduled(
                SystemRuntimeTaskKind::YggdrasilTextureCleanup,
                maintenance_cleanup_interval_for_web_state,
                Some(MAINTENANCE_CLEANUP_JITTER_CAP),
                run_yggdrasil_texture_cleanup,
            );
        })
        .await;
}

fn spawn_config_reload_subscription(
    shutdown_token: CancellationToken,
    state: web::Data<AppState>,
) -> impl Future<Output = ()> + Send + 'static {
    let settings = state.config_sync().clone();
    let state = state.into_inner();
    async move {
        if let Err(error) = crate::services::ops::config::runtime::run_config_reload_subscription(
            state,
            settings,
            shutdown_token,
        )
        .await
        {
            tracing::warn!(
                error = %error,
                "runtime config reload subscription stopped"
            );
        }
    }
}

async fn record_scheduled_task_outcome(
    state: web::Data<AppState>,
    name: SystemRuntimeTaskKind,
    claim: aster_forge_tasks::ScheduledTaskClaim,
    started_at: chrono::DateTime<Utc>,
    finished_at: chrono::DateTime<Utc>,
    outcome: RuntimeTaskRunOutcome,
) {
    let task_name = name.as_str();
    if let Err(error) = crate::services::ops::task::record_scheduled_runtime_task_run(
        state.get_ref(),
        name,
        claim.scheduled_at,
        started_at,
        finished_at,
        &outcome,
    )
    .await
    {
        tracing::warn!("failed to record runtime task '{task_name}': {error}");
    }
}

async fn run_mail_outbox_dispatch(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match crate::services::mail::outbox::dispatch_due(state.get_ref()).await {
        Ok(stats)
            if stats.claimed > 0 || stats.sent > 0 || stats.retried > 0 || stats.failed > 0 =>
        {
            tracing::info!(
                claimed = stats.claimed,
                sent = stats.sent,
                retried = stats.retried,
                failed = stats.failed,
                "processed mail outbox batch"
            );
            RuntimeTaskRunOutcome::succeeded(Some(format!(
                "claimed {}, sent {}, retried {}, failed {}",
                stats.claimed, stats.sent, stats.retried, stats.failed
            )))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => {
            tracing::warn!("mail outbox dispatch failed: {error}");
            RuntimeTaskRunOutcome::failed(
                Some("Mail outbox dispatch failed".to_string()),
                error.to_string(),
            )
        }
    }
}

async fn run_system_health_check(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    let report = crate::services::ops::health::run_system_health_checks(state.get_ref()).await;
    system_health_outcome(report)
}

fn system_health_outcome(report: aster_forge_runtime::SystemHealthReport) -> RuntimeTaskRunOutcome {
    let has_issues = report.has_issues();
    let summary = if has_issues {
        report.issue_summary()
    } else {
        "system healthy".to_string()
    };
    let error = has_issues.then(|| report.issue_details());
    let status = runtime_health_status(report.status());
    let system_health = crate::services::ops::task::types::RuntimeSystemHealthResult {
        status,
        components: report
            .components
            .into_iter()
            .map(
                |component| crate::services::ops::task::types::RuntimeSystemHealthComponent {
                    name: component.name.to_string(),
                    status: runtime_health_status(component.status),
                    message: component.message,
                    details: component.details,
                },
            )
            .collect(),
    };
    if let Some(error) = error {
        RuntimeTaskRunOutcome::failed_with_system_health(Some(summary), error, system_health)
    } else {
        RuntimeTaskRunOutcome::succeeded_with_system_health(Some(summary), system_health)
    }
}

fn runtime_health_status(
    status: aster_forge_runtime::HealthStatus,
) -> crate::services::ops::task::types::RuntimeSystemHealthStatus {
    match status {
        aster_forge_runtime::HealthStatus::Healthy => {
            crate::services::ops::task::types::RuntimeSystemHealthStatus::Healthy
        }
        aster_forge_runtime::HealthStatus::Degraded => {
            crate::services::ops::task::types::RuntimeSystemHealthStatus::Degraded
        }
        aster_forge_runtime::HealthStatus::Unhealthy => {
            crate::services::ops::task::types::RuntimeSystemHealthStatus::Unhealthy
        }
    }
}

async fn run_auth_session_cleanup(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match crate::services::auth::local::cleanup_expired_auth_sessions(state.get_ref()).await {
        Ok(count) if count > 0 => {
            tracing::info!(count, "cleaned up expired auth sessions");
            RuntimeTaskRunOutcome::succeeded(Some(format!(
                "cleaned up {count} expired auth sessions"
            )))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("Auth session cleanup failed".to_string()),
            error.to_string(),
        ),
    }
}

async fn run_external_auth_flow_cleanup(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match crate::services::auth::external::cleanup_expired_flows(state.get_ref()).await {
        Ok(count) if count > 0 => {
            tracing::info!(count, "cleaned up expired external auth flows");
            RuntimeTaskRunOutcome::succeeded(Some(format!(
                "cleaned up {count} expired external auth flows"
            )))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("External auth flow cleanup failed".to_string()),
            error.to_string(),
        ),
    }
}

async fn run_yggdrasil_token_cleanup(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match crate::services::yggdrasil::protocol::cleanup_expired_or_revoked_tokens(state.get_ref())
        .await
    {
        Ok(count) if count > 0 => {
            tracing::info!(count, "cleaned up expired or revoked Yggdrasil tokens");
            RuntimeTaskRunOutcome::succeeded(Some(format!(
                "cleaned up {count} expired or revoked Yggdrasil tokens"
            )))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("Yggdrasil token cleanup failed".to_string()),
            error.to_string(),
        ),
    }
}

async fn run_audit_cleanup(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match crate::services::ops::audit::cleanup_expired(state.get_ref()).await {
        Ok(count) if count > 0 => {
            tracing::info!(count, "cleaned up expired audit logs");
            RuntimeTaskRunOutcome::succeeded(Some(format!("cleaned up {count} expired audit logs")))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("Audit cleanup failed".to_string()),
            error.to_string(),
        ),
    }
}

async fn run_task_cleanup(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match crate::services::ops::task::cleanup_expired(state.get_ref()).await {
        Ok(count) if count > 0 => {
            tracing::info!(count, "cleaned up expired task temp dirs");
            RuntimeTaskRunOutcome::succeeded(Some(format!(
                "cleaned up {count} expired task temp dirs"
            )))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("Task cleanup failed".to_string()),
            error.to_string(),
        ),
    }
}

async fn run_yggdrasil_storage_consistency_check(
    state: web::Data<AppState>,
) -> RuntimeTaskRunOutcome {
    match crate::services::texture::check_texture_storage_consistency(state.get_ref()).await {
        Ok(report) if report.missing > 0 || report.hash_mismatched > 0 => {
            let summary = yggdrasil_storage_consistency_failure_summary(&report);
            tracing::warn!(
                checked = report.checked,
                missing = report.missing,
                hash_mismatched = report.hash_mismatched,
                issues = %summary,
                "Yggdrasil object storage consistency issues found"
            );
            RuntimeTaskRunOutcome::failed(Some(summary.clone()), summary)
        }
        Ok(report) if report.checked > 0 => RuntimeTaskRunOutcome::succeeded(Some(format!(
            "checked {} object storage records",
            report.checked
        ))),
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("Yggdrasil storage consistency check failed".to_string()),
            error.to_string(),
        ),
    }
}

fn yggdrasil_storage_consistency_failure_summary(
    report: &crate::services::texture::TextureStoreConsistencyReport,
) -> String {
    const MAX_ISSUES_IN_SUMMARY: usize = 5;

    let mut summary = format!(
        "checked {}, missing {}, hash/key mismatched {} texture blobs",
        report.checked, report.missing, report.hash_mismatched
    );

    if report.issues.is_empty() {
        return summary;
    }

    let issue_details = report
        .issues
        .iter()
        .take(MAX_ISSUES_IN_SUMMARY)
        .map(|issue| {
            let kind = match issue.kind {
                crate::services::texture::TextureStoreConsistencyIssueKind::MissingObject => {
                    "missing object"
                }
                crate::services::texture::TextureStoreConsistencyIssueKind::HashMismatch => {
                    "hash/key mismatch"
                }
            };
            format!(
                "{kind}: texture #{}, key {}, expected hash {}",
                issue.texture_id, issue.storage_key, issue.hash
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    summary.push_str(": ");
    summary.push_str(&issue_details);

    let remaining = report.issues.len().saturating_sub(MAX_ISSUES_IN_SUMMARY);
    if remaining > 0 {
        summary.push_str(&format!("; and {remaining} more"));
    }

    summary
}

async fn run_yggdrasil_texture_cleanup(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    let registration = match crate::services::texture::register_bound_textures_in_wardrobe(
        state.get_ref(),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            return RuntimeTaskRunOutcome::failed(
                Some("Yggdrasil texture cleanup failed".to_string()),
                error.protocol_message(),
            );
        }
    };

    match crate::services::texture::cleanup_orphan_texture_blobs(state.get_ref()).await {
        Ok(result)
            if registration.converted_textures > 0
                || registration.rebound_bindings > 0
                || registration.removed_duplicate_textures > 0
                || result.deleted > 0 =>
        {
            tracing::info!(
                scanned_bindings = registration.scanned_bindings,
                converted_textures = registration.converted_textures,
                rebound_bindings = registration.rebound_bindings,
                removed_duplicate_textures = registration.removed_duplicate_textures,
                scanned = result.scanned,
                deleted = result.deleted,
                skipped = result.skipped,
                "cleaned up orphan Yggdrasil texture blobs"
            );
            RuntimeTaskRunOutcome::succeeded(Some(format!(
                "registered {} bound textures, rebound {}, removed {} duplicates; scanned {}, deleted {}, skipped {} texture blobs",
                registration.converted_textures,
                registration.rebound_bindings,
                registration.removed_duplicate_textures,
                result.scanned,
                result.deleted,
                result.skipped
            )))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("Yggdrasil texture cleanup failed".to_string()),
            error.to_string(),
        ),
    }
}

fn build_background_tasks_base(
    metrics: &SharedMetricsRecorder,
    shutdown_token: CancellationToken,
    shutdown_grace: Duration,
) -> BackgroundTasks {
    // Forge registers this component's shutdown phase without a timeout, so the
    // product-configurable budget is the join grace: drain plus cancellation,
    // plus a small margin for the abort path, before remaining workers stop.
    let mut tasks =
        BackgroundTasks::with_shutdown_token(shutdown_token).with_shutdown_grace(shutdown_grace);
    if let Some(task) = metrics.system_metrics_updater_task(tasks.shutdown_token()) {
        tasks.push(task);
    }
    tasks
}

fn maintenance_cleanup_interval(state: &impl SharedRuntimeState) -> Duration {
    Duration::from_secs(operations::maintenance_cleanup_interval_secs(
        state.runtime_config(),
    ))
}

fn maintenance_cleanup_interval_for_web_state(state: &web::Data<AppState>) -> Duration {
    maintenance_cleanup_interval(state.get_ref())
}

fn mail_outbox_dispatch_interval(state: &impl SharedRuntimeState) -> Duration {
    Duration::from_secs(operations::mail_outbox_dispatch_interval_secs(
        state.runtime_config(),
    ))
}

fn mail_outbox_dispatch_interval_for_web_state(state: &web::Data<AppState>) -> Duration {
    mail_outbox_dispatch_interval(state.get_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::texture::{
        TextureStoreConsistencyIssue, TextureStoreConsistencyIssueKind,
        TextureStoreConsistencyReport,
    };
    use aster_forge_runtime::{HealthComponentReport, RuntimeComponentBundle, SystemHealthReport};

    #[test]
    fn queue_and_scheduled_components_register_as_separate_task_groups() {
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            aster_forge_runtime::runtime_component(
                aster_forge_tasks::BackgroundTaskRuntimeComponent::new(BackgroundTasks::new())
                    .named(TASK_CONSUMERS_COMPONENT),
            )
            .register(registry);
            aster_forge_runtime::runtime_component(
                aster_forge_tasks::BackgroundTaskRuntimeDefinitionsComponent::new(
                    BackgroundTasks::new(),
                    crate::services::ops::task::registered_system_runtime_tasks(),
                )
                .named(SCHEDULED_TASKS_COMPONENT),
            )
            .register(registry);
        });

        let queue = registry
            .descriptor(TASK_CONSUMERS_COMPONENT)
            .expect("queue consumer component should be registered");
        assert_eq!(queue.kind, aster_forge_runtime::RuntimeComponentKind::Tasks);
        assert_eq!(queue.shutdown.len(), 1);
        assert_eq!(
            queue.shutdown[0].phase_name,
            aster_forge_tasks::BACKGROUND_TASKS_SHUTDOWN_PHASE
        );
        assert!(
            queue.tasks.is_empty(),
            "queue consumers are per-instance runners without scheduled definitions"
        );

        let scheduled = registry
            .descriptor(SCHEDULED_TASKS_COMPONENT)
            .expect("scheduled component should be registered");
        assert_eq!(
            scheduled.kind,
            aster_forge_runtime::RuntimeComponentKind::Tasks
        );
        assert_eq!(
            scheduled
                .tasks
                .iter()
                .map(|task| task.task_name)
                .collect::<Vec<_>>(),
            crate::services::ops::task::registered_system_runtime_tasks()
                .iter()
                .map(|task| task.wire_value)
                .collect::<Vec<_>>()
        );
        assert_ne!(
            TASK_CONSUMERS_COMPONENT, SCHEDULED_TASKS_COMPONENT,
            "the two groups must stay independently addressable"
        );
    }

    #[tokio::test]
    async fn shutdown_only_awaits_each_handle_once() {
        let mut tasks = BackgroundTasks::new();
        tasks.push(async {});

        tasks.shutdown().await;
    }

    #[tokio::test]
    async fn external_shutdown_token_stops_background_worker_before_shutdown_join() {
        let shutdown_token = CancellationToken::new();
        let mut tasks = BackgroundTasks::with_shutdown_token(shutdown_token.clone());
        let (stopped_tx, stopped_rx) = tokio::sync::oneshot::channel();

        tasks.push({
            let shutdown_token = shutdown_token.clone();
            async move {
                shutdown_token.cancelled().await;
                let _ = stopped_tx.send(());
            }
        });

        shutdown_token.cancel();
        tokio::time::timeout(Duration::from_millis(50), stopped_rx)
            .await
            .expect("background worker should observe external shutdown")
            .expect("background worker should report shutdown");

        tasks.shutdown().await;
    }

    #[test]
    fn system_health_outcome_succeeds_when_report_is_healthy() {
        let outcome = system_health_outcome(SystemHealthReport::new(vec![
            HealthComponentReport::healthy(
                aster_forge_db::DATABASE_HEALTH_CHECK,
                "database ping succeeded",
            ),
            HealthComponentReport::healthy("cache", "cache health check succeeded"),
        ]));

        match outcome {
            RuntimeTaskRunOutcome::Succeeded {
                summary,
                system_health: Some(system_health),
            } => {
                assert_eq!(summary.as_deref(), Some("system healthy"));
                assert_eq!(
                    system_health.status,
                    crate::services::ops::task::types::RuntimeSystemHealthStatus::Healthy
                );
                assert_eq!(system_health.components.len(), 2);
            }
            other => panic!("expected healthy system health success, got {other:?}"),
        }
    }

    #[test]
    fn system_health_outcome_fails_when_report_has_issues() {
        let outcome = system_health_outcome(SystemHealthReport::new(vec![
            HealthComponentReport::healthy(
                aster_forge_db::DATABASE_HEALTH_CHECK,
                "database ping succeeded",
            ),
            HealthComponentReport::degraded("cache", "memory fallback active")
                .with_detail("active_backend", "memory"),
        ]));

        match outcome {
            RuntimeTaskRunOutcome::Failed {
                summary,
                error,
                system_health: Some(system_health),
            } => {
                assert_eq!(summary.as_deref(), Some("cache degraded"));
                assert_eq!(error, "cache=degraded: memory fallback active");
                assert_eq!(
                    system_health.status,
                    crate::services::ops::task::types::RuntimeSystemHealthStatus::Degraded
                );
                assert_eq!(
                    system_health.components[0].status,
                    crate::services::ops::task::types::RuntimeSystemHealthStatus::Healthy
                );
                assert_eq!(system_health.components[1].details.len(), 1);
                assert_eq!(system_health.components[1].details[0].key, "active_backend");
                assert_eq!(
                    system_health.components[1].details[0].value,
                    aster_forge_runtime::HealthComponentDetailValue::Text("memory".to_string())
                );
            }
            other => panic!("expected unhealthy system health failure, got {other:?}"),
        }
    }

    #[test]
    fn storage_consistency_failure_summary_includes_issue_details() {
        let report = TextureStoreConsistencyReport {
            checked: 2,
            missing: 1,
            hash_mismatched: 1,
            issues: vec![
                TextureStoreConsistencyIssue {
                    texture_id: 41,
                    storage_key: "aa/missing.png".to_string(),
                    hash: "expected-missing-hash".to_string(),
                    kind: TextureStoreConsistencyIssueKind::MissingObject,
                },
                TextureStoreConsistencyIssue {
                    texture_id: 42,
                    storage_key: "bb/mismatch.png".to_string(),
                    hash: "expected-mismatch-hash".to_string(),
                    kind: TextureStoreConsistencyIssueKind::HashMismatch,
                },
            ],
        };

        let summary = yggdrasil_storage_consistency_failure_summary(&report);

        assert!(summary.contains("checked 2, missing 1, hash/key mismatched 1 texture blobs"));
        assert!(summary.contains(
            "missing object: texture #41, key aa/missing.png, expected hash expected-missing-hash"
        ));
        assert!(summary.contains(
            "hash/key mismatch: texture #42, key bb/mismatch.png, expected hash expected-mismatch-hash"
        ));
    }
}
