//! Process responsibilities for the single AsterYggdrasil executable.
//!
//! A process serving the `api` role answers product HTTP traffic and must not
//! consume background tasks; a `worker` process consumes background tasks and
//! only exposes health probes. `all` keeps the historical single-process
//! behavior. Deployment topology (how many processes of each role run) is
//! chosen by the operator, independent of any configuration field.

/// Roles a process can take, mirroring the AsterDrive serve matrix.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum ServeRole {
    /// Serve HTTP traffic and consume background tasks in one process.
    #[default]
    All,
    /// Serve HTTP traffic only; background tasks stay queued for workers.
    Api,
    /// Consume background tasks only; HTTP serves health probes.
    Worker,
}

impl ServeRole {
    /// Canonical lowercase name of the role as used in logs and metrics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Api => "api",
            Self::Worker => "worker",
        }
    }

    /// Whether a process with this role answers product HTTP traffic.
    pub const fn serves_api(self) -> bool {
        !matches!(self, Self::Worker)
    }

    /// Whether a process with this role claims and executes background tasks.
    pub const fn consumes_tasks(self) -> bool {
        !matches!(self, Self::Api)
    }
}

/// Liveness and readiness state of the process role, consumed by health probes.
///
/// `initialized` flips once every component startup phase has completed, so a
/// probe hitting the process during startup reports not-ready instead of
/// success. Task consumer statuses track the leased scheduled runtime group;
/// an empty list is only valid for roles that do not consume tasks.
#[derive(Clone)]
pub struct RoleHealth {
    /// Responsibility this process carries; decides which probe conditions apply.
    pub role: ServeRole,
    /// One status per scheduled runtime task consumer group this process runs.
    /// Empty lists are only valid for roles that do not consume tasks.
    pub consumers: Vec<aster_forge_tasks::TaskRuntimeStatus>,
    /// Cancellation token fired when graceful shutdown begins; draining
    /// processes report not-ready but stay live while it is cancelled.
    pub shutdown: tokio_util::sync::CancellationToken,
    /// Flips to true once every component startup phase has completed, so
    /// probes during startup report not-ready instead of success.
    pub initialized: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl RoleHealth {
    /// Ready means fully initialized, every task consumer claiming normally,
    /// and shutdown not yet requested.
    pub fn ready(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::Acquire)
            && (!self.role.consumes_tasks() || !self.consumers.is_empty())
            && !self.shutdown.is_cancelled()
            && self
                .consumers
                .iter()
                .all(aster_forge_tasks::TaskRuntimeStatus::ready)
    }

    /// Live means the process is either healthy or already shutting down;
    /// a cancelling shutdown must not let orchestrators kill a draining
    /// process early, so cancelled shutdown reports live.
    pub fn live(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::Acquire)
            && (self.shutdown.is_cancelled()
                || ((!self.role.consumes_tasks() || !self.consumers.is_empty())
                    && self
                        .consumers
                        .iter()
                        .all(aster_forge_tasks::TaskRuntimeStatus::is_running)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aster_forge_tasks::TaskRuntimeStatus;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn health(role: ServeRole, consumers: Vec<TaskRuntimeStatus>) -> RoleHealth {
        RoleHealth {
            role,
            consumers,
            shutdown: Default::default(),
            initialized: Arc::new(AtomicBool::new(true)),
        }
    }

    #[test]
    fn responsibility_matrix_holds_for_every_role() {
        for role in [ServeRole::All, ServeRole::Api, ServeRole::Worker] {
            assert_eq!(role.serves_api(), role != ServeRole::Worker);
            assert_eq!(role.consumes_tasks(), role != ServeRole::Api);
        }
        assert_eq!(ServeRole::All.as_str(), "all");
        assert_eq!(ServeRole::Api.as_str(), "api");
        assert_eq!(ServeRole::Worker.as_str(), "worker");
    }

    #[test]
    fn default_role_is_all() {
        assert_eq!(ServeRole::default(), ServeRole::All);
        assert!(ServeRole::All.serves_api() && ServeRole::All.consumes_tasks());
    }

    #[test]
    fn uninitialized_process_is_never_ready_or_live() {
        for role in [ServeRole::All, ServeRole::Api, ServeRole::Worker] {
            let mut candidate = health(role, Vec::new());
            candidate.initialized = Arc::new(AtomicBool::new(false));
            assert!(!candidate.ready(), "{role:?} must not be ready pre-startup");
            assert!(!candidate.live(), "{role:?} must not be live pre-startup");
        }
    }

    #[test]
    fn api_role_is_ready_without_consumers() {
        let candidate = health(ServeRole::Api, Vec::new());
        assert!(candidate.ready());
        assert!(candidate.live());
    }

    #[test]
    fn worker_role_requires_at_least_one_consumer() {
        let mut candidate = health(ServeRole::Worker, Vec::new());
        assert!(
            !candidate.ready(),
            "worker without consumers is misconfigured"
        );
        assert!(!candidate.live());
        candidate.consumers.push(TaskRuntimeStatus::default());
        assert!(
            !candidate.ready(),
            "unstarted consumer must block readiness"
        );
        assert!(!candidate.live(), "unstarted consumer must block liveness");
    }

    #[test]
    fn draining_or_claim_failed_consumer_is_not_ready() {
        // A default status has never started, so a consumer that exists but has
        // not begun claiming keeps both probes reporting not-ready.
        let candidate = health(ServeRole::Worker, vec![TaskRuntimeStatus::default()]);
        assert!(!candidate.ready());
        assert!(!candidate.live());
    }

    #[test]
    fn cancelled_shutdown_reports_not_ready_but_still_live() {
        let candidate = health(ServeRole::Worker, Vec::new());
        candidate.shutdown.cancel();
        assert!(!candidate.ready(), "draining process must leave rotation");
        assert!(
            candidate.live(),
            "draining process must keep serving probes"
        );
    }

    #[test]
    fn cancelled_shutdown_overrides_broken_consumers_for_liveness() {
        let candidate = health(ServeRole::Worker, vec![TaskRuntimeStatus::default()]);
        candidate.shutdown.cancel();
        assert!(candidate.live());
        assert!(!candidate.ready());
    }

    #[test]
    fn all_role_requires_consumers_like_worker() {
        let candidate = health(ServeRole::All, Vec::new());
        assert!(!candidate.ready());
        assert!(!candidate.live());
    }
}
