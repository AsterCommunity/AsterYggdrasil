//! Standby takeover of the leased scheduled maintenance group: when the
//! lease-holding instance dies, a standby instance must claim the lease after
//! expiry and keep executing scheduled maintenance tasks. Queue consumers are
//! unaffected by design (they never touch the runtime lease); the queued-task
//! claim at the end pins that boundary.

use std::time::Duration;

use actix_web::web;
use aster_forge_db::{RuntimeLeaseDbStore, ScheduledTaskDbStore};
use aster_forge_tasks::TaskRuntimeStatus;
use aster_yggdrasil::runtime::AppState;
use aster_yggdrasil::services::ops::task::{RuntimeTaskRunOutcome, SystemRuntimeTaskKind};
use chrono::Utc;
use common::setup;
use sea_orm::ConnectionTrait;
use tokio_util::sync::CancellationToken;

mod common;

const LEASE_ID: &str = "aster_yggdrasil.background_tasks";
const LEASE_TTL: Duration = Duration::from_secs(2);
const RENEW_INTERVAL: Duration = Duration::from_millis(500);
const STANDBY_RETRY: Duration = Duration::from_millis(250);

fn spawn_group(
    state: web::Data<AppState>,
    status: TaskRuntimeStatus,
    stop: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let config = aster_forge_tasks::LeasedScheduledRuntimeConfig::new(
        "aster_yggdrasil",
        LEASE_ID,
        RuntimeLeaseDbStore::new(state.writer_db().clone()),
        ScheduledTaskDbStore::new(state.writer_db().clone()),
        state.clone(),
        |panic_message| {
            RuntimeTaskRunOutcome::failed(Some("Task panicked".to_string()), panic_message)
        },
        record_scheduled_outcome,
    )
    .status(status)
    .claim_ttl(Duration::from_secs(2))
    .lease_ttl(LEASE_TTL)
    .lease_renew_interval(RENEW_INTERVAL)
    .lease_standby_retry_interval(STANDBY_RETRY);

    tokio::spawn(async move {
        config
            .run(stop, |runtime| {
                runtime.scheduled(
                    SystemRuntimeTaskKind::TaskCleanup,
                    |_state| Duration::from_millis(400),
                    None,
                    run_task_cleanup_body,
                );
            })
            .await;
    })
}

async fn record_scheduled_outcome(
    state: web::Data<AppState>,
    name: SystemRuntimeTaskKind,
    claim: aster_forge_tasks::ScheduledTaskClaim,
    started_at: chrono::DateTime<Utc>,
    finished_at: chrono::DateTime<Utc>,
    outcome: RuntimeTaskRunOutcome,
) {
    if let Err(error) = aster_yggdrasil::services::ops::task::record_scheduled_runtime_task_run(
        state.get_ref(),
        name,
        claim.scheduled_at,
        started_at,
        finished_at,
        &outcome,
    )
    .await
    {
        tracing::warn!("failed to record scheduled task run: {error}");
    }
}

async fn run_task_cleanup_body(state: web::Data<AppState>) -> RuntimeTaskRunOutcome {
    match aster_yggdrasil::services::ops::task::cleanup_expired(state.get_ref()).await {
        Ok(cleaned) if cleaned > 0 => {
            RuntimeTaskRunOutcome::succeeded(Some(format!("cleaned {cleaned} artifacts")))
        }
        Ok(_) => RuntimeTaskRunOutcome::quiet(),
        Err(error) => RuntimeTaskRunOutcome::failed(
            Some("Task cleanup failed".to_string()),
            error.to_string(),
        ),
    }
}

async fn lease_owner(state: &AppState) -> Option<String> {
    // Forge keeps the lease model private; a raw query is the honest way to
    // observe the current owner from the shared table.
    let row = state
        .writer_db()
        .query_one_raw(sea_orm::Statement::from_sql_and_values(
            state.writer_db().get_database_backend(),
            "SELECT owner_id FROM runtime_leases WHERE lease_id = $1",
            [LEASE_ID.into()],
        ))
        .await
        .expect("runtime lease query should succeed");
    row.map(|row| {
        row.try_get::<String>("", "owner_id")
            .expect("owner_id should decode as a string")
    })
}

/// Reads the scheduled_tasks catalog row for the cleanup task; `last_finished_at`
/// advancing is the authoritative signal that the current lease holder keeps
/// executing it. Quiet runs do not create task-history records, so the
/// catalog — not the background_task table — is the observable here.
async fn cleanup_task_last_finished_at(state: &AppState) -> Option<chrono::DateTime<Utc>> {
    let row = state
        .writer_db()
        .query_one_raw(sea_orm::Statement::from_sql_and_values(
            state.writer_db().get_database_backend(),
            "SELECT last_finished_at FROM scheduled_tasks WHERE namespace = $1 AND task_name = $2",
            [
                "aster_yggdrasil".into(),
                SystemRuntimeTaskKind::TaskCleanup.as_str().into(),
            ],
        ))
        .await
        .expect("scheduled task catalog query should succeed");
    row.and_then(|row| {
        row.try_get::<Option<chrono::DateTime<Utc>>>("", "last_finished_at")
            .expect("last_finished_at should decode as a timestamp")
    })
}

/// Polls an async condition until it yields a value or the timeout fires.
macro_rules! wait_for {
    ($what:expr, $poll:expr) => {
        match tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if let Some(value) = $poll {
                    break value;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(value) => value,
            Err(_) => panic!("timed out waiting for {}", $what),
        }
    };
}

#[actix_web::test]
async fn standby_takes_over_scheduled_lease_after_holder_failure() {
    let state = setup().await;
    let state_data = web::Data::new(state.clone());

    let holder_stop = CancellationToken::new();
    let standby_stop = CancellationToken::new();
    let holder_status = TaskRuntimeStatus::default();
    let standby_status = TaskRuntimeStatus::default();

    let holder = spawn_group(state_data.clone(), holder_status, holder_stop);
    let standby = spawn_group(state_data.clone(), standby_status, standby_stop.clone());

    // Exactly one of the two instances holds the lease.
    let first_owner = wait_for!("the first instance to acquire the runtime lease", {
        lease_owner(&state).await
    });

    // The holder crashes: its runner future is aborted, so it can neither
    // renew nor release — exactly what a kill -9 leaves behind.
    holder.abort();
    let _ = holder.await;

    let second_owner = wait_for!("the standby to take over the expired lease", {
        match lease_owner(&state).await {
            Some(owner) if owner != first_owner => Some(owner),
            _ => None,
        }
    });
    assert_ne!(second_owner, first_owner, "a new owner must hold the lease");

    // The new owner keeps renewing: the same owner still holds the lease well
    // past one TTL window after takeover.
    tokio::time::sleep(LEASE_TTL * 2).await;
    let renewed = lease_owner(&state)
        .await
        .expect("lease row must persist after takeover");
    assert_eq!(
        renewed, second_owner,
        "the standby must keep holding the lease"
    );

    // Scheduled work keeps flowing under the new owner: the catalog's
    // last_finished_at must start advancing, proving the standby executes the
    // maintenance task it took over.
    let first_finished = wait_for!("scheduled cleanup executed after takeover", {
        cleanup_task_last_finished_at(&state).await
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    wait_for!("scheduled cleanup keeps running under the new owner", {
        let next = cleanup_task_last_finished_at(&state).await;
        next.is_some_and(|at| at > first_finished).then_some(())
    });

    // Queue consumers never touch the runtime lease: claiming still works
    // while the scheduled group runs.
    let queue_consumer = aster_yggdrasil::services::ops::task::FallbackTaskConsumer::new(
        web::Data::new(state.clone()),
    );
    let claimed = aster_forge_tasks::TaskConsumer::claim(&queue_consumer, 4).await;
    assert!(
        claimed.is_ok(),
        "queue consumers must claim independently of the runtime lease"
    );

    standby_stop.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(10), standby).await;
}
