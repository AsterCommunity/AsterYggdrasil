//! Background task processing lease bindings.
//!
//! Forge owns the in-memory lease guard implementation. Yggdrasil keeps this narrow binding layer
//! because task specs and service helpers use `AsterError`, while Forge reports lease-control
//! failures as `TaskCoreError`.

use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;

use crate::errors::{AsterError, Result};

use super::{TASK_HEARTBEAT_INTERVAL_SECS, TASK_PROCESSING_STALE_SECS};

pub(in crate::services::task_service) use aster_forge_tasks::TaskLease;

const TASK_LEASE_LOST_MESSAGE_PREFIX: &str = "background task lease lost";
const TASK_LEASE_RENEWAL_TIMEOUT_MESSAGE_PREFIX: &str = "background task lease renewal timed out";
const TASK_WORKER_SHUTDOWN_REQUESTED_MESSAGE_PREFIX: &str =
    "background task worker shutdown requested";

#[derive(Debug, Clone)]
pub(in crate::services::task_service) struct TaskLeaseGuard {
    inner: aster_forge_tasks::TaskLeaseGuard,
}

impl TaskLeaseGuard {
    pub(super) fn new(lease: TaskLease) -> Self {
        Self::with_renewal_timeout(lease, task_lease_renewal_timeout())
    }

    pub(super) fn with_renewal_timeout(lease: TaskLease, renewal_timeout: StdDuration) -> Self {
        Self {
            inner: aster_forge_tasks::TaskLeaseGuard::new(lease, renewal_timeout),
        }
    }

    fn with_shutdown_token(lease: TaskLease, shutdown_token: CancellationToken) -> Self {
        Self {
            inner: aster_forge_tasks::TaskLeaseGuard::with_shutdown_token(
                lease,
                task_lease_renewal_timeout(),
                shutdown_token,
            ),
        }
    }

    pub(super) fn lease(&self) -> TaskLease {
        self.inner.lease()
    }

    pub(super) fn record_renewed(&self) {
        self.inner.record_renewed();
    }

    pub(super) fn mark_lost(&self) -> AsterError {
        AsterError::from(self.inner.mark_lost())
    }

    pub(super) fn ensure_active(&self) -> Result<()> {
        self.inner.ensure_active().map_err(AsterError::from)
    }

    pub(super) fn as_forge(&self) -> aster_forge_tasks::TaskLeaseGuard {
        self.inner.clone()
    }
}

#[derive(Debug, Clone)]
pub(in crate::services::task_service) struct TaskExecutionContext {
    lease_guard: TaskLeaseGuard,
    inner: aster_forge_tasks::TaskExecutionContext,
}

impl TaskExecutionContext {
    pub(super) fn new(lease: TaskLease, shutdown_token: CancellationToken) -> Self {
        Self {
            lease_guard: TaskLeaseGuard::with_shutdown_token(lease, shutdown_token.clone()),
            inner: aster_forge_tasks::TaskExecutionContext::new(
                lease,
                task_lease_renewal_timeout(),
                shutdown_token,
            ),
        }
    }

    pub(super) fn lease_guard(&self) -> &TaskLeaseGuard {
        &self.lease_guard
    }

    pub(super) fn ensure_active(&self) -> Result<()> {
        self.inner.ensure_active().map_err(AsterError::from)
    }

    pub(super) async fn sleep_or_shutdown(&self, duration: StdDuration) -> Result<()> {
        self.inner
            .sleep_or_shutdown(duration)
            .await
            .map_err(AsterError::from)
    }

    pub(super) async fn shutdown_requested(&self) -> Result<()> {
        self.inner
            .shutdown_requested()
            .await
            .map_err(AsterError::from)
    }
}

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

pub(super) fn task_lease_renewal_timeout() -> StdDuration {
    aster_forge_tasks::task_lease_renewal_timeout(
        TASK_PROCESSING_STALE_SECS,
        TASK_HEARTBEAT_INTERVAL_SECS,
    )
}

pub(super) fn task_lease_expires_at(now: DateTime<Utc>) -> DateTime<Utc> {
    aster_forge_tasks::task_lease_expires_at(now, TASK_PROCESSING_STALE_SECS)
}
