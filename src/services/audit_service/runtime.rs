//! Audit runtime component integration.

use std::sync::Arc;

use crate::config::RuntimeConfig;
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use aster_forge_runtime::RuntimeComponentBundleRegistration;
use sea_orm::DatabaseConnection;

/// Minimal runtime resources needed to record the process shutdown audit event.
#[derive(Clone)]
pub struct AuditRuntimeResources {
    db: DatabaseConnection,
    runtime_config: Arc<RuntimeConfig>,
}

impl AuditRuntimeResources {
    /// Creates audit runtime resources from concrete runtime dependencies.
    pub fn new(db: DatabaseConnection, runtime_config: Arc<RuntimeConfig>) -> Self {
        Self { db, runtime_config }
    }

    /// Captures audit runtime resources from product runtime state.
    pub fn from_state<S>(state: &S) -> Self
    where
        S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
    {
        Self::new(state.writer_db().clone(), state.runtime_config().clone())
    }
}

impl DatabaseRuntimeState for AuditRuntimeResources {
    fn writer_db(&self) -> &DatabaseConnection {
        &self.db
    }

    fn reader_db(&self) -> &DatabaseConnection {
        &self.db
    }
}

impl RuntimeConfigRuntimeState for AuditRuntimeResources {
    fn runtime_config(&self) -> &Arc<RuntimeConfig> {
        &self.runtime_config
    }
}

/// Creates the audit runtime component used by the product entrypoint.
pub fn audit_component(
    resources: AuditRuntimeResources,
) -> RuntimeComponentBundleRegistration<impl aster_forge_runtime::RuntimeComponentBundle> {
    aster_forge_audit::audit_component(
        resources,
        record_server_start_on_startup,
        record_server_shutdown_on_shutdown,
        |()| async {
            super::shutdown_global_audit_log_manager().await;
            Ok(())
        },
    )
}

/// Creates the full audit runtime component from product state.
pub fn audit_runtime_component<S>(
    state: &S,
) -> RuntimeComponentBundleRegistration<impl aster_forge_runtime::RuntimeComponentBundle + use<S>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    audit_component(AuditRuntimeResources::from_state(state))
}

/// Initializes the global audit log manager for runtime writes.
pub fn prepare_runtime_audit_manager(db: DatabaseConnection) {
    super::init_global_audit_log_manager(db);
}

async fn record_server_shutdown_on_shutdown(
    resources: AuditRuntimeResources,
) -> Result<(), String> {
    record_server_shutdown(&resources).await;
    Ok(())
}

async fn record_server_start_on_startup(resources: AuditRuntimeResources) -> Result<(), String> {
    record_server_start(&resources).await;
    Ok(())
}

/// Records the process start audit event.
pub async fn record_server_start<S>(state: &S)
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    super::log(
        state,
        &super::AuditContext::system(),
        crate::types::audit::AuditAction::ServerStart,
        crate::types::audit::AuditEntityType::System,
        None,
        Some("server"),
        None,
    )
    .await;
}

/// Records the process shutdown audit event.
pub async fn record_server_shutdown<S>(state: &S)
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let backend = state.writer_db().get_database_backend();
    super::log(
        state,
        &super::AuditContext::system(),
        crate::types::audit::AuditAction::ServerShutdown,
        crate::types::audit::AuditEntityType::System,
        None,
        Some("server"),
        None,
    )
    .await;
    tracing::info!(?backend, "server shutdown recorded");
}

#[cfg(test)]
mod tests {
    use super::{AuditRuntimeResources, audit_component};
    use aster_forge_runtime::RuntimeComponentBundle;

    async fn test_resources() -> AuditRuntimeResources {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("audit runtime test database should connect");
        AuditRuntimeResources::new(db, std::sync::Arc::new(crate::config::RuntimeConfig::new()))
    }

    #[tokio::test]
    async fn audit_component_registers_shutdown_phase() {
        let resources = test_resources().await;
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            audit_component(resources).register(registry);
        });

        let descriptor = registry
            .descriptor(aster_forge_audit::AUDIT_LOGS_COMPONENT)
            .expect("audit logs component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Product
        );
        assert_eq!(
            descriptor.dependencies,
            vec![aster_forge_mail::MAIL_OUTBOX_COMPONENT]
        );
        assert_eq!(
            descriptor
                .shutdown
                .expect("server shutdown audit should be registered")
                .phase_name,
            aster_forge_audit::SERVER_SHUTDOWN_AUDIT_PHASE
        );

        let descriptor = registry
            .descriptor(aster_forge_audit::AUDIT_MANAGER_COMPONENT)
            .expect("audit manager component should be registered");
        assert_eq!(
            descriptor.dependencies,
            vec![aster_forge_audit::AUDIT_LOGS_COMPONENT]
        );
        assert_eq!(
            descriptor
                .shutdown
                .expect("audit manager shutdown should be registered")
                .phase_name,
            aster_forge_audit::AUDIT_MANAGER_FLUSH_SHUTDOWN_PHASE
        );
    }
}
