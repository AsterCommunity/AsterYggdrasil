//! SeaORM entity definition for `external_auth_identities`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "external_auth_identities")]
/// Link between a local account and one subject at an external auth provider.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Local account the identity signs into.
    pub user_id: i64,
    /// Provider the identity belongs to.
    pub provider_id: i64,
    /// Issuer namespace that scopes the subject within the provider.
    pub identity_namespace: String,
    /// Stable subject identifier issued by the provider; unique per provider and namespace.
    pub subject: String,
    /// Email claim captured at the most recent link or login.
    pub email_snapshot: Option<String>,
    /// Display name claim captured at the most recent link or login.
    pub display_name_snapshot: Option<String>,
    /// When the identity was linked to the local account.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the identity was last refreshed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
    /// When this identity was last used to sign in.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_login_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the provider and the linked account.
pub enum Relation {
    /// Provider the identity belongs to.
    #[sea_orm(
        belongs_to = "super::external_auth_provider::Entity",
        from = "Column::ProviderId",
        to = "super::external_auth_provider::Column::Id"
    )]
    ExternalAuthProvider,
    /// Local account the identity signs into.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::external_auth_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalAuthProvider.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
