//! Runtime state and lifecycle modules.

pub mod assembly;
pub mod bootstrap;
pub mod entrypoint;
pub mod http;
pub mod metrics;
pub mod startup;
pub mod state;
pub mod state_traits;

pub use state::{AppState, AppStateParts};
pub use state_traits::{
    AppConfigRuntimeState, CacheRuntimeState, DatabaseRuntimeState, MailRuntimeState,
    MetricsRuntimeState, ObjectStorageRuntimeState, RuntimeConfigRuntimeState, SharedRuntimeState,
    TaskRuntimeState, YggdrasilSessionForwardRuntimeState,
};
