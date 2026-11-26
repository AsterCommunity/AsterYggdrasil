//! Shared product runtime state.

use crate::api::middleware::yggdrasil_rate_limit::YggdrasilRateLimiter;
use crate::config::{Config, RuntimeConfig};
use crate::errors::{AsterError, Result};
use crate::texture_storage::TextureStore;
use aster_forge_cache::CacheBackend;
use aster_forge_config::ConfigSyncRuntime;
use aster_forge_db::DbHandles;
use aster_forge_mail::MailSender;
use aster_forge_metrics::SharedMetricsRecorder;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;

const YGGDRASIL_SESSION_FORWARD_USER_AGENT: &str = concat!(
    "AsterYggdrasil/",
    env!("CARGO_PKG_VERSION"),
    " yggdrasil-session-forward"
);

/// Shared per-process runtime state handed to HTTP handlers, services, and
/// background workers. Built once during startup and cloned cheaply via `Arc`
/// members; capability access goes through the [`super::state_traits`] traits.
#[derive(Clone)]
pub struct AppState {
    /// Writer/reader database connections resolved at startup.
    pub db_handles: DbHandles,
    /// Static, file/CLI-loaded configuration; immutable for the process lifetime.
    pub config: Arc<Config>,
    /// Database-backed runtime configuration with cross-process hot reload.
    pub runtime_config: Arc<RuntimeConfig>,
    /// Selected cache backend (memory, noop, or Redis per static config).
    pub cache: Arc<dyn CacheBackend>,
    /// Object storage handle for Minecraft texture files.
    pub texture_storage: Arc<dyn TextureStore>,
    /// Outbound mail client driven by the current runtime SMTP configuration.
    pub mail_sender: Arc<dyn MailSender>,
    /// Cross-process runtime configuration change propagation runtime.
    pub config_sync: ConfigSyncRuntime,
    /// Shared metrics recorder used by every instrumented component.
    pub metrics: SharedMetricsRecorder,
    /// Buffered asynchronous audit log writer; admission is best-effort and
    /// overflow records fall back to direct writes.
    pub audit_writer: Arc<aster_forge_audit::AuditWriter>,
    /// Monotonic instant the process state was assembled, used for uptime.
    pub started_at: Instant,
    /// Rate limiter for Yggdrasil protocol endpoints, seeded from static config.
    pub yggdrasil_rate_limiter: YggdrasilRateLimiter,
    /// Outbound HTTP client dedicated to Yggdrasil session forwarding.
    pub yggdrasil_session_forward_http_client: aster_forge_http::HttpClient,
    /// Notified to wake the background task dispatcher when a task is
    /// enqueued outside the regular polling interval.
    pub background_task_dispatch_wakeup: Arc<Notify>,
}

/// Injectable subset of [`AppState`]: the components built during startup
/// phases that must be supplied rather than derived in-process. Consumed by
/// [`AppState::from_parts`], which constructs everything not listed here.
pub struct AppStateParts {
    /// Writer/reader database connections resolved at startup.
    pub db_handles: DbHandles,
    /// Static, file/CLI-loaded configuration.
    pub config: Arc<Config>,
    /// Database-backed runtime configuration.
    pub runtime_config: Arc<RuntimeConfig>,
    /// Selected cache backend.
    pub cache: Arc<dyn CacheBackend>,
    /// Object storage handle for Minecraft texture files.
    pub texture_storage: Arc<dyn TextureStore>,
    /// Outbound mail client built from runtime SMTP configuration.
    pub mail_sender: Arc<dyn MailSender>,
    /// Cross-process runtime configuration change propagation runtime.
    pub config_sync: ConfigSyncRuntime,
    /// Shared metrics recorder.
    pub metrics: SharedMetricsRecorder,
    /// Buffered asynchronous audit log writer.
    pub audit_writer: Arc<aster_forge_audit::AuditWriter>,
}

impl AppState {
    /// Assembles [`AppState`] from injectable parts, constructing the
    /// process-derived members (uptime instant, rate limiter, session
    /// forward HTTP client, dispatch wakeup) that cannot be injected.
    pub fn from_parts(parts: AppStateParts) -> Result<Self> {
        let yggdrasil_rate_limiter = Self::new_yggdrasil_rate_limiter(&parts.config);
        let yggdrasil_session_forward_http_client =
            Self::new_yggdrasil_session_forward_http_client()?;

        Ok(Self {
            db_handles: parts.db_handles,
            config: parts.config,
            runtime_config: parts.runtime_config,
            cache: parts.cache,
            texture_storage: parts.texture_storage,
            mail_sender: parts.mail_sender,
            config_sync: parts.config_sync,
            metrics: parts.metrics,
            audit_writer: parts.audit_writer,
            started_at: Self::new_started_at(),
            yggdrasil_rate_limiter,
            yggdrasil_session_forward_http_client,
            background_task_dispatch_wakeup: Self::new_background_task_dispatch_wakeup(),
        })
    }

    /// Captures the monotonic instant used as the process start time.
    pub fn new_started_at() -> Instant {
        Instant::now()
    }

    /// Creates a fresh dispatcher wakeup handle; tests use this to build
    /// isolated states without sharing wake signals.
    pub fn new_background_task_dispatch_wakeup() -> Arc<Notify> {
        Arc::new(Notify::new())
    }

    /// Builds the Yggdrasil rate limiter from the static rate-limit config.
    pub fn new_yggdrasil_rate_limiter(config: &Config) -> YggdrasilRateLimiter {
        YggdrasilRateLimiter::from_config(&config.rate_limit)
    }

    /// Builds the outbound HTTP client used for Yggdrasil session forwarding,
    /// pinned to the session-forward outbound profile and user agent.
    pub fn new_yggdrasil_session_forward_http_client() -> Result<aster_forge_http::HttpClient> {
        aster_yggdrasil_outbound::ClientConfig::new(
            aster_yggdrasil_outbound::Profile::SessionForward,
        )
        .user_agent(YGGDRASIL_SESSION_FORWARD_USER_AGENT)
        .build()
        .map_err(|error| {
            AsterError::internal_error(format!(
                "build Yggdrasil session forward HTTP client: {error}"
            ))
        })
    }

    /// Primary database connection used for writes.
    pub fn writer_db(&self) -> &DatabaseConnection {
        self.db_handles.writer()
    }

    /// Read-preferred database connection; falls back to the writer when no
    /// replica is configured.
    pub fn reader_db(&self) -> &DatabaseConnection {
        self.db_handles.reader()
    }

    /// Static, file/CLI-loaded configuration.
    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    /// Database-backed runtime configuration with hot reload.
    pub fn runtime_config(&self) -> &Arc<RuntimeConfig> {
        &self.runtime_config
    }

    /// Selected cache backend.
    pub fn cache(&self) -> &Arc<dyn CacheBackend> {
        &self.cache
    }

    /// Object storage handle for Minecraft texture files.
    pub fn texture_storage(&self) -> &Arc<dyn TextureStore> {
        &self.texture_storage
    }

    /// Outbound mail client built from runtime SMTP configuration.
    pub fn mail_sender(&self) -> &Arc<dyn MailSender> {
        &self.mail_sender
    }

    /// Cross-process runtime configuration change propagation runtime.
    pub fn config_sync(&self) -> &ConfigSyncRuntime {
        &self.config_sync
    }

    /// Shared metrics recorder.
    pub fn metrics(&self) -> &SharedMetricsRecorder {
        &self.metrics
    }

    /// Buffered asynchronous audit log writer.
    pub fn audit_writer(&self) -> &Arc<aster_forge_audit::AuditWriter> {
        &self.audit_writer
    }

    /// Time elapsed since the process state was assembled.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Rate limiter shared by Yggdrasil protocol endpoints.
    pub fn yggdrasil_rate_limiter(&self) -> &YggdrasilRateLimiter {
        &self.yggdrasil_rate_limiter
    }

    /// Outbound HTTP client dedicated to Yggdrasil session forwarding.
    pub fn yggdrasil_session_forward_http_client(&self) -> &aster_forge_http::HttpClient {
        &self.yggdrasil_session_forward_http_client
    }

    /// Wake handle that pokes the background task dispatcher loop.
    pub fn background_task_dispatch_wakeup(&self) -> &Arc<Notify> {
        &self.background_task_dispatch_wakeup
    }
}
