//! SeaORM entity definition for `external_auth_login_flows`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "external_auth_login_flows")]
/// In-progress external auth login: the OIDC/OAuth2 authorization state started by
/// one browser and single-use consumed at the provider callback.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Provider being used for this login.
    pub provider_id: i64,
    /// Hash of the OAuth2/OIDC `state` parameter; unique, and the callback lookup key.
    pub state_hash: String,
    /// OIDC `nonce` value sent with the authorization request, validated against the ID token.
    pub nonce: Option<String>,
    /// PKCE code verifier for providers that require it.
    pub pkce_verifier: Option<String>,
    /// Callback URL the provider must redirect back to for this flow.
    pub redirect_uri: String,
    /// In-app path to redirect the browser to after a successful login.
    pub return_path: Option<String>,
    /// When the flow was started.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the flow expires and its callback is rejected.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub expires_at: DateTimeUtc,
    /// When the flow was consumed by a callback; `None` until then.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub consumed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the provider used for this login.
pub enum Relation {
    /// Provider whose authorization page the browser was sent to.
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
