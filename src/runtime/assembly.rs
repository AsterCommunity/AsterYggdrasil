//! Runtime component assembly.
//!
//! This module turns prepared product state into the concrete Forge runtime.
//! It keeps the process entrypoint focused on bootstrap and execution while
//! centralizing the Yggdrasil-specific component graph.

use std::io;

use actix_web::web;
use tokio_util::sync::CancellationToken;

/// Assembles and runs the Forge runtime from prepared product state.
pub async fn run(prepared: crate::runtime::startup::PreparedRuntime) -> io::Result<()> {
    let host = prepared.state.config.server.host.clone();
    let port = prepared.state.config.server.port;
    let workers = worker_count(prepared.state.config.server.workers);

    tracing::info!(host = %host, port, workers, "starting AsterYggdrasil HTTP service");

    let shutdown_token = CancellationToken::new();
    let state = web::Data::new(prepared.state);
    let shutdown_db_handles = state.get_ref().db_handles.clone();
    let audit_resources =
        crate::services::audit_service::runtime::AuditRuntimeResources::from_state(state.get_ref());
    let mail_outbox_resources =
        crate::services::mail_outbox_service::runtime::MailOutboxRuntimeResources::from_state(
            state.get_ref(),
        );
    let shutdown_data = web::Data::new(shutdown_token.clone());
    let metrics_data = web::Data::new(state.get_ref().metrics.clone());
    let background_tasks = crate::tasks::runtime::spawn_runtime_background_tasks(
        state.clone(),
        shutdown_token.clone(),
    );

    let http_component = crate::runtime::http::http_component(
        crate::runtime::http::HttpRuntimeConfig {
            host: host.as_str(),
            port,
            workers,
        },
        state.clone(),
        shutdown_data,
        metrics_data,
    )?;

    let runtime = aster_forge_runtime::AsterRuntime::builder()
        .component(http_component)
        .component(crate::tasks::runtime::task_component(background_tasks))
        .component(
            crate::services::mail_outbox_service::runtime::mail_outbox_component(
                mail_outbox_resources,
            ),
        )
        .component(crate::services::audit_service::runtime::audit_component(
            audit_resources,
        ))
        .component(crate::db::runtime::database_component(shutdown_db_handles));

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
    use aster_forge_runtime::RuntimeComponentBundle;
    use aster_forge_tasks::BackgroundTasks;

    #[test]
    fn worker_count_uses_cpu_count_when_configured_zero() {
        assert_eq!(worker_count(0), num_cpus::get());
    }

    #[test]
    fn worker_count_uses_explicit_value() {
        assert_eq!(worker_count(4), 4);
    }

    #[tokio::test]
    async fn runtime_components_register_shutdown_order_dependencies() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("component order test database should connect");
        let db_handles = aster_forge_db::DbHandles::single(db);
        let mail_outbox_resources =
            crate::services::mail_outbox_service::runtime::MailOutboxRuntimeResources::new(
                db_handles.writer().clone(),
                std::sync::Arc::new(crate::config::RuntimeConfig::new()),
                aster_forge_mail::memory_sender(),
            );
        let audit_resources = crate::services::audit_service::runtime::AuditRuntimeResources::new(
            db_handles.writer().clone(),
            std::sync::Arc::new(crate::config::RuntimeConfig::new()),
        );

        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            crate::tasks::runtime::task_component(BackgroundTasks::new()).register(registry);
            crate::services::mail_outbox_service::runtime::mail_outbox_component(
                mail_outbox_resources,
            )
            .register(registry);
            crate::services::audit_service::runtime::audit_component(audit_resources)
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
                "background_tasks",
                "mail_outbox",
                "audit_logs",
                "audit_manager",
                "database"
            ]
        );

        assert_eq!(
            registry
                .descriptor("mail_outbox")
                .expect("mail outbox component should be registered")
                .dependencies,
            vec!["background_tasks"]
        );
        assert_eq!(
            registry
                .descriptor("audit_logs")
                .expect("audit logs component should be registered")
                .dependencies,
            vec!["mail_outbox"]
        );
        assert_eq!(
            registry
                .descriptor("audit_manager")
                .expect("audit manager component should be registered")
                .dependencies,
            vec!["audit_logs"]
        );
        assert_eq!(
            registry
                .descriptor("database")
                .expect("database component should be registered")
                .dependencies,
            vec!["background_tasks", "mail_outbox", "audit_manager"]
        );
    }
}
