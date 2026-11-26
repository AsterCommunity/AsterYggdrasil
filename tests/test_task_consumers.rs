//! Queue consumer behavior against the real claim transaction: parallel
//! instances claim disjoint task sets, the cluster quota caps concurrent
//! claims, and unstarted claims release back to retry under the original
//! token fence.

use aster_yggdrasil::services::ops::task::FallbackTaskConsumer;
use aster_yggdrasil_model::entities::background_task;
use aster_yggdrasil_model::types::task::{
    BackgroundTaskKind, BackgroundTaskStatus, StoredTaskPayload,
};
use common::setup;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
mod common;

async fn insert_due_task(state: &aster_yggdrasil::runtime::AppState, name: &str) -> i64 {
    let now = chrono::Utc::now();
    background_task::ActiveModel {
        kind: Set(BackgroundTaskKind::SystemRuntime),
        status: Set(BackgroundTaskStatus::Pending),
        creator_user_id: Set(None),
        display_name: Set(name.to_string()),
        payload_json: Set(StoredTaskPayload(
            serde_json::json!({ "task_name": "task-cleanup" }).to_string(),
        )),
        result_json: Set(None),
        runtime_json: Set(None),
        steps_json: Set(None),
        progress_current: Set(0),
        progress_total: Set(1),
        status_text: Set(None),
        attempt_count: Set(0),
        max_attempts: Set(3),
        next_run_at: Set(now - chrono::Duration::seconds(30)),
        processing_token: Set(0),
        processing_started_at: Set(None),
        last_heartbeat_at: Set(None),
        lease_expires_at: Set(None),
        started_at: Set(None),
        finished_at: Set(None),
        last_error: Set(None),
        failure_can_retry: Set(None),
        expires_at: Set(now + chrono::Duration::hours(24)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(state.writer_db())
    .await
    .expect("due task should insert")
    .id
}

fn consumer(state: &aster_yggdrasil::runtime::AppState) -> FallbackTaskConsumer {
    FallbackTaskConsumer::new(actix_web::web::Data::new(state.clone()))
}

async fn claim(
    consumer: &FallbackTaskConsumer,
    available: usize,
) -> Vec<(background_task::Model, aster_forge_tasks::TaskLease)> {
    use aster_forge_tasks::TaskConsumer;
    TaskConsumer::claim(consumer, available)
        .await
        .expect("consumer claim should succeed")
}

#[actix_web::test]
async fn parallel_consumer_instances_claim_disjoint_task_sets() {
    let state = setup().await;
    // Raise the cluster quota above the task count so the quota cap (covered
    // by the dedicated test below) does not mask the exclusivity assertion.
    set_quota(&state, 6).await;
    let first = consumer(&state);
    let second = consumer(&state);
    for name in ["t1", "t2", "t3", "t4", "t5", "t6"] {
        insert_due_task(&state, name).await;
    }

    // Two worker instances poll at the same time against the same store.
    let (a, b) = tokio::join!(claim(&first, 8), claim(&second, 8));
    let a_ids: Vec<i64> = a.iter().map(|(task, _)| task.id).collect();
    let b_ids: Vec<i64> = b.iter().map(|(task, _)| task.id).collect();

    let mut union = a_ids.iter().chain(b_ids.iter()).collect::<Vec<_>>();
    union.sort_unstable();
    union.dedup();
    assert_eq!(union.len(), 6, "every due task must be claimable");
    let mut overlap = a_ids.clone();
    overlap.retain(|id| b_ids.contains(id));
    assert!(
        overlap.is_empty(),
        "the processing_token CAS must keep claims exclusive, overlapped {overlap:?}"
    );
}

#[actix_web::test]
async fn cluster_quota_caps_concurrent_claims_across_instances() {
    let state = setup().await;
    set_quota(&state, 2).await;
    let first = consumer(&state);
    let second = consumer(&state);
    for name in ["q1", "q2", "q3", "q4"] {
        insert_due_task(&state, name).await;
    }

    let held = claim(&first, 8).await;
    assert_eq!(held.len(), 2, "a single claim respects the lane limit");
    let drained = claim(&second, 8).await;
    assert!(
        drained.is_empty(),
        "processing tasks hold the whole cluster quota; second instance gets nothing"
    );
}

#[actix_web::test]
async fn unstarted_claim_releases_back_to_retry_under_original_token() {
    let state = setup().await;
    let instance = consumer(&state);
    let task_id = insert_due_task(&state, "release-me").await;

    let claimed = claim(&instance, 8).await;
    assert_eq!(claimed.len(), 1);
    let (task, lease) = &claimed[0];
    assert_eq!(task.id, task_id);

    use aster_forge_tasks::TaskConsumer;
    TaskConsumer::release_unstarted(&instance, (task.clone(), *lease))
        .await
        .expect("release should succeed");

    let released = background_task::Entity::find_by_id(task_id)
        .one(state.writer_db())
        .await
        .expect("released task should query")
        .expect("released task should exist");
    assert_eq!(released.status, BackgroundTaskStatus::Retry);
    assert_eq!(released.processing_token, lease.processing_token);
}

async fn set_quota(state: &aster_yggdrasil::runtime::AppState, limit: usize) {
    aster_yggdrasil::db::repository::system_config_repo::upsert(
        state.writer_db(),
        aster_yggdrasil::config::definitions::BACKGROUND_TASK_MAX_CONCURRENCY_KEY,
        &limit.to_string(),
        0,
    )
    .await
    .expect("quota config should upsert");
    state
        .runtime_config()
        .reload(state.writer_db())
        .await
        .expect("quota reload should succeed");
}

#[actix_web::test]
async fn stale_processing_lease_is_taken_over_by_another_instance() {
    let state = setup().await;
    let instance = consumer(&state);
    // A crashed worker left this task Processing with an expired lease.
    let now = chrono::Utc::now();
    let task_id = insert_due_task(&state, "orphaned").await;
    let mut update: background_task::ActiveModel = background_task::Entity::find_by_id(task_id)
        .one(state.writer_db())
        .await
        .expect("task should query")
        .expect("task should exist")
        .into();
    update.status = Set(BackgroundTaskStatus::Processing);
    update.processing_token = Set(77);
    update.processing_started_at = Set(Some(now - chrono::Duration::seconds(120)));
    update.last_heartbeat_at = Set(Some(now - chrono::Duration::seconds(120)));
    update.lease_expires_at = Set(Some(now - chrono::Duration::seconds(60)));
    update
        .update(state.writer_db())
        .await
        .expect("stale processing update should apply");

    let claimed = claim(&instance, 8).await;
    assert_eq!(claimed.len(), 1, "expired lease must be claimable again");
    let (task, lease) = &claimed[0];
    assert_eq!(task.id, task_id);
    assert_ne!(
        lease.processing_token, 77,
        "takeover must fence with a fresh processing token"
    );
}

#[actix_web::test]
async fn not_yet_due_and_backed_off_tasks_are_not_claimed() {
    let state = setup().await;
    let instance = consumer(&state);
    let now = chrono::Utc::now();

    // Scheduled for the future.
    let future_id = insert_due_task(&state, "future").await;
    // Backed off after a failure.
    let retry_id = insert_due_task(&state, "backoff").await;
    for (id, status, next_run_at) in [
        (
            future_id,
            BackgroundTaskStatus::Pending,
            now + chrono::Duration::seconds(300),
        ),
        (
            retry_id,
            BackgroundTaskStatus::Retry,
            now + chrono::Duration::seconds(300),
        ),
    ] {
        let mut update: background_task::ActiveModel = background_task::Entity::find_by_id(id)
            .one(state.writer_db())
            .await
            .expect("task should query")
            .expect("task should exist")
            .into();
        update.status = Set(status);
        update.next_run_at = Set(next_run_at);
        update
            .update(state.writer_db())
            .await
            .expect("schedule update should apply");
    }

    let claimed = claim(&instance, 8).await;
    assert!(
        claimed.is_empty(),
        "tasks must stay queued until their next_run_at is due"
    );
}

#[actix_web::test]
async fn claim_with_zero_available_returns_empty() {
    let state = setup().await;
    let instance = consumer(&state);
    insert_due_task(&state, "unreachable").await;
    assert!(
        claim(&instance, 0).await.is_empty(),
        "zero available slots must never claim"
    );
}

#[actix_web::test]
async fn execute_failure_lands_terminal_state_without_a_fresh_claim() {
    let state = setup().await;
    let instance = consumer(&state);
    // SystemRuntime records are queue-managed bookkeeping payloads; their
    // process step refuses execution, so the consumer must park the task in
    // a terminal failed state instead of retrying it forever.
    let task_id = insert_due_task(&state, "refused").await;
    let claimed = claim(&instance, 8).await;
    assert_eq!(claimed.len(), 1);

    use aster_forge_tasks::TaskConsumer;
    let (task, lease) = &claimed[0];
    let outcome = TaskConsumer::execute(
        &instance,
        (task.clone(), *lease),
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("execution errors surface as outcomes, not driver errors");

    let failed = background_task::Entity::find_by_id(task_id)
        .one(state.writer_db())
        .await
        .expect("failed task should query")
        .expect("failed task should exist");
    assert_eq!(failed.status, BackgroundTaskStatus::Failed);
    assert!(
        failed
            .last_error
            .as_deref()
            .is_some_and(|error| !error.is_empty()),
        "terminal failure must record the error"
    );
    assert_eq!(
        outcome.failed, 1,
        "the refusal must count as a failed outcome"
    );
}
