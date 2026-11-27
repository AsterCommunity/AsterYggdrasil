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
use aster_forge_runtime::RuntimeComponentBundleRegistration;
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

/// Creates the mail outbox runtime component used by the product entrypoint.
///
/// The outbox drain waits for background task dispatch to stop so no new rows
/// are enqueued after draining. Roles that consume tasks additionally wait for
/// the dedicated task-consumer group before draining.
pub fn mail_outbox_component(
    resources: MailOutboxRuntimeResources,
    role: crate::runtime::role::ServeRole,
) -> RuntimeComponentBundleRegistration<
    aster_forge_runtime::ShutdownResourceComponent<MailOutboxRuntimeResources>,
> {
    let dependencies: &'static [&'static str] = if role.consumes_tasks() {
        &[
            aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
            crate::components::TASK_CONSUMERS_COMPONENT,
        ]
    } else {
        &[aster_forge_tasks::BACKGROUND_TASKS_COMPONENT]
    };
    // The outbox rows are shared cluster state; only roles that consume tasks
    // may drain them on shutdown. An api-only process must not claim and send
    // mail as a side effect of stopping, so its drain is a no-op that keeps
    // the component (and the shutdown dependency anchor) registered.
    type MailDrainFuture = std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<(), String>> + Send>,
    >;
    let drain: fn(MailOutboxRuntimeResources) -> MailDrainFuture = if role.consumes_tasks() {
        |resources| Box::pin(drain_mail_outbox_on_shutdown(resources))
    } else {
        |_resources| Box::pin(async { Ok(()) })
    };
    aster_forge_runtime::shutdown_resource_component_after(
        aster_forge_mail::MAIL_OUTBOX_COMPONENT,
        aster_forge_runtime::RuntimeComponentKind::Mail,
        aster_forge_mail::MAIL_OUTBOX_DRAIN_SHUTDOWN_PHASE,
        dependencies,
        resources,
        drain,
    )
}

/// Creates the mail runtime component from product state.
pub fn mail_runtime_component<S>(
    state: &S,
    role: crate::runtime::role::ServeRole,
) -> RuntimeComponentBundleRegistration<
    aster_forge_runtime::ShutdownResourceComponent<MailOutboxRuntimeResources>,
>
where
    S: MailRuntimeState,
{
    mail_outbox_component(MailOutboxRuntimeResources::from_state(state), role)
}

async fn drain_mail_outbox_on_shutdown(
    resources: MailOutboxRuntimeResources,
) -> Result<(), String> {
    super::outbox::drain_with(
        &resources.db,
        &resources.runtime_config,
        &resources.mail_sender,
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{MailOutboxRuntimeResources, mail_outbox_component};
    use crate::runtime::role::ServeRole;
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
            mail_outbox_component(resources, ServeRole::All).register(registry);
        });

        let descriptor = registry
            .descriptor(aster_forge_mail::MAIL_OUTBOX_COMPONENT)
            .expect("mail outbox component should be registered");
        assert_eq!(
            descriptor.kind,
            aster_forge_runtime::RuntimeComponentKind::Mail
        );
        assert_eq!(
            descriptor.dependencies,
            vec![
                aster_forge_tasks::BACKGROUND_TASKS_COMPONENT,
                crate::components::TASK_CONSUMERS_COMPONENT
            ]
        );
        assert_eq!(descriptor.shutdown.len(), 1);
        assert_eq!(
            descriptor.shutdown[0].phase_name,
            aster_forge_mail::MAIL_OUTBOX_DRAIN_SHUTDOWN_PHASE
        );
    }

    #[tokio::test]
    async fn api_role_mail_drain_does_not_wait_for_absent_consumers() {
        let resources = test_resources().await;
        let registry = aster_forge_runtime::RuntimeComponentRegistry::configured(|registry| {
            mail_outbox_component(resources, ServeRole::Api).register(registry);
        });

        let descriptor = registry
            .descriptor(aster_forge_mail::MAIL_OUTBOX_COMPONENT)
            .expect("mail outbox component should be registered");
        assert_eq!(
            descriptor.dependencies,
            vec![aster_forge_tasks::BACKGROUND_TASKS_COMPONENT],
            "api role never registers task consumers, so the drain must not depend on them"
        );
    }

    #[tokio::test]
    async fn mail_outbox_shutdown_drains_pending_rows() {
        let db = sea_orm::Database::connect("sqlite::memory:")
            .await
            .expect("mail outbox drain test database should connect");
        aster_yggdrasil_migration::Migrator::up(&db, None)
            .await
            .expect("mail outbox drain test migrations should run");
        crate::services::ops::config::ensure_defaults(&db)
            .await
            .expect("mail outbox drain test defaults should seed");

        let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
        runtime_config
            .reload(&db)
            .await
            .expect("mail outbox drain test runtime config should reload");
        let mail_sender = aster_forge_mail::memory_sender();
        let row = crate::services::mail::outbox::enqueue(
            &db,
            "operator@example.com",
            Some("Operator"),
            crate::services::mail::template::MailTemplatePayload::login_email_code(
                "operator",
                "123456",
                "AsterYggdrasil",
                "10 minutes",
            ),
        )
        .await
        .expect("mail outbox drain test row should enqueue");

        let mut registry = aster_forge_runtime::RuntimeComponentRegistry::new();
        registry.register_bundle(mail_outbox_component(
            MailOutboxRuntimeResources::new(db.clone(), runtime_config, mail_sender.clone()),
            ServeRole::All,
        ));
        let report = registry.shutdown().await;

        assert!(!report.has_failures());
        assert_eq!(report.phases.len(), 1);
        assert_eq!(
            report.phases[0].name,
            aster_forge_mail::MAIL_OUTBOX_DRAIN_SHUTDOWN_PHASE
        );

        let stored = aster_yggdrasil_model::entities::mail_outbox::Entity::find_by_id(row.id)
            .one(&db)
            .await
            .expect("mail outbox drain test row lookup should succeed")
            .expect("mail outbox drain test row should exist");
        assert_eq!(stored.status, aster_forge_mail::MailOutboxStatus::Sent);
        assert_eq!(
            stored.payload_json.as_ref(),
            aster_forge_mail::StoredMailPayload::CLEARED_JSON
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
