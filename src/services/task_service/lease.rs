//! Background task processing lease bindings.
//!
//! Forge owns the in-memory lease guard and execution context. Yggdrasil keeps only the narrow
//! error-classification helpers needed after those `TaskCoreError` values are mapped into
//! `AsterError`.

use chrono::{DateTime, Utc};

use crate::errors::AsterError;
use aster_forge_tasks::TaskLease;

use super::{TASK_HEARTBEAT_INTERVAL_SECS, TASK_PROCESSING_STALE_SECS};

const TASK_LEASE_LOST_MESSAGE_PREFIX: &str = "background task lease lost";
const TASK_LEASE_RENEWAL_TIMEOUT_MESSAGE_PREFIX: &str = "background task lease renewal timed out";
const TASK_WORKER_SHUTDOWN_REQUESTED_MESSAGE_PREFIX: &str =
    "background task worker shutdown requested";

pub(super) fn task_lease_lost(lease: TaskLease) -> AsterError {
    AsterError::from(aster_forge_tasks::TaskCoreError::LeaseLost {
        task_id: lease.task_id,
        processing_token: lease.processing_token,
    })
}

pub(super) fn is_task_lease_lost(error: &AsterError) -> bool {
    error.message().starts_with(TASK_LEASE_LOST_MESSAGE_PREFIX)
}

pub(super) fn is_task_lease_renewal_timed_out(error: &AsterError) -> bool {
    error
        .message()
        .starts_with(TASK_LEASE_RENEWAL_TIMEOUT_MESSAGE_PREFIX)
}

pub(super) fn is_task_worker_shutdown_requested(error: &AsterError) -> bool {
    error
        .message()
        .starts_with(TASK_WORKER_SHUTDOWN_REQUESTED_MESSAGE_PREFIX)
}

pub(super) fn task_lease_renewal_timeout() -> std::time::Duration {
    aster_forge_tasks::task_lease_renewal_timeout(
        TASK_PROCESSING_STALE_SECS,
        TASK_HEARTBEAT_INTERVAL_SECS,
    )
}

pub(super) fn task_lease_expires_at(now: DateTime<Utc>) -> DateTime<Utc> {
    aster_forge_tasks::task_lease_expires_at(now, TASK_PROCESSING_STALE_SECS)
}
