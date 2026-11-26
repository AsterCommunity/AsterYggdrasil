//! Capability traits exposed by the shared runtime state.

use std::sync::Arc;

use crate::config::{Config, RuntimeConfig};
use crate::texture_storage::TextureStore;
use aster_forge_cache::CacheBackend;
use aster_forge_config::ConfigSyncRuntime;
use aster_forge_mail::MailSender;
use aster_forge_metrics::SharedMetricsRecorder;
use sea_orm::DatabaseConnection;

use super::state::AppState;

/// Exposes the database connections carried by runtime state. Implemented by
/// [`AppState`]; services and repositories accept this trait instead of the
/// concrete state so tests can supply minimal stand-ins.
pub trait DatabaseRuntimeState {
    /// Primary database connection used for writes.
    fn writer_db(&self) -> &DatabaseConnection;
    /// Read-preferred connection; falls back to the writer when no replica
    /// is configured.
    fn reader_db(&self) -> &DatabaseConnection;
}

/// Exposes the static, file/CLI-loaded configuration.
pub trait AppConfigRuntimeState {
    /// Immutable process-lifetime configuration.
    fn config(&self) -> &Arc<Config>;
}

/// Exposes the database-backed runtime configuration.
pub trait RuntimeConfigRuntimeState {
    /// Hot-reloaded runtime configuration shared across processes.
    fn runtime_config(&self) -> &Arc<RuntimeConfig>;
}

/// Exposes the cross-process runtime configuration change propagation runtime.
pub trait ConfigSyncRuntimeState {
    /// Config sync runtime this state participates in.
    fn config_sync(&self) -> &ConfigSyncRuntime;
}

/// Exposes the selected cache backend.
pub trait CacheRuntimeState {
    /// Cache backend (memory, noop, or Redis per static config).
    fn cache(&self) -> &Arc<dyn CacheBackend>;
}

/// Exposes the object storage handle for Minecraft texture files.
pub trait TextureStorageRuntimeState {
    /// Texture object storage backend selected at startup.
    fn texture_storage(&self) -> &Arc<dyn TextureStore>;
}

/// Exposes the shared metrics recorder.
pub trait MetricsRuntimeState {
    /// Recorder every instrumented component reports through.
    fn metrics(&self) -> &SharedMetricsRecorder;
}

/// Exposes the buffered asynchronous audit log writer.
pub trait AuditRuntimeState {
    /// Audit writer all audit admissions go through.
    fn audit_writer(&self) -> &Arc<aster_forge_audit::AuditWriter>;
}

/// Capability for driving Yggdrasil session-forward upstream calls.
/// Requires the full shared state because forwarding depends on several
/// components at once.
pub trait YggdrasilSessionForwardRuntimeState: SharedRuntimeState {
    /// Outbound HTTP client dedicated to session forwarding.
    fn yggdrasil_session_forward_http_client(&self) -> &aster_forge_http::HttpClient;
}

/// Composite capability bound covering the core runtime components. Any type
/// implementing all seven member traits implements this automatically, so
/// services can demand "the standard runtime state" with one bound.
pub trait SharedRuntimeState:
    DatabaseRuntimeState
    + AppConfigRuntimeState
    + RuntimeConfigRuntimeState
    + CacheRuntimeState
    + TextureStorageRuntimeState
    + MetricsRuntimeState
    + AuditRuntimeState
{
}

impl<T> SharedRuntimeState for T where
    T: DatabaseRuntimeState
        + AppConfigRuntimeState
        + RuntimeConfigRuntimeState
        + CacheRuntimeState
        + TextureStorageRuntimeState
        + MetricsRuntimeState
        + AuditRuntimeState
{
}

/// Capability for sending mail through the runtime-configured sender.
pub trait MailRuntimeState: SharedRuntimeState {
    /// Outbound mail client built from runtime SMTP configuration.
    fn mail_sender(&self) -> &Arc<dyn MailSender>;
}

/// Capability for participating in background task dispatch.
pub trait TaskRuntimeState: SharedRuntimeState {
    /// Wakes the dispatcher loop so freshly enqueued tasks run without
    /// waiting for the next polling tick.
    fn wake_background_task_dispatcher(&self);
}

/// Exposes the process-wide permission registry that validates permission
/// identifiers at authorization boundaries.
pub trait PermissionRegistryRuntimeState {
    /// Registry of valid permission descriptors (built-ins plus any
    /// process-injected extras).
    fn permission_registry(&self) -> &aster_yggdrasil_model::types::permission::PermissionRegistry;
}

impl DatabaseRuntimeState for AppState {
    fn writer_db(&self) -> &DatabaseConnection {
        self.writer_db()
    }

    fn reader_db(&self) -> &DatabaseConnection {
        self.reader_db()
    }
}

impl AppConfigRuntimeState for AppState {
    fn config(&self) -> &Arc<Config> {
        self.config()
    }
}

impl RuntimeConfigRuntimeState for AppState {
    fn runtime_config(&self) -> &Arc<RuntimeConfig> {
        self.runtime_config()
    }
}

impl ConfigSyncRuntimeState for AppState {
    fn config_sync(&self) -> &ConfigSyncRuntime {
        self.config_sync()
    }
}

impl CacheRuntimeState for AppState {
    fn cache(&self) -> &Arc<dyn CacheBackend> {
        self.cache()
    }
}

impl TextureStorageRuntimeState for AppState {
    fn texture_storage(&self) -> &Arc<dyn TextureStore> {
        self.texture_storage()
    }
}

impl MetricsRuntimeState for AppState {
    fn metrics(&self) -> &SharedMetricsRecorder {
        self.metrics()
    }
}

impl AuditRuntimeState for AppState {
    fn audit_writer(&self) -> &Arc<aster_forge_audit::AuditWriter> {
        self.audit_writer()
    }
}

impl YggdrasilSessionForwardRuntimeState for AppState {
    fn yggdrasil_session_forward_http_client(&self) -> &aster_forge_http::HttpClient {
        self.yggdrasil_session_forward_http_client()
    }
}

impl MailRuntimeState for AppState {
    fn mail_sender(&self) -> &Arc<dyn MailSender> {
        self.mail_sender()
    }
}

impl TaskRuntimeState for AppState {
    fn wake_background_task_dispatcher(&self) {
        self.background_task_dispatch_wakeup.notify();
    }
}

impl PermissionRegistryRuntimeState for AppState {
    fn permission_registry(&self) -> &aster_yggdrasil_model::types::permission::PermissionRegistry {
        self.permission_registry()
    }
}
