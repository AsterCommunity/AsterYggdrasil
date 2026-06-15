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
use crate::metrics_core::SharedMetricsRecorder;
use crate::services::mail_service::MailSender;
use crate::texture_storage::TextureStorage;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone)]
pub struct AppState {
    pub db_handles: DbHandles,
    pub config: Arc<Config>,
    pub runtime_config: Arc<RuntimeConfig>,
    pub cache: Arc<dyn CacheBackend>,
    pub texture_storage: Arc<dyn TextureStorage>,
    pub mail_sender: Arc<dyn MailSender>,
    pub metrics: SharedMetricsRecorder,
    pub yggdrasil_rate_limiter: YggdrasilRateLimiter,
    pub background_task_dispatch_wakeup: Arc<Notify>,
}

impl AppState {
    pub fn new_background_task_dispatch_wakeup() -> Arc<Notify> {
        Arc::new(Notify::new())
    }

    pub fn new_yggdrasil_rate_limiter(config: &Config) -> YggdrasilRateLimiter {
        YggdrasilRateLimiter::from_config(&config.rate_limit)
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

    pub fn texture_storage(&self) -> &Arc<dyn TextureStorage> {
        &self.texture_storage
    }

    pub fn mail_sender(&self) -> &Arc<dyn MailSender> {
        &self.mail_sender
    }

    pub fn metrics(&self) -> &SharedMetricsRecorder {
        &self.metrics
    }

    pub fn yggdrasil_rate_limiter(&self) -> &YggdrasilRateLimiter {
        &self.yggdrasil_rate_limiter
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

pub trait TextureStorageRuntimeState {
    fn texture_storage(&self) -> &Arc<dyn TextureStorage>;
}

pub trait MetricsRuntimeState {
    fn metrics(&self) -> &SharedMetricsRecorder;
}

pub trait SharedRuntimeState:
    DatabaseRuntimeState
    + AppConfigRuntimeState
    + RuntimeConfigRuntimeState
    + CacheRuntimeState
    + TextureStorageRuntimeState
    + MetricsRuntimeState
{
}

impl<T> SharedRuntimeState for T where
    T: DatabaseRuntimeState
        + AppConfigRuntimeState
        + RuntimeConfigRuntimeState
        + CacheRuntimeState
        + TextureStorageRuntimeState
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

impl TextureStorageRuntimeState for AppState {
    fn texture_storage(&self) -> &Arc<dyn TextureStorage> {
        self.texture_storage()
    }
}

impl MetricsRuntimeState for AppState {
    fn metrics(&self) -> &SharedMetricsRecorder {
        self.metrics()
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
