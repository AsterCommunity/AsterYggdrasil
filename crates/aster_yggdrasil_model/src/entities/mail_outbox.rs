//! SeaORM entity definition for `mail_outbox`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use aster_forge_mail::{MailOutboxStatus, MailTemplateCode, StoredMailPayload};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "mail_outbox")]
/// Outgoing email queued for delivery by the mail sender loop.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Forge mail template the email is rendered from.
    pub template_code: MailTemplateCode,
    /// Recipient email address.
    pub to_address: String,
    /// Recipient display name used in the email header.
    pub to_name: Option<String>,
    /// Template variables and email metadata stored as JSON text.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub payload_json: StoredMailPayload,
    /// Delivery state (`pending`, `processing`, `retry`, `sent`, `failed`).
    pub status: MailOutboxStatus,
    /// Delivery attempts made so far.
    pub attempt_count: i32,
    /// When the next delivery attempt is due.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub next_attempt_at: DateTimeUtc,
    /// When the current delivery attempt was claimed by a sender.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub processing_started_at: Option<DateTimeUtc>,
    /// When the email was delivered successfully.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub sent_at: Option<DateTimeUtc>,
    /// Message from the most recent delivery failure.
    pub last_error: Option<String>,
    /// When the email was queued.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the record was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No foreign-key relations; outbox rows stand alone.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
