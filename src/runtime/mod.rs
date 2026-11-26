//! Runtime state and lifecycle modules.

pub mod assembly;
pub(crate) mod audit_recovery;
pub mod bootstrap;
pub mod entrypoint;
pub mod metrics;
pub mod role;
pub mod shutdown;
pub mod startup;
pub mod state;
pub mod state_traits;
pub mod tasks;

#[cfg(test)]
pub(crate) mod test_support;

pub use state::{AppState, AppStateParts};
pub use state_traits::{
    AppConfigRuntimeState, AuditRuntimeState, CacheRuntimeState, ConfigSyncRuntimeState,
    DatabaseRuntimeState, MailRuntimeState, MetricsRuntimeState, RuntimeConfigRuntimeState,
    SharedRuntimeState, TaskRuntimeState, TextureStorageRuntimeState,
    YggdrasilSessionForwardRuntimeState,
};
