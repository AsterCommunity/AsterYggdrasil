//! Audit runtime component integration.
//!
//! The audit writer is product-owned (AsterDrive model): the runtime state carries an
//! `Arc<AsterForgeAuditWriter>`, this bundle starts it, records the product's lifecycle
//! audit entries, and always drains it before the database component shuts down.

use std::sync::Arc;
use std::time::Duration;

use crate::config::RuntimeConfig;
use crate::runtime::{AuditRuntimeState, DatabaseRuntimeState, RuntimeConfigRuntimeState};
use aster_forge_audit::AuditWriter;
use aster_forge_runtime::RuntimeComponentBundleRegistration;
use sea_orm::DatabaseConnection;

/// Drain timeout used by the standard audit bundle shutdown phase.
pub const AUDIT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimal runtime resources needed to start the audit writer and record the
/// process lifecycle audit events.
#[derive(Clone)]
pub struct AuditRuntimeResources {
    db: DatabaseConnection,
    runtime_config: Arc<RuntimeConfig>,
    audit_writer: Arc<AuditWriter>,
}

impl AuditRuntimeResources {
    /// Creates audit runtime resources from concrete runtime dependencies.
    pub fn new(
        db: DatabaseConnection,
        runtime_config: Arc<RuntimeConfig>,
        audit_writer: Arc<AuditWriter>,
    ) -> Self {
        Self {
            db,
            runtime_config,
            audit_writer,
        }
    }

    /// Captures audit runtime resources from product runtime state.
    pub fn from_state<S>(state: &S) -> Self
    where
        S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AuditRuntimeState,
    {
        Self::new(
            state.writer_db().clone(),
            state.runtime_config().clone(),
            state.audit_writer().clone(),
        )
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

impl AuditRuntimeState for AuditRuntimeResources {
    fn audit_writer(&self) -> &Arc<AuditWriter> {
        &self.audit_writer
    }
}

/// Creates the audit runtime component used by the product entrypoint.
///
/// The bundle starts the owned writer first, records the server start event, and on
/// shutdown records the server shutdown event before draining the writer. With the
/// `mail-outbox-dependency` feature, mail drain finishes before the shutdown record
/// is admitted.
pub fn audit_component(
    resources: AuditRuntimeResources,
) -> RuntimeComponentBundleRegistration<impl aster_forge_runtime::RuntimeComponentBundle> {
    let writer = resources.audit_writer.clone();
    aster_forge_audit::audit_component_with_writer(
        resources,
        writer,
        AUDIT_DRAIN_TIMEOUT,
        record_server_start_on_startup,
        record_server_shutdown_on_shutdown,
    )
}

/// Creates the full audit runtime component from product state.
pub fn audit_runtime_component<S>(
    state: &S,
) -> RuntimeComponentBundleRegistration<impl aster_forge_runtime::RuntimeComponentBundle + use<S>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + AuditRuntimeState,
{
    audit_component(AuditRuntimeResources::from_state(state))
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
    S: AuditRuntimeState + RuntimeConfigRuntimeState,
{
    super::log(
        state,
        &super::AuditContext::system(),
        aster_yggdrasil_model::types::audit::AuditAction::ServerStart,
        aster_yggdrasil_model::types::audit::AuditEntityType::System,
        None,
        Some("server"),
        None,
    )
    .await;
}

/// Records the process shutdown audit event.
pub async fn record_server_shutdown<S>(state: &S)
where
    S: AuditRuntimeState + DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let backend = state.writer_db().get_database_backend();
    super::log(
        state,
        &super::AuditContext::system(),
        aster_yggdrasil_model::types::audit::AuditAction::ServerShutdown,
        aster_yggdrasil_model::types::audit::AuditEntityType::System,
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
        AuditRuntimeResources::new(
            db.clone(),
            std::sync::Arc::new(crate::config::RuntimeConfig::new()),
            std::sync::Arc::new(aster_forge_audit::AuditWriter::new(db)),
        )
    }

    #[tokio::test]
    async fn audit_component_registers_writer_and_lifecycle_phases() {
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
            descriptor.shutdown
                .iter()
                .any(|phase| phase.phase_name == aster_forge_audit::SERVER_SHUTDOWN_AUDIT_PHASE),
            true,
            "shutdown audit phase should be registered"
        );

        let descriptor = registry
            .descriptor(aster_forge_audit::AUDIT_MANAGER_COMPONENT)
            .expect("audit manager component should be registered");
        assert_eq!(
            descriptor.shutdown
                .iter()
                .any(|phase| phase.phase_name
                    == aster_forge_audit::AUDIT_MANAGER_FLUSH_SHUTDOWN_PHASE),
            true,
            "audit writer drain phase should be registered"
        );
    }
}
