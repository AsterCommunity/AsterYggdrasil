//! Audit runtime component integration.
//!
//! The audit writer is product-owned (AsterDrive model): the runtime state carries an
//! `Arc<aster_forge_audit::AuditWriter>`, this bundle starts it, records the product's
//! lifecycle audit entries, and always drains it before the database component shuts down.

use std::time::Duration;

use aster_forge_runtime::RuntimeComponentBundleRegistration;

use crate::runtime::SharedRuntimeState;

/// Drain timeout used by the standard audit bundle shutdown phase.
pub const AUDIT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Creates the audit runtime component from product state.
///
/// The bundle starts the owned writer first, records the server start event, and on
/// shutdown records the server shutdown event before draining the writer. With the
/// `mail-outbox-dependency` feature, mail drain finishes before the shutdown record
/// is admitted.
pub fn audit_runtime_component<S>(
    state: &S,
) -> RuntimeComponentBundleRegistration<impl aster_forge_runtime::RuntimeComponentBundle + use<S>>
where
    S: SharedRuntimeState + Clone + Send + Sync + 'static,
{
    let writer = state.audit_writer().clone();
    aster_forge_audit::audit_component_with_writer(
        state.clone(),
        writer,
        AUDIT_DRAIN_TIMEOUT,
        |state| async move {
            record_server_start(&state).await;
            Ok(())
        },
        |state| async move {
            record_server_shutdown(&state).await;
            Ok(())
        },
    )
}

/// Records the process start audit event.
pub async fn record_server_start<S>(state: &S)
where
    S: SharedRuntimeState + Sync,
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
    S: SharedRuntimeState + Sync,
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
    use aster_forge_runtime::RuntimeComponentBundle;

    use super::audit_runtime_component;

    #[tokio::test]
    async fn audit_component_registers_writer_and_lifecycle_phases() {
        let state = crate::runtime::test_support::setup_state().await;
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            audit_runtime_component(&state).register(registry);
        });

        let descriptor = registry
            .descriptor(aster_forge_audit::AUDIT_LOGS_COMPONENT)
            .expect("audit logs component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Product
        );
        assert!(
            descriptor
                .shutdown
                .iter()
                .any(|phase| phase.phase_name == aster_forge_audit::SERVER_SHUTDOWN_AUDIT_PHASE),
            "shutdown audit phase should be registered"
        );

        let descriptor = registry
            .descriptor(aster_forge_audit::AUDIT_MANAGER_COMPONENT)
            .expect("audit manager component should be registered");
        assert!(
            descriptor.shutdown.iter().any(|phase| phase.phase_name
                == aster_forge_audit::AUDIT_MANAGER_FLUSH_SHUTDOWN_PHASE),
            "audit writer drain phase should be registered"
        );
    }
}
