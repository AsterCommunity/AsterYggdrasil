//! Runtime state and lifecycle modules.

pub mod assembly;
pub mod bootstrap;
pub mod entrypoint;
pub mod metrics;
pub mod startup;
pub mod state;
pub mod state_traits;

#[cfg(test)]
pub(crate) mod test_support;

pub use state::{AppState, AppStateParts};
pub use state_traits::{
    AppConfigRuntimeState, AuditRuntimeState, CacheRuntimeState, ConfigSyncRuntimeState,
    DatabaseRuntimeState, MailRuntimeState, MetricsRuntimeState, ObjectStorageRuntimeState,
    RuntimeConfigRuntimeState, SharedRuntimeState, TaskRuntimeState,
    YggdrasilSessionForwardRuntimeState,
};
