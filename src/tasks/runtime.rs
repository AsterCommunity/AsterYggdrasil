//! Runtime background task management.

use std::time::Duration;

use actix_web::web;
use chrono::Utc;
use tokio_util::sync::CancellationToken;

use crate::config::operations;
use crate::runtime::{AppState, SharedRuntimeState};
use crate::services::task_service::{RuntimeTaskRunOutcome, SystemRuntimeTaskKind};
use aster_forge_metrics::SharedMetricsRecorder;
use aster_forge_tasks::BackgroundTasks;

const MAINTENANCE_CLEANUP_JITTER_CAP: Duration = Duration::from_secs(30);
const BACKGROUND_TASK_RUNTIME_LEASE_ID: &str = "aster_yggdrasil.background_tasks";
const BACKGROUND_TASK_RUNTIME_LEASE_TTL: Duration = Duration::from_secs(30);
const BACKGROUND_TASK_RUNTIME_LEASE_RENEW_INTERVAL: Duration = Duration::from_secs(10);
const BACKGROUND_TASK_RUNTIME_LEASE_STANDBY_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const SCHEDULED_TASK_CLAIM_TTL: Duration = Duration::from_secs(120);

type BackgroundTasksComponent<F> =
    aster_forge_tasks::BackgroundTaskRuntimeDefinitionsComponentFromShutdown<
        SystemRuntimeTaskKind,
        crate::services::task_service::types::TaskPresentationCode,
        F,
    >;

/// Creates the background task component used by the product entrypoint.
pub fn background_tasks_component(
    state: web::Data<AppState>,
) -> BackgroundTasksComponent<impl FnOnce(CancellationToken) -> BackgroundTasks> {
    aster_forge_tasks::background_task_component_with_definitions_from_shutdown(
        crate::services::task_service::registered_system_runtime_tasks(),
        move |shutdown_token| spawn_runtime_background_tasks(state, shutdown_token),
    )
}

/// Creates the background task runtime component used by the product entrypoint.
pub fn task_component(
    background_tasks: BackgroundTasks,
) -> aster_forge_runtime::RuntimeComponentBundleRegistration<
    aster_forge_tasks::BackgroundTaskRuntimeDefinitionsComponent<
        SystemRuntimeTaskKind,
        crate::services::task_service::types::TaskPresentationCode,
    >,
> {
    aster_forge_tasks::background_task_component_with_definitions(
        background_tasks,
        crate::services::task_service::registered_system_runtime_tasks(),
    )
}

pub fn spawn_runtime_background_tasks(
    state: web::Data<AppState>,
    shutdown_token: CancellationToken,
) -> BackgroundTasks {
    tracing::info!("starting runtime background tasks");
    let mut tasks = build_background_tasks_base(state.metrics(), shutdown_token);
    let shutdown_token = tasks.shutdown_token();

    if state.get_ref().config_sync().enabled() {
        tasks.push(spawn_config_reload_subscription(
            shutdown_token.clone(),
            state.clone(),
        ));
    }

    tasks.push(run_background_runtime_group(shutdown_token.clone(), state));

    tasks
}

async fn run_background_runtime_group(
    shutdown_token: CancellationToken,
    state: web::Data<AppState>,
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
    .claim_ttl(SCHEDULED_TASK_CLAIM_TTL)
    .lease_ttl(BACKGROUND_TASK_RUNTIME_LEASE_TTL)
    .lease_renew_interval(BACKGROUND_TASK_RUNTIME_LEASE_RENEW_INTERVAL)
    .lease_standby_retry_interval(BACKGROUND_TASK_RUNTIME_LEASE_STANDBY_RETRY_INTERVAL);

    config
        .run(shutdown_token, |runtime| {
            runtime.worker(spawn_background_task_dispatcher);
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
        if let Err(error) =
            crate::services::config_service::runtime::run_config_reload_subscription(
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
    if let Err(error) = crate::services::task_service::record_scheduled_runtime_task_run(
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

async fn record_runtime_task_outcome(
    state: web::Data<AppState>,
    name: SystemRuntimeTaskKind,
    started_at: chrono::DateTime<Utc>,
    finished_at: chrono::DateTime<Utc>,
    outcome: RuntimeTaskRunOutcome,
) {
    let task_name = name.as_str();
    if let Err(error) = crate::services::task_service::record_runtime_task_run(
        state.get_ref(),
        name,
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
    match crate::services::mail_outbox_service::dispatch_due(state.get_ref()).await {
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
    let report = crate::services::health_service::run_system_health_checks(state.get_ref()).await;
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
    let system_health = crate::services::task_service::types::RuntimeSystemHealthResult {
        status,
        components: report
            .components
            .into_iter()
            .map(
                |component| crate::services::task_service::types::RuntimeSystemHealthComponent {
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
) -> crate::services::task_service::types::RuntimeSystemHealthStatus {
    match status {
        aster_forge_runtime::HealthStatus::Healthy => {
            crate::services::task_service::types::RuntimeSystemHealthStatus::Healthy
        }
        aster_forge_runtime::HealthStatus::Degraded => {
            crate::services::task_service::types::RuntimeSystemHealthStatus::Degraded
        }
        aster_forge_runtime::HealthStatus::Unhealthy => {
            crate::services::task_service::types::RuntimeSystemHealthStatus::Unhealthy
        }
    }
}

async fn run_auth_session_cleanup(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match crate::services::auth_service::cleanup_expired_auth_sessions(state.get_ref()).await {
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
    match crate::services::external_auth_service::cleanup_expired_flows(state.get_ref()).await {
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
    match crate::services::yggdrasil_service::cleanup_expired_or_revoked_tokens(state.get_ref())
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
    match crate::services::audit_service::cleanup_expired(state.get_ref()).await {
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
    match crate::services::task_service::cleanup_expired(state.get_ref()).await {
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
    match crate::services::texture_service::check_object_storage_consistency(state.get_ref()).await
    {
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
    report: &crate::services::texture_service::ObjectStorageConsistencyReport,
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
                crate::services::texture_service::ObjectStorageConsistencyIssueKind::MissingObject => {
                    "missing object"
                }
                crate::services::texture_service::ObjectStorageConsistencyIssueKind::HashMismatch => {
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
    let registration = match crate::services::texture_service::register_bound_textures_in_wardrobe(
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

    match crate::services::texture_service::cleanup_orphan_texture_blobs(state.get_ref()).await {
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

async fn spawn_background_task_dispatcher(
    shutdown_token: CancellationToken,
    state: web::Data<AppState>,
) {
    aster_forge_tasks::run_dispatch_worker(
        SystemRuntimeTaskKind::BackgroundTaskDispatch.as_str(),
        shutdown_token,
        state,
        |state| background_task_dispatch_interval(state.get_ref()),
        |state| background_task_dispatch_idle_max_interval(state.get_ref()),
        |state: web::Data<AppState>| async move {
            state.background_task_dispatch_wakeup().notified().await;
        },
        run_background_task_dispatch_iteration,
    )
    .await;
}

async fn run_background_task_dispatch_iteration(
    state: web::Data<AppState>,
    shutdown_token: CancellationToken,
) -> aster_forge_tasks::BackgroundTaskDispatchIteration {
    let iteration = std::sync::Arc::new(std::sync::Mutex::new(
        aster_forge_tasks::BackgroundTaskDispatchIteration::idle(),
    ));
    let iteration_for_record = iteration.clone();
    let iteration_for_panic = iteration.clone();

    aster_forge_tasks::run_recorded_task_iteration(
        SystemRuntimeTaskKind::BackgroundTaskDispatch,
        SystemRuntimeTaskKind::BackgroundTaskDispatch.as_str(),
        state,
        &move |state: web::Data<AppState>| {
            let shutdown_token = shutdown_token.clone();
            let iteration_for_record = iteration_for_record.clone();
            async move {
                let result = crate::services::task_service::dispatch::dispatch_due_with_shutdown(
                    state.get_ref(),
                    shutdown_token,
                )
                .await;
                let dispatch_iteration = match &result {
                    Ok(stats) if stats.has_activity() => {
                        aster_forge_tasks::BackgroundTaskDispatchIteration::active()
                    }
                    Ok(_) => aster_forge_tasks::BackgroundTaskDispatchIteration::idle(),
                    Err(_) => aster_forge_tasks::BackgroundTaskDispatchIteration::failed(),
                };
                match iteration_for_record.lock() {
                    Ok(mut stored) => *stored = dispatch_iteration,
                    Err(poisoned) => *poisoned.into_inner() = dispatch_iteration,
                }
                background_task_dispatch_outcome(result)
            }
        },
        &move |panic_message| {
            match iteration_for_panic.lock() {
                Ok(mut stored) => {
                    *stored = aster_forge_tasks::BackgroundTaskDispatchIteration::failed();
                }
                Err(poisoned) => {
                    *poisoned.into_inner() =
                        aster_forge_tasks::BackgroundTaskDispatchIteration::failed();
                }
            }
            RuntimeTaskRunOutcome::failed(Some("Task panicked".to_string()), panic_message)
        },
        &record_runtime_task_outcome,
    )
    .await;

    match iteration.lock() {
        Ok(stored) => *stored,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

fn background_task_dispatch_outcome(
    result: crate::errors::Result<aster_forge_tasks::DispatchStats>,
) -> RuntimeTaskRunOutcome {
    match result {
        Ok(stats) => {
            if stats.has_activity() {
                tracing::info!(
                    claimed = stats.claimed,
                    succeeded = stats.succeeded,
                    retried = stats.retried,
                    failed = stats.failed,
                    "processed background task batch"
                );
            }
            RuntimeTaskRunOutcome::quiet()
        }
        Err(error) => {
            tracing::warn!("background task dispatch failed: {error}");
            RuntimeTaskRunOutcome::failed(
                Some("Background task dispatch failed".to_string()),
                error.to_string(),
            )
        }
    }
}

fn build_background_tasks_base(
    metrics: &SharedMetricsRecorder,
    shutdown_token: CancellationToken,
) -> BackgroundTasks {
    let mut tasks = BackgroundTasks::with_shutdown_token(shutdown_token);
    if let Some(task) = metrics.system_metrics_updater_task(tasks.shutdown_token()) {
        tasks.push(task);
    }
    tasks
}

fn background_task_dispatch_interval(state: &impl SharedRuntimeState) -> Duration {
    Duration::from_secs(operations::background_task_dispatch_interval_secs(
        state.runtime_config(),
    ))
}

fn background_task_dispatch_idle_max_interval(state: &impl SharedRuntimeState) -> Duration {
    Duration::from_secs(operations::background_task_dispatch_idle_max_interval_secs(
        state.runtime_config(),
    ))
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
    use crate::services::texture_service::{
        ObjectStorageConsistencyIssue, ObjectStorageConsistencyIssueKind,
        ObjectStorageConsistencyReport,
    };
    use aster_forge_runtime::{HealthComponentReport, RuntimeComponentBundle, SystemHealthReport};

    #[test]
    fn task_component_registers_background_task_shutdown() {
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            task_component(BackgroundTasks::new()).register(registry);
        });

        let descriptor = registry
            .descriptor(aster_forge_tasks::BACKGROUND_TASKS_COMPONENT)
            .expect("background task component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Tasks
        );
        assert_eq!(
            descriptor
                .shutdown
                .expect("background task shutdown should be registered")
                .phase_name,
            aster_forge_tasks::BACKGROUND_TASKS_SHUTDOWN_PHASE
        );
        assert_eq!(
            descriptor
                .tasks
                .iter()
                .map(|task| task.task_name)
                .collect::<Vec<_>>(),
            crate::services::task_service::registered_system_runtime_tasks()
                .iter()
                .map(|task| task.wire_value)
                .collect::<Vec<_>>()
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
                    crate::services::task_service::types::RuntimeSystemHealthStatus::Healthy
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
                    crate::services::task_service::types::RuntimeSystemHealthStatus::Degraded
                );
                assert_eq!(
                    system_health.components[0].status,
                    crate::services::task_service::types::RuntimeSystemHealthStatus::Healthy
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
        let report = ObjectStorageConsistencyReport {
            checked: 2,
            missing: 1,
            hash_mismatched: 1,
            issues: vec![
                ObjectStorageConsistencyIssue {
                    texture_id: 41,
                    storage_key: "aa/missing.png".to_string(),
                    hash: "expected-missing-hash".to_string(),
                    kind: ObjectStorageConsistencyIssueKind::MissingObject,
                },
                ObjectStorageConsistencyIssue {
                    texture_id: 42,
                    storage_key: "bb/mismatch.png".to_string(),
                    hash: "expected-mismatch-hash".to_string(),
                    kind: ObjectStorageConsistencyIssueKind::HashMismatch,
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
