//! Audit runtime component integration.

use std::sync::Arc;

use crate::config::RuntimeConfig;
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use aster_forge_runtime::{
    RuntimeComponentBundle, RuntimeComponentBundleRegistration, RuntimeComponentKind,
    RuntimeComponentRegistry, runtime_component,
};
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

/// Runtime component that records process shutdown and drains the global audit log manager.
pub struct AuditRuntimeComponent {
    resources: AuditRuntimeResources,
}

impl AuditRuntimeComponent {
    /// Creates an audit runtime component from owned runtime resources.
    pub const fn new(resources: AuditRuntimeResources) -> Self {
        Self { resources }
    }
}

impl RuntimeComponentBundle for AuditRuntimeComponent {
    fn register(self, registry: &mut RuntimeComponentRegistry) {
        register_server_shutdown_audit(registry, self.resources);
        register_audit_shutdown(registry);
    }
}

/// Creates the audit runtime component used by the product entrypoint.
pub fn audit_component(
    resources: AuditRuntimeResources,
) -> RuntimeComponentBundleRegistration<AuditRuntimeComponent> {
    runtime_component(AuditRuntimeComponent::new(resources))
}

/// Initializes the global audit log manager for runtime writes.
pub fn prepare_runtime_audit_manager(db: DatabaseConnection) {
    super::init_global_audit_log_manager(db);
}

/// Registers the process shutdown audit event before the audit manager is drained.
pub fn register_server_shutdown_audit(
    registry: &mut RuntimeComponentRegistry,
    resources: AuditRuntimeResources,
) {
    registry
        .component("audit_logs")
        .kind(RuntimeComponentKind::Product)
        .depends_on("mail_outbox")
        .shutdown_once(
            "server_shutdown_audit",
            None,
            resources,
            |resources| async move {
                record_server_shutdown(&resources).await;
                Ok(())
            },
        );
}

/// Registers graceful shutdown for the global audit log manager.
pub fn register_audit_shutdown(registry: &mut RuntimeComponentRegistry) {
    registry
        .component("audit_manager")
        .kind(RuntimeComponentKind::Product)
        .depends_on("audit_logs")
        .shutdown("audit_logs", None, || async {
            super::shutdown_global_audit_log_manager().await;
            Ok(())
        });
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
        crate::types::AuditAction::ServerShutdown,
        crate::types::AuditEntityType::System,
        None,
        Some("server"),
        None,
    )
    .await;
    tracing::info!(?backend, "server shutdown recorded");
}

#[cfg(test)]
mod tests {
    use super::{AuditRuntimeResources, audit_component, register_audit_shutdown};
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
            .descriptor("audit_logs")
            .expect("audit logs component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Product
        );
        assert_eq!(descriptor.dependencies, vec!["mail_outbox"]);
        assert_eq!(
            descriptor
                .shutdown
                .expect("server shutdown audit should be registered")
                .phase_name,
            "server_shutdown_audit"
        );

        let descriptor = registry
            .descriptor("audit_manager")
            .expect("audit manager component should be registered");
        assert_eq!(descriptor.dependencies, vec!["audit_logs"]);
        assert_eq!(
            descriptor
                .shutdown
                .expect("audit manager shutdown should be registered")
                .phase_name,
            "audit_logs"
        );
    }

    #[test]
    fn audit_shutdown_registrar_can_be_used_directly() {
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            register_audit_shutdown(registry);
        });

        assert!(registry.descriptor("audit_manager").is_some());
    }
}
