//! Shared product runtime state.

use crate::api::middleware::yggdrasil_rate_limit::YggdrasilRateLimiter;
use crate::config::{Config, RuntimeConfig};
use crate::errors::{AsterError, Result};
use crate::object_storage::ObjectStorage;
use aster_forge_cache::CacheBackend;
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

#[derive(Clone)]
pub struct AppState {
    pub db_handles: DbHandles,
    pub config: Arc<Config>,
    pub runtime_config: Arc<RuntimeConfig>,
    pub cache: Arc<dyn CacheBackend>,
    pub object_storage: Arc<dyn ObjectStorage>,
    pub mail_sender: Arc<dyn MailSender>,
    pub metrics: SharedMetricsRecorder,
    pub started_at: Instant,
    pub yggdrasil_rate_limiter: YggdrasilRateLimiter,
    pub yggdrasil_session_forward_http_client: reqwest::Client,
    pub background_task_dispatch_wakeup: Arc<Notify>,
}

pub struct AppStateParts {
    pub db_handles: DbHandles,
    pub config: Arc<Config>,
    pub runtime_config: Arc<RuntimeConfig>,
    pub cache: Arc<dyn CacheBackend>,
    pub object_storage: Arc<dyn ObjectStorage>,
    pub mail_sender: Arc<dyn MailSender>,
    pub metrics: SharedMetricsRecorder,
}

impl AppState {
    pub fn from_parts(parts: AppStateParts) -> Result<Self> {
        let yggdrasil_rate_limiter = Self::new_yggdrasil_rate_limiter(&parts.config);
        let yggdrasil_session_forward_http_client =
            Self::new_yggdrasil_session_forward_http_client()?;

        Ok(Self {
            db_handles: parts.db_handles,
            config: parts.config,
            runtime_config: parts.runtime_config,
            cache: parts.cache,
            object_storage: parts.object_storage,
            mail_sender: parts.mail_sender,
            metrics: parts.metrics,
            started_at: Self::new_started_at(),
            yggdrasil_rate_limiter,
            yggdrasil_session_forward_http_client,
            background_task_dispatch_wakeup: Self::new_background_task_dispatch_wakeup(),
        })
    }

    pub fn new_started_at() -> Instant {
        Instant::now()
    }

    pub fn new_background_task_dispatch_wakeup() -> Arc<Notify> {
        Arc::new(Notify::new())
    }

    pub fn new_yggdrasil_rate_limiter(config: &Config) -> YggdrasilRateLimiter {
        YggdrasilRateLimiter::from_config(&config.rate_limit)
    }

    pub fn new_yggdrasil_session_forward_http_client() -> Result<reqwest::Client> {
        reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(YGGDRASIL_SESSION_FORWARD_USER_AGENT)
            .build()
            .map_err(|error| {
                AsterError::internal_error(format!(
                    "build Yggdrasil session forward HTTP client: {error}"
                ))
            })
    }

    pub fn writer_db(&self) -> &DatabaseConnection {
        self.db_handles.writer()
    }

    pub fn reader_db(&self) -> &DatabaseConnection {
        self.db_handles.reader()
    }

    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    pub fn runtime_config(&self) -> &Arc<RuntimeConfig> {
        &self.runtime_config
    }

    pub fn cache(&self) -> &Arc<dyn CacheBackend> {
        &self.cache
    }

    pub fn object_storage(&self) -> &Arc<dyn ObjectStorage> {
        &self.object_storage
    }

    pub fn mail_sender(&self) -> &Arc<dyn MailSender> {
        &self.mail_sender
    }

    pub fn metrics(&self) -> &SharedMetricsRecorder {
        &self.metrics
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn yggdrasil_rate_limiter(&self) -> &YggdrasilRateLimiter {
        &self.yggdrasil_rate_limiter
    }

    pub fn yggdrasil_session_forward_http_client(&self) -> &reqwest::Client {
        &self.yggdrasil_session_forward_http_client
    }

    pub fn background_task_dispatch_wakeup(&self) -> &Arc<Notify> {
        &self.background_task_dispatch_wakeup
    }
}
