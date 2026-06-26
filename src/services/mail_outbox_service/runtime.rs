//! Mail outbox runtime component integration.
//!
//! The mail outbox has process lifecycle behavior even though most mail
//! rendering and delivery code remains ordinary business service code. This
//! component drains claimable outbox rows after background task dispatch has
//! stopped and before database handles are closed.

use std::sync::Arc;

use crate::config::RuntimeConfig;
use crate::runtime::MailRuntimeState;
use aster_forge_mail::MailSender;
use aster_forge_runtime::{
    RuntimeComponentBundle, RuntimeComponentBundleRegistration, RuntimeComponentKind,
    RuntimeComponentRegistry, runtime_component,
};
use sea_orm::DatabaseConnection;

/// Minimal runtime resources needed to drain the mail outbox.
#[derive(Clone)]
pub struct MailOutboxRuntimeResources {
    db: DatabaseConnection,
    runtime_config: Arc<RuntimeConfig>,
    mail_sender: Arc<dyn MailSender>,
}

impl MailOutboxRuntimeResources {
    /// Creates mail outbox resources from the concrete runtime dependencies.
    pub fn new(
        db: DatabaseConnection,
        runtime_config: Arc<RuntimeConfig>,
        mail_sender: Arc<dyn MailSender>,
    ) -> Self {
        Self {
            db,
            runtime_config,
            mail_sender,
        }
    }

    /// Captures mail outbox resources from product runtime state.
    pub fn from_state<S>(state: &S) -> Self
    where
        S: MailRuntimeState,
    {
        Self::new(
            state.writer_db().clone(),
            state.runtime_config().clone(),
            state.mail_sender().clone(),
        )
    }
}

/// Runtime component that drains due mail outbox rows during graceful shutdown.
pub struct MailOutboxRuntimeComponent {
    resources: MailOutboxRuntimeResources,
}

impl MailOutboxRuntimeComponent {
    /// Creates a mail outbox runtime component from owned runtime resources.
    pub const fn new(resources: MailOutboxRuntimeResources) -> Self {
        Self { resources }
    }
}

impl RuntimeComponentBundle for MailOutboxRuntimeComponent {
    fn register(self, registry: &mut RuntimeComponentRegistry) {
        register_mail_outbox_shutdown(registry, self.resources);
    }
}

/// Creates the mail outbox runtime component used by the product entrypoint.
pub fn mail_outbox_component(
    resources: MailOutboxRuntimeResources,
) -> RuntimeComponentBundleRegistration<MailOutboxRuntimeComponent> {
    runtime_component(MailOutboxRuntimeComponent::new(resources))
}

/// Registers graceful mail outbox draining after runtime background tasks stop.
pub fn register_mail_outbox_shutdown(
    registry: &mut RuntimeComponentRegistry,
    resources: MailOutboxRuntimeResources,
) {
    registry
        .component("mail_outbox")
        .kind(RuntimeComponentKind::Mail)
        .depends_on("background_tasks")
        .shutdown_once(
            "mail_outbox_drain",
            None,
            resources,
            |resources| async move {
                super::drain_with(
                    &resources.db,
                    &resources.runtime_config,
                    &resources.mail_sender,
                )
                .await
                .map(|_| ())
                .map_err(|error| error.to_string())
            },
        );
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{MailOutboxRuntimeResources, mail_outbox_component, register_mail_outbox_shutdown};
    use aster_forge_runtime::RuntimeComponentBundle;
    use sea_orm::EntityTrait;

    async fn test_resources() -> MailOutboxRuntimeResources {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("mail outbox runtime test database should connect");
        MailOutboxRuntimeResources::new(
            db,
            Arc::new(crate::config::RuntimeConfig::new()),
            aster_forge_mail::memory_sender(),
        )
    }

    #[tokio::test]
    async fn mail_outbox_component_registers_shutdown_dependency() {
        let resources = test_resources().await;
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            mail_outbox_component(resources).register(registry);
        });

        let descriptor = registry
            .descriptor("mail_outbox")
            .expect("mail outbox component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Mail
        );
        assert_eq!(descriptor.dependencies, vec!["background_tasks"]);
        assert_eq!(
            descriptor
                .shutdown
                .expect("mail outbox shutdown should be registered")
                .phase_name,
            "mail_outbox_drain"
        );
    }

    #[tokio::test]
    async fn mail_outbox_shutdown_registrar_can_be_used_directly() {
        let resources = test_resources().await;
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            register_mail_outbox_shutdown(registry, resources);
        });

        assert!(registry.descriptor("mail_outbox").is_some());
    }

    #[tokio::test]
    async fn mail_outbox_shutdown_drains_pending_rows() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("mail outbox drain test database should connect");
        migration::Migrator::up(&db, None)
            .await
            .expect("mail outbox drain test migrations should run");
        crate::services::config_service::ensure_defaults(&db)
            .await
            .expect("mail outbox drain test defaults should seed");

        let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
        runtime_config
            .reload(&db)
            .await
            .expect("mail outbox drain test runtime config should reload");
        let mail_sender = aster_forge_mail::memory_sender();
        let row = super::super::enqueue(
            &db,
            "operator@example.com",
            Some("Operator"),
            crate::services::mail_template::MailTemplatePayload::login_email_code(
                "operator",
                "123456",
                "AsterYggdrasil",
                "10 minutes",
            ),
        )
        .await
        .expect("mail outbox drain test row should enqueue");

        let report =
            aster_forge_runtime::RuntimeComponentRegistry::shutdown_bundle(mail_outbox_component(
                MailOutboxRuntimeResources::new(db.clone(), runtime_config, mail_sender.clone()),
            ))
            .await;

        assert!(!report.has_failures());
        assert_eq!(report.phases.len(), 1);
        assert_eq!(report.phases[0].name, "mail_outbox_drain");

        let stored = crate::entities::mail_outbox::Entity::find_by_id(row.id)
            .one(&db)
            .await
            .expect("mail outbox drain test row lookup should succeed")
            .expect("mail outbox drain test row should exist");
        assert_eq!(stored.status, crate::types::MailOutboxStatus::Sent);
        assert_eq!(
            stored.payload_json.as_ref(),
            crate::types::StoredMailPayload::CLEARED_JSON
        );

        let sender = aster_forge_mail::memory_sender_ref(&mail_sender)
            .expect("mail outbox drain test should use memory sender");
        assert_eq!(sender.messages().len(), 1);
        assert_eq!(
            sender
                .last_message()
                .expect("mail outbox drain test message should exist")
                .to
                .address,
            "operator@example.com"
        );
    }
}
