use chrono::Utc;
use sea_orm::{ConnectionTrait, Set};

use super::{
    BackgroundTaskSpec, registry, serialize_task_steps, spec, task_expiration_from,
    truncate_display_name, truncate_error, truncate_status_text,
};
use crate::db::repository::background_task_repo;
use crate::errors::Result;
use crate::runtime::{RuntimeConfigRuntimeState, TaskRuntimeState};
use aster_yggdrasil_model::entities::background_task;
use aster_yggdrasil_model::types::task::{BackgroundTaskStatus, StoredTaskResult, StoredTaskSteps};

pub(in crate::services::task_service) struct TypedTaskCreate<S: BackgroundTaskSpec> {
    display_name: String,
    payload: S::Payload,
    dedupe_key: Option<String>,
    creator_user_id: Option<i64>,
    status: BackgroundTaskStatus,
    result_json: Option<StoredTaskResult>,
    include_steps: bool,
    progress_current: i64,
    progress_total: i64,
    status_text: Option<String>,
    next_run_at: chrono::DateTime<Utc>,
    started_at: Option<chrono::DateTime<Utc>>,
    finished_at: Option<chrono::DateTime<Utc>>,
    last_error: Option<String>,
    failure_can_retry: Option<bool>,
    expires_at_anchor: chrono::DateTime<Utc>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl<S: BackgroundTaskSpec> TypedTaskCreate<S> {
    pub(in crate::services::task_service) fn new(
        display_name: impl Into<String>,
        payload: S::Payload,
    ) -> Self {
        let now = Utc::now();
        Self {
            display_name: display_name.into(),
            payload,
            dedupe_key: None,
            creator_user_id: None,
            status: BackgroundTaskStatus::Pending,
            result_json: None,
            include_steps: true,
            progress_current: 0,
            progress_total: 0,
            status_text: None,
            next_run_at: now,
            started_at: None,
            finished_at: None,
            last_error: None,
            failure_can_retry: None,
            expires_at_anchor: now,
            created_at: now,
            updated_at: now,
        }
    }

    pub(in crate::services::task_service) fn creator_user_id(
        mut self,
        creator_user_id: Option<i64>,
    ) -> Self {
        self.creator_user_id = creator_user_id;
        self
    }

    pub(in crate::services::task_service) fn dedupe_key(
        mut self,
        dedupe_key: aster_forge_tasks::TaskDedupeKey,
    ) -> Self {
        self.dedupe_key = Some(dedupe_key.into_string());
        self
    }

    pub(in crate::services::task_service) fn dedupe_key_str(
        self,
        dedupe_key: impl Into<String>,
    ) -> Result<Self> {
        Ok(self.dedupe_key(aster_forge_tasks::TaskDedupeKey::new(dedupe_key)?))
    }

    pub(in crate::services::task_service) fn next_run_at(
        mut self,
        next_run_at: chrono::DateTime<Utc>,
    ) -> Self {
        self.next_run_at = next_run_at;
        self.expires_at_anchor = next_run_at;
        self
    }

    pub(in crate::services::task_service) fn progress(mut self, current: i64, total: i64) -> Self {
        self.progress_current = current;
        self.progress_total = total;
        self
    }

    pub(in crate::services::task_service) fn status_text(mut self, status_text: String) -> Self {
        self.status_text = Some(status_text);
        self
    }

    pub(in crate::services::task_service) fn status(
        mut self,
        status: BackgroundTaskStatus,
    ) -> Self {
        self.status = status;
        self
    }

    pub(in crate::services::task_service) fn result(mut self, result: &S::Result) -> Result<Self> {
        self.result_json = Some(spec::serialize_result::<S>(result)?);
        Ok(self)
    }

    pub(in crate::services::task_service) fn without_steps(mut self) -> Self {
        self.include_steps = false;
        self
    }

    pub(in crate::services::task_service) fn started_at(
        mut self,
        started_at: chrono::DateTime<Utc>,
    ) -> Self {
        self.started_at = Some(started_at);
        self.created_at = started_at;
        self
    }

    pub(in crate::services::task_service) fn finished_at(
        mut self,
        finished_at: chrono::DateTime<Utc>,
    ) -> Self {
        self.finished_at = Some(finished_at);
        self.next_run_at = finished_at;
        self.expires_at_anchor = finished_at;
        self.updated_at = finished_at;
        self
    }

    pub(in crate::services::task_service) fn last_error(
        mut self,
        last_error: Option<String>,
    ) -> Self {
        self.last_error = last_error;
        self
    }

    pub(in crate::services::task_service) fn failure_can_retry(
        mut self,
        failure_can_retry: Option<bool>,
    ) -> Self {
        self.failure_can_retry = failure_can_retry;
        self
    }

    fn steps_json(&self) -> Result<Option<StoredTaskSteps>> {
        if self.include_steps {
            serialize_task_steps(&registry::initial_task_steps(S::KIND)).map(Some)
        } else {
            Ok(None)
        }
    }

    pub(in crate::services::task_service) fn into_active_model(
        self,
        state: &impl RuntimeConfigRuntimeState,
    ) -> Result<background_task::ActiveModel> {
        let payload_json = spec::serialize_payload::<S>(&self.payload)?;
        let steps_json = self.steps_json()?;

        Ok(background_task::ActiveModel {
            kind: Set(S::KIND),
            status: Set(self.status),
            creator_user_id: Set(self.creator_user_id),
            display_name: Set(truncate_display_name(&self.display_name)),
            dedupe_key: Set(self.dedupe_key),
            payload_json: Set(payload_json),
            result_json: Set(self.result_json),
            runtime_json: Set(None),
            steps_json: Set(steps_json),
            progress_current: Set(self.progress_current),
            progress_total: Set(self.progress_total),
            status_text: Set(self.status_text.as_deref().map(truncate_status_text)),
            attempt_count: Set(0),
            max_attempts: Set(registry::max_attempts(
                state.runtime_config().as_ref(),
                S::KIND,
            )),
            next_run_at: Set(self.next_run_at),
            processing_token: Set(0),
            processing_started_at: Set(None),
            last_heartbeat_at: Set(None),
            lease_expires_at: Set(None),
            started_at: Set(self.started_at),
            finished_at: Set(self.finished_at),
            last_error: Set(self.last_error.as_deref().map(truncate_error)),
            failure_can_retry: Set(self.failure_can_retry),
            expires_at: Set(task_expiration_from(state, self.expires_at_anchor)),
            created_at: Set(self.created_at),
            updated_at: Set(self.updated_at),
            ..Default::default()
        })
    }
}

pub(in crate::services::task_service) async fn insert_typed_task_record<
    C: ConnectionTrait,
    S: BackgroundTaskSpec,
>(
    state: &impl RuntimeConfigRuntimeState,
    db: &C,
    request: TypedTaskCreate<S>,
) -> Result<background_task::Model> {
    let dedupe_key = request.dedupe_key.clone();
    let active = request.into_active_model(state)?;
    tracing::debug!("inserting typed background task record");
    let task = match background_task_repo::create(db, active).await {
        Ok(task) => task,
        Err(error) => {
            if let Some(dedupe_key) = dedupe_key
                && let Some(existing) =
                    background_task_repo::find_by_dedupe_key(db, &dedupe_key).await?
            {
                tracing::debug!(
                    task_id = existing.id,
                    dedupe_key,
                    "returning existing background task after dedupe collision"
                );
                return Ok(existing);
            }
            return Err(error);
        }
    };
    tracing::debug!(
        task_id = task.id,
        kind = ?task.kind,
        status = ?task.status,
        "inserted typed background task record"
    );
    Ok(task)
}

pub(in crate::services::task_service) async fn create_typed_task_record<S: BackgroundTaskSpec>(
    state: &impl TaskRuntimeState,
    display_name: &str,
    payload: &S::Payload,
    creator_user_id: Option<i64>,
) -> Result<background_task::Model> {
    tracing::debug!(
        display_name_len = display_name.len(),
        has_creator = creator_user_id.is_some(),
        "creating typed background task record"
    );
    let task = insert_typed_task_record(
        state,
        state.writer_db(),
        TypedTaskCreate::<S>::new(display_name, payload.clone()).creator_user_id(creator_user_id),
    )
    .await?;
    state.wake_background_task_dispatcher();
    tracing::debug!(
        task_id = task.id,
        kind = ?task.kind,
        "created typed background task record and woke dispatcher"
    );
    Ok(task)
}
