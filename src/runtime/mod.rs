//! Runtime state and lifecycle modules.

pub mod logging;
pub mod panic;
pub mod shutdown;
pub mod startup;
pub mod tasks;

use crate::api::middleware::yggdrasil_rate_limit::YggdrasilRateLimiter;
use crate::cache::CacheBackend;
use crate::config::{Config, RuntimeConfig};
use crate::db::DbHandles;
use crate::errors::{AsterError, Result};
use crate::object_storage::ObjectStorage;
use crate::services::mail_service::MailSender;
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

impl AppState {
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

pub trait DatabaseRuntimeState {
    fn writer_db(&self) -> &DatabaseConnection;
    fn reader_db(&self) -> &DatabaseConnection;
}

pub trait AppConfigRuntimeState {
    fn config(&self) -> &Arc<Config>;
}

pub trait RuntimeConfigRuntimeState {
    fn runtime_config(&self) -> &Arc<RuntimeConfig>;
}

pub trait CacheRuntimeState {
    fn cache(&self) -> &Arc<dyn CacheBackend>;
}

pub trait ObjectStorageRuntimeState {
    fn object_storage(&self) -> &Arc<dyn ObjectStorage>;
}

pub trait MetricsRuntimeState {
    fn metrics(&self) -> &SharedMetricsRecorder;
}

pub trait YggdrasilSessionForwardRuntimeState {
    fn yggdrasil_session_forward_http_client(&self) -> &reqwest::Client;
}

pub trait SharedRuntimeState:
    DatabaseRuntimeState
    + AppConfigRuntimeState
    + RuntimeConfigRuntimeState
    + CacheRuntimeState
    + ObjectStorageRuntimeState
    + MetricsRuntimeState
{
}

impl<T> SharedRuntimeState for T where
    T: DatabaseRuntimeState
        + AppConfigRuntimeState
        + RuntimeConfigRuntimeState
        + CacheRuntimeState
        + ObjectStorageRuntimeState
        + MetricsRuntimeState
{
}

pub trait MailRuntimeState: DatabaseRuntimeState + RuntimeConfigRuntimeState {
    fn mail_sender(&self) -> &Arc<dyn MailSender>;
}

pub trait TaskRuntimeState: SharedRuntimeState {
    fn wake_background_task_dispatcher(&self);
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

impl CacheRuntimeState for AppState {
    fn cache(&self) -> &Arc<dyn CacheBackend> {
        self.cache()
    }
}

impl ObjectStorageRuntimeState for AppState {
    fn object_storage(&self) -> &Arc<dyn ObjectStorage> {
        self.object_storage()
    }
}

impl MetricsRuntimeState for AppState {
    fn metrics(&self) -> &SharedMetricsRecorder {
        self.metrics()
    }
}

impl YggdrasilSessionForwardRuntimeState for AppState {
    fn yggdrasil_session_forward_http_client(&self) -> &reqwest::Client {
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
        self.background_task_dispatch_wakeup.notify_one();
    }
}
