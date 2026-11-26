//! SeaORM entity definition for `external_auth_email_verification_flows`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "external_auth_email_verification_flows")]
/// Pending email-verification flow for an external auth subject whose provider
/// did not return a verified email address.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Provider whose login produced this flow.
    pub provider_id: i64,
    /// Issuer namespace that scoped the external subject.
    pub identity_namespace: String,
    /// External subject being verified.
    pub subject: String,
    /// Email claimed for the subject that must be proven by the verification email; `None`
    /// when the provider offered none and linking must rely on other criteria.
    pub target_email: Option<String>,
    /// Provider display name claim captured at flow start.
    pub display_name_snapshot: Option<String>,
    /// Provider preferred-username claim captured at flow start.
    pub preferred_username_snapshot: Option<String>,
    /// In-app path to return to after the flow completes.
    pub return_path: Option<String>,
    /// Hash of the flow token carried in URLs; unique.
    pub flow_token_hash: String,
    /// Hash of the token embedded in the verification email once it has been sent.
    pub verification_token_hash: Option<String>,
    /// When the verification email was requested; gates resend cooldowns.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_requested_at: Option<DateTimeUtc>,
    /// When the flow was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the flow expires and can no longer be completed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub expires_at: DateTimeUtc,
    /// When the flow was completed; `None` while pending.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub consumed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the originating provider.
pub enum Relation {
    /// Provider whose login started this flow.
    #[sea_orm(
        belongs_to = "super::external_auth_provider::Entity",
        from = "Column::ProviderId",
        to = "super::external_auth_provider::Column::Id"
    )]
    ExternalAuthProvider,
}

impl Related<super::external_auth_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalAuthProvider.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
