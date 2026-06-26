//! Capability traits exposed by the shared runtime state.

use std::sync::Arc;

use crate::config::{Config, RuntimeConfig};
use crate::object_storage::ObjectStorage;
use aster_forge_cache::CacheBackend;
use aster_forge_mail::MailSender;
use aster_forge_metrics::SharedMetricsRecorder;
use sea_orm::DatabaseConnection;

use super::state::AppState;

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
