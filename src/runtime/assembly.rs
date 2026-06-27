//! Runtime component assembly.
//!
//! This module turns prepared product state into the concrete Forge runtime.
//! It keeps the process entrypoint focused on bootstrap and execution while
//! centralizing the Yggdrasil-specific component graph.

use std::io;

use actix_web::web;

/// Assembles and runs the Forge runtime from prepared product state.
pub async fn run(state: crate::runtime::AppState) -> io::Result<()> {
    let host = state.config.server.host.clone();
    let port = state.config.server.port;
    let workers = worker_count(state.config.server.workers);

    tracing::info!(host = %host, port, workers, "starting AsterYggdrasil HTTP service");

    let state = web::Data::new(state);
    let metrics_data: web::Data<dyn aster_forge_metrics::MetricsRecorder> =
        web::Data::from(state.get_ref().metrics.clone());
    let app_state = state.get_ref();

    let runtime = aster_forge_runtime::AsterRuntime::builder()
        .component(crate::runtime::http::http_component(
            crate::runtime::http::HttpRuntimeConfig {
                host: host.as_str(),
                port,
                workers,
            },
            state.clone(),
            metrics_data,
        ))?
        .component(crate::tasks::runtime::background_tasks_component(
            state.clone(),
        ))
        .component(crate::services::mail_outbox_service::runtime::mail_runtime_component(app_state))
        .component(crate::services::audit_service::runtime::audit_runtime_component(app_state))
        .component(crate::db::runtime::database_component(
            app_state.db_handles.clone(),
        ));

    runtime.run().await.map_err(to_io_error)?
}

fn worker_count(configured_workers: usize) -> usize {
    if configured_workers == 0 {
        num_cpus::get()
    } else {
        configured_workers
    }
}

fn to_io_error(error: impl ToString) -> io::Error {
    io::Error::other(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::worker_count;
    use crate::runtime::{DatabaseRuntimeState, MailRuntimeState, RuntimeConfigRuntimeState};
    use sea_orm::DatabaseConnection;

    struct ComponentGraphState {
        db: DatabaseConnection,
        runtime_config: Arc<crate::config::RuntimeConfig>,
        mail_sender: Arc<dyn aster_forge_mail::MailSender>,
    }

    impl DatabaseRuntimeState for ComponentGraphState {
        fn writer_db(&self) -> &DatabaseConnection {
            &self.db
        }

        fn reader_db(&self) -> &DatabaseConnection {
            &self.db
        }
    }

    impl RuntimeConfigRuntimeState for ComponentGraphState {
        fn runtime_config(&self) -> &Arc<crate::config::RuntimeConfig> {
            &self.runtime_config
        }
    }

    impl MailRuntimeState for ComponentGraphState {
        fn mail_sender(&self) -> &Arc<dyn aster_forge_mail::MailSender> {
            &self.mail_sender
        }
    }

    #[test]
    fn worker_count_uses_cpu_count_when_configured_zero() {
        assert_eq!(worker_count(0), num_cpus::get());
    }

    #[test]
    fn worker_count_uses_explicit_value() {
        assert_eq!(worker_count(4), 4);
    }

    #[tokio::test]
    async fn runtime_component_graph_registers_domain_components() {
        use aster_forge_runtime::RuntimeComponentBundle;

        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("component graph test database should connect");
        let db_handles = aster_forge_db::DbHandles::single(db.clone());
        let state = ComponentGraphState {
            db,
            runtime_config: Arc::new(crate::config::RuntimeConfig::new()),
            mail_sender: aster_forge_mail::memory_sender(),
        };
        let components = (
            crate::tasks::runtime::task_component(aster_forge_tasks::BackgroundTasks::new()),
            crate::services::mail_outbox_service::runtime::mail_runtime_component(&state),
            crate::services::audit_service::runtime::audit_runtime_component(&state),
            crate::db::runtime::database_component(db_handles),
        );
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            components.register(registry);
        });

        registry
            .validate()
            .expect("Yggdrasil runtime component graph should validate");
        let component_names = registry
            .descriptors()
            .iter()
            .map(|descriptor| descriptor.name)
            .collect::<Vec<_>>();
        assert_eq!(
            component_names,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
                aster_forge_mail::MAIL_OUTBOX_COMPONENT,
                aster_forge_audit::AUDIT_LOGS_COMPONENT,
                aster_forge_audit::AUDIT_MANAGER_COMPONENT,
                aster_forge_db::DATABASE_COMPONENT
            ]
        );

        let audit_logs = registry
            .descriptor(aster_forge_audit::AUDIT_LOGS_COMPONENT)
            .expect("audit logs component should be registered");
        assert_eq!(audit_logs.startup.len(), 1);
        assert_eq!(
            audit_logs.startup[0].phase_name,
            aster_forge_audit::SERVER_START_AUDIT_PHASE
        );
        assert_eq!(
            audit_logs
                .shutdown
                .expect("audit logs shutdown should be registered")
                .phase_name,
            aster_forge_audit::SERVER_SHUTDOWN_AUDIT_PHASE
        );
        assert_eq!(
            registry
                .descriptor(aster_forge_db::DATABASE_COMPONENT)
                .expect("database component should be registered")
                .dependencies,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
                aster_forge_mail::MAIL_OUTBOX_COMPONENT,
                aster_forge_audit::AUDIT_MANAGER_COMPONENT
            ]
        );
    }
}
