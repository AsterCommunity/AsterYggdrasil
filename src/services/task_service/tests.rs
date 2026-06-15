use super::*;
use crate::entities::background_task;
use crate::runtime::AppState;
use crate::services::task_service::admin::{build_task_info, validate_admin_task_cleanup_status};
use crate::services::task_service::create::create_typed_task_record;
use crate::services::task_service::lease::{
    is_task_lease_renewal_timed_out, is_task_worker_shutdown_requested, task_lease_lost,
};
use crate::services::task_service::runtime::SystemRuntimeTaskKind;
use crate::services::task_service::spec::SystemRuntimeTask;
use crate::services::task_service::types::{
    RuntimeTaskName, RuntimeTaskPayload, RuntimeTaskResult, TaskStepStatus,
};
use crate::types::{
    BackgroundTaskKind, BackgroundTaskStatus, StoredTaskPayload, SystemConfigSource,
    SystemConfigVisibility,
};
use sea_orm::{ActiveModelTrait, Set};
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio_util::sync::CancellationToken;

async fn test_state() -> AppState {
    let db_cfg = crate::config::DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        pool_size: 1,
        retry_count: 0,
    };
    let db = crate::db::connect_with_metrics(&db_cfg, crate::metrics_core::NoopMetrics::arc())
        .await
        .expect("task service test database should connect");
    migration::Migrator::up(&db, None)
        .await
        .expect("task service test migrations should run");
    crate::services::system_config_service::ensure_defaults(&db)
        .await
        .expect("task service test defaults should seed");

    let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
    runtime_config
        .reload(&db)
        .await
        .expect("task service runtime config should reload");

    let test_dir = format!("/tmp/asteryggdrasil-task-service-{}", uuid::Uuid::new_v4());
    let temp_dir = format!("{test_dir}/temp");
    std::fs::create_dir_all(&temp_dir).expect("task service temp dir should exist");

    let config = Arc::new(crate::config::Config {
        server: crate::config::ServerConfig {
            temp_dir,
            ..Default::default()
        },
        database: db_cfg,
        cache: crate::config::CacheConfig {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    });
    let cache = crate::cache::create_cache(&config.cache).await;
    let texture_storage = crate::texture_storage::create_texture_storage(&config.texture_storage)
        .expect("texture storage should initialize");
    let yggdrasil_rate_limiter = AppState::new_yggdrasil_rate_limiter(&config);

    AppState {
        db_handles: crate::db::DbHandles::single(db),
        config,
        runtime_config,
        cache,
        texture_storage,
        mail_sender: crate::services::mail_service::memory_sender(),
        metrics: crate::metrics_core::NoopMetrics::arc(),
        yggdrasil_rate_limiter,
        background_task_dispatch_wakeup: AppState::new_background_task_dispatch_wakeup(),
    }
}

fn task_model(
    status: BackgroundTaskStatus,
    payload_json: StoredTaskPayload,
) -> background_task::Model {
    let now = Utc::now();
    background_task::Model {
        id: 42,
        kind: BackgroundTaskKind::SystemRuntime,
        status,
        creator_user_id: None,
        display_name: "Task".to_string(),
        payload_json,
        result_json: None,
        runtime_json: None,
        steps_json: None,
        progress_current: 0,
        progress_total: 1,
        status_text: None,
        attempt_count: 0,
        max_attempts: 1,
        next_run_at: now,
        processing_token: 0,
        processing_started_at: None,
        last_heartbeat_at: None,
        lease_expires_at: None,
        started_at: None,
        finished_at: None,
        last_error: None,
        failure_can_retry: None,
        expires_at: now + Duration::hours(24),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn admin_cleanup_rejects_non_terminal_status() {
    assert!(validate_admin_task_cleanup_status(Some(BackgroundTaskStatus::Processing)).is_err());
    assert!(validate_admin_task_cleanup_status(Some(BackgroundTaskStatus::Succeeded)).is_ok());
}

#[test]
fn lease_control_errors_are_internal_messages_only() {
    let lease = TaskLease::new(1, 2);
    let lost = task_lease_lost(lease);
    assert!(is_task_lease_lost(&lost));
    assert!(!is_task_lease_renewal_timed_out(&lost));
    assert!(!is_task_worker_shutdown_requested(&lost));
}

#[tokio::test]
async fn typed_task_create_builds_active_model_with_truncation_and_runtime_defaults() {
    let state = test_state().await;
    state
        .runtime_config()
        .apply(crate::entities::system_config::Model {
            id: 999,
            key: operations::TASK_RETENTION_HOURS_KEY.to_string(),
            value: "48".to_string(),
            value_type: crate::types::SystemConfigValueType::Number,
            requires_restart: false,
            is_sensitive: false,
            source: SystemConfigSource::System,
            visibility: SystemConfigVisibility::Private,
            namespace: String::new(),
            category: String::new(),
            description: String::new(),
            updated_at: Utc::now(),
            updated_by: None,
        });

    let started_at = Utc::now() - Duration::minutes(2);
    let finished_at = Utc::now();
    let result =
        RuntimeTaskResult::from_timestamps(started_at, finished_at, Some("done".to_string()), None);
    let active = TypedTaskCreate::<SystemRuntimeTask>::new(
        "x".repeat(TASK_DISPLAY_NAME_MAX_LEN + 16),
        RuntimeTaskPayload {
            task_name: RuntimeTaskName::from(SystemRuntimeTaskKind::TaskCleanup),
        },
    )
    .creator_user_id(Some(7))
    .status(BackgroundTaskStatus::Succeeded)
    .progress(5, 10)
    .status_text("s".repeat(TASK_STATUS_TEXT_MAX_LEN + 16))
    .started_at(started_at)
    .finished_at(finished_at)
    .last_error(Some("e".repeat(TASK_LAST_ERROR_MAX_LEN + 16)))
    .failure_can_retry(Some(false))
    .result(&result)
    .unwrap()
    .into_active_model(&state)
    .unwrap();

    assert_eq!(active.kind.unwrap(), BackgroundTaskKind::SystemRuntime);
    assert_eq!(active.status.unwrap(), BackgroundTaskStatus::Succeeded);
    assert_eq!(active.creator_user_id.unwrap(), Some(7));
    assert_eq!(
        active.display_name.unwrap().len(),
        TASK_DISPLAY_NAME_MAX_LEN
    );
    assert_eq!(
        active.status_text.unwrap().unwrap().chars().count(),
        TASK_STATUS_TEXT_MAX_LEN
    );
    assert_eq!(
        active.last_error.unwrap().unwrap().chars().count(),
        TASK_LAST_ERROR_MAX_LEN
    );
    assert_eq!(active.progress_current.unwrap(), 5);
    assert_eq!(active.progress_total.unwrap(), 10);
    assert_eq!(active.max_attempts.unwrap(), 1);
    assert!(active.result_json.unwrap().is_some());
    assert!(active.steps_json.unwrap().is_some());
    assert_eq!(active.started_at.unwrap(), Some(started_at));
    assert_eq!(active.finished_at.unwrap(), Some(finished_at));
    assert_eq!(active.created_at.unwrap(), started_at);
    assert_eq!(active.updated_at.unwrap(), finished_at);
    assert_eq!(
        active.expires_at.unwrap(),
        finished_at + Duration::hours(48)
    );
}

#[tokio::test]
async fn create_typed_task_record_persists_pending_task_without_creator() {
    let state = test_state().await;
    let task = create_typed_task_record::<SystemRuntimeTask>(
        &state,
        "Runtime task",
        &RuntimeTaskPayload {
            task_name: RuntimeTaskName::from(SystemRuntimeTaskKind::AuditCleanup),
        },
        None,
    )
    .await
    .unwrap();

    assert_eq!(task.kind, BackgroundTaskKind::SystemRuntime);
    assert_eq!(task.status, BackgroundTaskStatus::Pending);
    assert_eq!(task.display_name, "Runtime task");
    assert_eq!(task.max_attempts, 1);
    assert_eq!(task.steps_json.as_ref().map(AsRef::as_ref), Some("[]"));
}

#[tokio::test]
async fn task_info_decodes_payload_steps_result_and_retryability() {
    let state = test_state().await;
    let steps = vec![TaskStepInfo {
        key: "step".to_string(),
        title: "Step".to_string(),
        status: TaskStepStatus::Succeeded,
        progress_current: 1,
        progress_total: 1,
        detail: Some("done".to_string()),
        started_at: Some(Utc::now()),
        finished_at: Some(Utc::now()),
    }];
    let result = RuntimeTaskResult {
        duration_ms: 10,
        summary: Some("failed summary".to_string()),
        system_health: None,
    };
    let mut active: background_task::ActiveModel = task_model(
        BackgroundTaskStatus::Failed,
        spec::serialize_payload::<SystemRuntimeTask>(&RuntimeTaskPayload {
            task_name: RuntimeTaskName::from(SystemRuntimeTaskKind::TaskCleanup),
        })
        .unwrap(),
    )
    .into();
    active.id = sea_orm::ActiveValue::NotSet;
    active.display_name = Set("Failed runtime task".to_string());
    active.result_json = Set(Some(
        spec::serialize_result::<SystemRuntimeTask>(&result).unwrap(),
    ));
    active.steps_json = Set(Some(serialize_task_steps(&steps).unwrap()));
    active.progress_current = Set(3);
    active.progress_total = Set(4);
    active.failure_can_retry = Set(Some(false));
    let task = active.insert(state.writer_db()).await.unwrap();

    let info = build_task_info(task, None).unwrap();
    assert_eq!(info.progress_percent, 75);
    assert!(!info.can_retry);
    assert_eq!(info.steps.len(), 1);
    assert!(matches!(
        info.payload,
        crate::services::task_service::types::TaskPayload::SystemRuntime(_)
    ));
    assert!(matches!(
        info.result,
        Some(crate::services::task_service::types::TaskResult::SystemRuntime(_))
    ));

    let succeeded = task_model(
        BackgroundTaskStatus::Succeeded,
        spec::serialize_payload::<SystemRuntimeTask>(&RuntimeTaskPayload {
            task_name: RuntimeTaskName::from(SystemRuntimeTaskKind::TaskCleanup),
        })
        .unwrap(),
    );
    let mut succeeded = succeeded;
    succeeded.progress_total = 0;
    assert_eq!(
        build_task_info(succeeded, None).unwrap().progress_percent,
        100
    );
}

#[tokio::test]
async fn lease_guard_reports_lost_timeout_and_shutdown_states() {
    let lease = TaskLease::new(10, 20);
    let lost_guard = TaskLeaseGuard::new(lease);
    let lost = lost_guard.mark_lost();
    assert!(is_task_lease_lost(&lost));
    assert!(is_task_lease_lost(&lost_guard.ensure_active().unwrap_err()));

    let timeout_guard = TaskLeaseGuard::with_renewal_timeout(lease, StdDuration::ZERO);
    let timeout = timeout_guard.ensure_active().unwrap_err();
    assert!(is_task_lease_renewal_timed_out(&timeout));

    let shutdown = CancellationToken::new();
    let context = TaskExecutionContext::new(lease, shutdown.clone());
    shutdown.cancel();
    let error = context.ensure_active().unwrap_err();
    assert!(is_task_worker_shutdown_requested(&error));
}

#[tokio::test]
async fn execution_context_sleep_and_shutdown_return_shutdown_errors() {
    let shutdown = CancellationToken::new();
    let context = TaskExecutionContext::new(TaskLease::new(11, 22), shutdown.clone());
    shutdown.cancel();

    let sleep_error = context
        .sleep_or_shutdown(StdDuration::from_secs(60))
        .await
        .unwrap_err();
    assert!(is_task_worker_shutdown_requested(&sleep_error));

    let context = TaskExecutionContext::new(TaskLease::new(12, 23), CancellationToken::new());
    context
        .sleep_or_shutdown(StdDuration::from_millis(1))
        .await
        .unwrap();
}

#[tokio::test]
async fn temp_dir_helpers_prepare_and_cleanup_token_and_task_directories() {
    let temp_root = format!("/tmp/asteryggdrasil-task-temp-{}", uuid::Uuid::new_v4());
    let lease = TaskLease::new(123, 456);

    let token_dir = prepare_task_temp_dir_in_root(&temp_root, lease)
        .await
        .unwrap();
    assert!(tokio::fs::try_exists(&token_dir).await.unwrap());

    cleanup_task_temp_dir_for_lease_in_root(&temp_root, lease)
        .await
        .unwrap();
    assert!(!tokio::fs::try_exists(&token_dir).await.unwrap());

    let task_dir = crate::utils::paths::task_temp_dir(&temp_root, lease.task_id);
    let other_token_dir = crate::utils::paths::task_token_temp_dir(&temp_root, lease.task_id, 999);
    tokio::fs::create_dir_all(&other_token_dir).await.unwrap();
    assert!(tokio::fs::try_exists(&task_dir).await.unwrap());

    cleanup_task_temp_dir_for_task_in_root(&temp_root, lease.task_id)
        .await
        .unwrap();
    assert!(!tokio::fs::try_exists(&task_dir).await.unwrap());
}

#[test]
fn truncation_helpers_preserve_utf8_boundaries_and_limits() {
    let long_display = format!("{}{}", "a".repeat(TASK_DISPLAY_NAME_MAX_LEN), "雪");
    assert_eq!(
        truncate_display_name(&long_display).len(),
        TASK_DISPLAY_NAME_MAX_LEN
    );
    assert_eq!(
        truncate_status_text(&"雪".repeat(TASK_STATUS_TEXT_MAX_LEN + 1))
            .chars()
            .count(),
        TASK_STATUS_TEXT_MAX_LEN
    );
    assert_eq!(
        truncate_error(&"e".repeat(TASK_LAST_ERROR_MAX_LEN + 1)).len(),
        TASK_LAST_ERROR_MAX_LEN
    );
}
