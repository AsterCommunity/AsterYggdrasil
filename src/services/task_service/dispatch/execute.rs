use std::time::Duration as StdDuration;

use chrono::Utc;
use tokio_util::sync::CancellationToken;

use super::{
    TASK_HEARTBEAT_INTERVAL_SECS, task_expiration_from, task_lease_expires_at, truncate_error,
};
use crate::db::repository::background_task_repo;
use crate::entities::background_task;
use crate::errors::{AsterError, Result};
use crate::runtime::{AppState, TaskRuntimeState};
use crate::services::task_service::{
    registry,
    steps::{parse_task_steps_json, serialize_task_steps},
};
use crate::types::{BackgroundTaskKind, BackgroundTaskStatus};
use aster_forge_tasks::{DispatchStats, TaskLease, mark_active_step_failed};

pub(super) async fn run_claimed_tasks(
    state: &AppState,
    claimed_tasks: Vec<(background_task::Model, TaskLease)>,
    shutdown_token: CancellationToken,
) -> Result<DispatchStats> {
    aster_forge_tasks::run_claimed_task_batch_with_store(
        BackgroundTaskExecutionStore {
            state: state.clone(),
        },
        claimed_tasks,
        |(task, _)| (task.created_at, task.id),
        shutdown_token,
        aster_forge_tasks::ClaimedTaskExecutionConfig {
            renewal_timeout: aster_forge_tasks::task_lease_renewal_timeout(
                super::super::TASK_PROCESSING_STALE_SECS,
                TASK_HEARTBEAT_INTERVAL_SECS,
            ),
            heartbeat_interval: StdDuration::from_secs(TASK_HEARTBEAT_INTERVAL_SECS),
            lease_expires_at: task_lease_expires_at,
            retry_delay_secs: aster_forge_tasks::default_task_retry_delay_secs,
        },
    )
    .await
}

impl aster_forge_tasks::ExecutableTaskRecord<BackgroundTaskKind> for background_task::Model {
    fn attempt_count(&self) -> i32 {
        self.attempt_count
    }

    fn max_attempts(&self) -> i32 {
        self.max_attempts
    }
}

#[derive(Clone)]
struct BackgroundTaskExecutionStore {
    state: AppState,
}

#[async_trait::async_trait]
impl aster_forge_tasks::TaskHeartbeatStore for BackgroundTaskExecutionStore {
    type Error = AsterError;

    async fn touch_task_heartbeat(
        &self,
        lease: TaskLease,
        now: chrono::DateTime<Utc>,
        lease_expires_at: chrono::DateTime<Utc>,
    ) -> Result<bool> {
        background_task_repo::touch_heartbeat(
            self.state.writer_db(),
            lease.task_id,
            lease.processing_token,
            now,
            lease_expires_at,
        )
        .await
    }
}

#[async_trait::async_trait]
impl aster_forge_tasks::ClaimedTaskExecutionStore<background_task::Model, BackgroundTaskKind>
    for BackgroundTaskExecutionStore
{
    async fn process_task(
        &self,
        task: &background_task::Model,
        context: aster_forge_tasks::TaskExecutionContext,
    ) -> Result<()> {
        registry::process_task(&self.state, task, context).await
    }

    fn is_lease_lost_error(&self, error: &AsterError) -> bool {
        super::super::is_task_lease_lost(error)
    }

    fn is_lease_renewal_timed_out_error(&self, error: &AsterError) -> bool {
        super::super::is_task_lease_renewal_timed_out(error)
    }

    fn is_worker_shutdown_requested_error(&self, error: &AsterError) -> bool {
        super::super::is_task_worker_shutdown_requested(error)
    }

    fn retry_class(
        &self,
        task: &background_task::Model,
        error: &AsterError,
    ) -> aster_forge_tasks::TaskRetryClass {
        registry::task_retry_class(task.kind, error)
    }

    fn storage_error(&self, error: &AsterError) -> String {
        truncate_error(error.message())
    }

    fn display_error(&self, storage_error: &str) -> String {
        storage_error.to_string()
    }

    async fn failed_steps_json(
        &self,
        task: &background_task::Model,
        display_error: &str,
    ) -> Option<String> {
        let latest = background_task_repo::find_by_id(self.state.writer_db(), task.id)
            .await
            .ok()?;
        let mut steps =
            parse_task_steps_json(latest.steps_json.as_ref().map(|raw| raw.as_ref())).ok()?;
        if steps.is_empty() {
            return None;
        }
        mark_active_step_failed(&mut steps, Some(display_error));
        serialize_task_steps(&steps).ok().map(Into::into)
    }

    async fn mark_task_failed(
        &self,
        task: &background_task::Model,
        lease: TaskLease,
        failure: aster_forge_tasks::TaskPermanentFailure<'_>,
    ) -> Result<bool> {
        background_task_repo::mark_failed(
            self.state.writer_db(),
            background_task_repo::TaskFailureUpdate {
                id: task.id,
                processing_token: lease.processing_token,
                attempt_count: failure.attempt_count,
                last_error: failure.storage_error,
                finished_at: failure.finished_at,
                expires_at: task_expiration_from(&self.state, failure.finished_at),
                steps_json: failure.failed_steps_json,
                failure_can_retry: failure.failure_can_retry,
            },
        )
        .await
    }

    async fn mark_task_retry(
        &self,
        task: &background_task::Model,
        lease: TaskLease,
        retry: aster_forge_tasks::TaskRetryUpdate<'_>,
    ) -> Result<bool> {
        background_task_repo::mark_retry(
            self.state.writer_db(),
            task.id,
            lease.processing_token,
            retry.attempt_count,
            retry.retry_at,
            retry.storage_error,
            retry.failed_steps_json,
        )
        .await
    }

    async fn release_task_for_shutdown(
        &self,
        task: &background_task::Model,
        lease: TaskLease,
    ) -> Result<bool> {
        background_task_repo::release_processing(
            self.state.writer_db(),
            task.id,
            lease.processing_token,
            Utc::now(),
            BackgroundTaskStatus::Retry,
        )
        .await
    }

    fn record_task_transition(&self, task: &background_task::Model, status: &'static str) {
        self.state
            .metrics()
            .record_background_task_transition(task.kind.as_str(), status);
    }

    fn wake_dispatcher(&self) {
        self.state.wake_background_task_dispatcher();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use migration::Migrator;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    use super::BackgroundTaskExecutionStore;
    use crate::entities::background_task;
    use crate::runtime::AppState;
    use crate::types::{BackgroundTaskKind, BackgroundTaskStatus, StoredTaskPayload};
    use aster_forge_tasks::{ClaimedTaskExecutionStore, TaskLease};

    async fn test_state() -> AppState {
        let db = crate::db::connect_with_metrics(
            &crate::config::DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("test database should connect");
        Migrator::up(&db, None)
            .await
            .expect("test database migration should run");
        crate::services::config_service::ensure_defaults(&db)
            .await
            .expect("system config defaults should be installed");
        let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
        runtime_config
            .reload(&db)
            .await
            .expect("runtime config should load");
        let cache = crate::cache::create_cache(&crate::config::CacheConfig {
            ..Default::default()
        })
        .await;
        let config = Arc::new(crate::config::Config::default());

        AppState {
            db_handles: aster_forge_db::DbHandles::single(db),
            config: config.clone(),
            runtime_config,
            cache,
            object_storage: crate::object_storage::create_object_storage(&config.object_storage)
                .expect("object storage should initialize"),
            mail_sender: crate::services::mail_service::memory_sender(),
            metrics: aster_forge_metrics::NoopMetrics::arc(),
            started_at: AppState::new_started_at(),
            yggdrasil_rate_limiter: AppState::new_yggdrasil_rate_limiter(&config),
            yggdrasil_session_forward_http_client:
                AppState::new_yggdrasil_session_forward_http_client()
                    .expect("Yggdrasil session forward HTTP client should build"),
            background_task_dispatch_wakeup: AppState::new_background_task_dispatch_wakeup(),
        }
    }

    #[tokio::test]
    async fn shutdown_release_returns_processing_task_to_retry_without_failure_update() {
        let state = test_state().await;
        let now = Utc::now();
        let task = background_task::ActiveModel {
            kind: Set(BackgroundTaskKind::SystemRuntime),
            status: Set(BackgroundTaskStatus::Processing),
            creator_user_id: Set(None),
            display_name: Set("Shutdown release task".to_string()),
            payload_json: Set(StoredTaskPayload(
                serde_json::json!({ "task_name": "task-cleanup" }).to_string(),
            )),
            result_json: Set(None),
            runtime_json: Set(None),
            steps_json: Set(None),
            progress_current: Set(0),
            progress_total: Set(1),
            status_text: Set(Some("in progress".to_string())),
            attempt_count: Set(2),
            max_attempts: Set(3),
            next_run_at: Set(now - Duration::seconds(30)),
            processing_token: Set(7),
            processing_started_at: Set(Some(now - Duration::seconds(20))),
            last_heartbeat_at: Set(Some(now - Duration::seconds(5))),
            lease_expires_at: Set(Some(now + Duration::seconds(30))),
            started_at: Set(Some(now - Duration::seconds(20))),
            finished_at: Set(None),
            last_error: Set(Some("previous failure".to_string())),
            failure_can_retry: Set(Some(true)),
            expires_at: Set(now + Duration::hours(24)),
            created_at: Set(now - Duration::hours(1)),
            updated_at: Set(now - Duration::seconds(5)),
            ..Default::default()
        }
        .insert(state.writer_db())
        .await
        .expect("processing task should insert");

        let store = BackgroundTaskExecutionStore {
            state: state.clone(),
        };
        store
            .release_task_for_shutdown(&task, TaskLease::new(task.id, 7))
            .await
            .expect("shutdown release should succeed");

        let released = background_task::Entity::find_by_id(task.id)
            .one(state.reader_db())
            .await
            .expect("released task should query")
            .expect("released task should exist");
        assert_eq!(released.status, BackgroundTaskStatus::Retry);
        assert_eq!(released.processing_token, 7);
        assert_eq!(released.attempt_count, 2);
        assert_eq!(released.last_error.as_deref(), Some("previous failure"));
        assert_eq!(released.failure_can_retry, Some(true));
        assert_eq!(released.status_text, None);
        assert_eq!(released.processing_started_at, None);
        assert_eq!(released.last_heartbeat_at, None);
        assert_eq!(released.lease_expires_at, None);
        assert_eq!(released.finished_at, None);
    }
}
