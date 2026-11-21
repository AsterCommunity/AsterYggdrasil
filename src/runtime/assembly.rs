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
        .component(crate::api::http::http_component(
            crate::api::http::HttpRuntimeConfig {
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
    use super::worker_count;
    use crate::runtime::AppState;

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

        let state: AppState = crate::runtime::test_support::setup_state().await;
        let db_handles = state.db_handles.clone();
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            crate::tasks::runtime::task_component(aster_forge_tasks::BackgroundTasks::new())
                .register(registry);
            crate::services::mail_outbox_service::runtime::mail_runtime_component(&state)
                .register(registry);
            crate::services::audit_service::runtime::audit_runtime_component(&state)
                .register(registry);
            crate::db::runtime::database_component(db_handles).register(registry);
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
                aster_forge_audit::AUDIT_MANAGER_COMPONENT,
                aster_forge_audit::AUDIT_LOGS_COMPONENT,
                aster_forge_db::DATABASE_COMPONENT
            ]
        );

        let audit_logs = registry
            .descriptor(aster_forge_audit::AUDIT_LOGS_COMPONENT)
            .expect("audit logs component should be registered");
        assert!(
            audit_logs
                .startup
                .iter()
                .any(|phase| phase.phase_name == aster_forge_audit::SERVER_START_AUDIT_PHASE),
            "server start audit phase should be registered"
        );
        assert!(
            audit_logs
                .shutdown
                .iter()
                .any(|phase| phase.phase_name == aster_forge_audit::SERVER_SHUTDOWN_AUDIT_PHASE),
            "server shutdown audit phase should be registered"
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
