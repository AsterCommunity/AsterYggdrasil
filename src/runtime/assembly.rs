//! Runtime component assembly.
//!
//! This module turns prepared product state into the concrete Forge runtime.
//! It keeps the process entrypoint focused on bootstrap and execution while
//! centralizing the Yggdrasil-specific component graph. The serve role gates
//! which responsibilities each process mounts: task consumers only run when
//! the role consumes tasks, and the HTTP service degrades to health probes
//! when the role does not serve the API.

use std::io;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use actix_web::web;

use crate::runtime::role::{RoleHealth, ServeRole};

/// Assembles and runs the Forge runtime from prepared product state.
pub async fn run(state: crate::runtime::AppState, role: ServeRole) -> io::Result<()> {
    let host = state.config.server.host.clone();
    let port = state.config.server.port;
    let workers = http_worker_count(state.config.server.workers, role.serves_api());
    let drain_timeout_secs = state.config.worker.drain_timeout_secs;
    let cancellation_timeout_secs = state.config.worker.cancellation_timeout_secs;

    tracing::info!(
        role = role.as_str(),
        host = %host,
        port,
        workers,
        drain_timeout_secs,
        cancellation_timeout_secs,
        "starting AsterYggdrasil"
    );

    let state = web::Data::new(state);
    let metrics_data: web::Data<dyn aster_forge_metrics::MetricsRecorder> =
        web::Data::from(state.get_ref().metrics.clone());
    let app_state = state.get_ref();

    // Queue consumer statuses drive the per-instance consumer component and
    // the HTTP probes; the scheduled group reports through its own status.
    let consumer_statuses = if role.consumes_tasks() {
        crate::services::ops::task::consumer_statuses()
    } else {
        Vec::new()
    };
    let scheduled_status = aster_forge_tasks::TaskRuntimeStatus::default();
    // Only consuming roles mount the leased scheduled group; an api process
    // must report an empty list so probes do not wait on a runner that never
    // starts.
    let scheduled_consumers = if role.consumes_tasks() {
        vec![scheduled_status.clone()]
    } else {
        Vec::new()
    };
    let initialized = Arc::new(AtomicBool::new(false));
    let role_initialization = initialized.clone();
    let runtime = aster_forge_runtime::AsterRuntime::builder()
        .component(crate::api::http::http_component(
            crate::api::http::HttpRuntimeConfig {
                host: host.as_str(),
                port,
                workers,
                shutdown_timeout_secs: app_state.config.worker.drain_timeout_secs,
            },
            state.clone(),
            metrics_data,
            RoleHealth {
                role,
                consumers: consumer_statuses.clone(),
                scheduled: scheduled_consumers,
                shutdown: Default::default(),
                initialized,
            },
        ))?
        .component(crate::runtime::tasks::maintenance_component(state.clone()));
    let runtime = if role.consumes_tasks() {
        runtime
            .component(crate::runtime::tasks::task_consumers_component(
                state.clone(),
                consumer_statuses,
            ))
            .component(crate::runtime::tasks::scheduled_tasks_component(
                state.clone(),
                scheduled_status,
            ))
    } else {
        runtime
    };
    let runtime = runtime
        .component(crate::services::mail::outbox_runtime::mail_runtime_component(app_state, role))
        .component(crate::services::ops::audit::runtime::audit_runtime_component(app_state))
        .component(crate::db::runtime::database_component(
            app_state.db_handles.clone(),
            role,
        ))
        .component(role_component(role_initialization));

    runtime
        .run()
        .await
        .map_err(|error| io::Error::other(error.to_string()))?
}

fn http_worker_count(configured_workers: usize, serves_api: bool) -> usize {
    if configured_workers == 0 && !serves_api {
        1
    } else if configured_workers == 0 {
        num_cpus::get()
    } else {
        configured_workers
    }
}

/// Startup marker that flips [`RoleHealth`] to ready once every component has
/// started. Naming follows the AsterDrive `process_role` component so dashboards
/// can treat both products uniformly.
fn role_component(
    initialized: Arc<AtomicBool>,
) -> aster_forge_runtime::RuntimeComponentBundleRegistration<
    impl aster_forge_runtime::RuntimeComponentBundle,
> {
    aster_forge_runtime::runtime_component(
        move |registry: &mut aster_forge_runtime::RuntimeComponentRegistry| {
            let initialization = initialized.clone();
            registry.component_startup(
                "process_role",
                aster_forge_runtime::RuntimeComponentKind::Core,
                "role_initialized",
                aster_forge_runtime::StartupPhaseFailurePolicy::Required,
                move || {
                    let initialization = initialization.clone();
                    async move {
                        initialization.store(true, std::sync::atomic::Ordering::Release);
                        Ok(())
                    }
                },
            );
        },
    )
}

#[cfg(test)]
mod tests {
    use super::http_worker_count;
    use crate::runtime::AppState;
    use crate::runtime::role::ServeRole;
    use aster_forge_runtime::RuntimeComponentBundle;

    #[test]
    fn worker_count_uses_cpu_count_when_configured_zero() {
        assert_eq!(http_worker_count(0, true), num_cpus::get());
    }

    #[test]
    fn worker_count_uses_explicit_value() {
        assert_eq!(http_worker_count(4, true), 4);
    }

    #[test]
    fn worker_role_operational_http_defaults_to_one_thread() {
        assert_eq!(http_worker_count(0, false), 1);
        assert_eq!(http_worker_count(3, false), 3);
    }

    #[tokio::test]
    async fn all_role_component_graph_matches_single_process_wiring() {
        let registry = role_registry(ServeRole::All).await;
        registry
            .validate()
            .expect("all-role runtime component graph should validate");
        assert_component_names(
            &registry,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
                crate::runtime::tasks::TASK_CONSUMERS_COMPONENT,
                aster_forge_mail::MAIL_OUTBOX_COMPONENT,
                aster_forge_audit::AUDIT_MANAGER_COMPONENT,
                aster_forge_audit::AUDIT_LOGS_COMPONENT,
                aster_forge_db::DATABASE_COMPONENT,
                "process_role",
            ],
        );
        assert_database_dependencies(
            &registry,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT.to_string(),
                crate::runtime::tasks::TASK_CONSUMERS_COMPONENT.to_string(),
                aster_forge_mail::MAIL_OUTBOX_COMPONENT.to_string(),
                aster_forge_audit::AUDIT_MANAGER_COMPONENT.to_string(),
            ],
        );
    }

    #[tokio::test]
    async fn api_role_drops_task_consumers_from_the_graph() {
        let registry = role_registry(ServeRole::Api).await;
        registry
            .validate()
            .expect("api-role runtime component graph should validate");
        let names = component_names(&registry);
        assert!(
            !names.contains(&crate::runtime::tasks::TASK_CONSUMERS_COMPONENT),
            "api role must not register task consumers: {names:?}"
        );
        assert_database_dependencies(
            &registry,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT.to_string(),
                aster_forge_mail::MAIL_OUTBOX_COMPONENT.to_string(),
                aster_forge_audit::AUDIT_MANAGER_COMPONENT.to_string(),
            ],
        );
    }

    #[tokio::test]
    async fn worker_role_component_graph_keeps_consumers_without_product_api_components() {
        let registry = role_registry(ServeRole::Worker).await;
        registry
            .validate()
            .expect("worker-role runtime component graph should validate");
        let names = component_names(&registry);
        assert!(
            names.contains(&crate::runtime::tasks::TASK_CONSUMERS_COMPONENT),
            "worker role must consume tasks: {names:?}"
        );
        assert_database_dependencies(
            &registry,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT.to_string(),
                crate::runtime::tasks::TASK_CONSUMERS_COMPONENT.to_string(),
                aster_forge_mail::MAIL_OUTBOX_COMPONENT.to_string(),
                aster_forge_audit::AUDIT_MANAGER_COMPONENT.to_string(),
            ],
        );
    }

    #[tokio::test]
    async fn consumer_component_registers_background_task_definitions_and_shutdown() {
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            aster_forge_runtime::runtime_component(
                aster_forge_tasks::BackgroundTaskRuntimeDefinitionsComponent::new(
                    aster_forge_tasks::BackgroundTasks::new(),
                    crate::services::ops::task::registered_system_runtime_tasks(),
                )
                .named(crate::runtime::tasks::TASK_CONSUMERS_COMPONENT),
            )
            .register(registry);
        });

        let descriptor = registry
            .descriptor(crate::runtime::tasks::TASK_CONSUMERS_COMPONENT)
            .expect("task consumers component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Tasks
        );
        assert_eq!(descriptor.shutdown.len(), 1);
        assert_eq!(
            descriptor.shutdown[0].phase_name,
            aster_forge_tasks::BACKGROUND_TASKS_SHUTDOWN_PHASE
        );
        assert_eq!(
            descriptor
                .tasks
                .iter()
                .map(|task| task.task_name)
                .collect::<Vec<_>>(),
            crate::services::ops::task::registered_system_runtime_tasks()
                .iter()
                .map(|task| task.wire_value)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn maintenance_component_registers_shutdown_phase_without_task_definitions() {
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            aster_forge_tasks::background_task_component(aster_forge_tasks::BackgroundTasks::new())
                .register(registry);
        });

        let descriptor = registry
            .descriptor(aster_forge_tasks::BACKGROUND_TASKS_COMPONENT)
            .expect("maintenance component should keep the shared tasks name");
        assert_eq!(descriptor.shutdown.len(), 1);
        assert_eq!(
            descriptor.shutdown[0].phase_name,
            aster_forge_tasks::BACKGROUND_TASKS_SHUTDOWN_PHASE
        );
        assert!(
            descriptor.tasks.is_empty(),
            "maintenance runs no recorded tasks: {:?}",
            descriptor.tasks
        );
    }

    async fn role_registry(role: ServeRole) -> aster_forge_runtime::RuntimeComponentRegistry {
        let state: AppState = crate::runtime::test_support::setup_state().await;
        let db_handles = state.db_handles.clone();
        // The FromShutdown component shapes only mount on the runtime builder;
        // these bundle equivalents carry the same component names and filters,
        // which is what the shutdown dependency graph is derived from.
        aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            aster_forge_tasks::background_task_component(aster_forge_tasks::BackgroundTasks::new())
                .register(registry);
            if role.consumes_tasks() {
                aster_forge_runtime::runtime_component(
                    aster_forge_tasks::BackgroundTaskRuntimeDefinitionsComponent::new(
                        aster_forge_tasks::BackgroundTasks::new(),
                        crate::services::ops::task::registered_system_runtime_tasks(),
                    )
                    .named(crate::runtime::tasks::TASK_CONSUMERS_COMPONENT),
                )
                .register(registry);
            }
            crate::services::mail::outbox_runtime::mail_runtime_component(&state, role)
                .register(registry);
            crate::services::ops::audit::runtime::audit_runtime_component(&state)
                .register(registry);
            crate::db::runtime::database_component(db_handles, role).register(registry);
            super::role_component(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                false,
            )))
            .register(registry);
        })
    }

    fn component_names(
        registry: &aster_forge_runtime::RuntimeComponentRegistry,
    ) -> Vec<&'static str> {
        registry
            .descriptors()
            .iter()
            .map(|descriptor| descriptor.name)
            .collect()
    }

    fn assert_component_names(
        registry: &aster_forge_runtime::RuntimeComponentRegistry,
        expected: Vec<&'static str>,
    ) {
        assert_eq!(component_names(registry), expected);
    }

    fn assert_database_dependencies(
        registry: &aster_forge_runtime::RuntimeComponentRegistry,
        expected: Vec<String>,
    ) {
        assert_eq!(
            registry
                .descriptor(aster_forge_db::DATABASE_COMPONENT)
                .expect("database component should be registered")
                .dependencies,
            expected
        );
    }
}
