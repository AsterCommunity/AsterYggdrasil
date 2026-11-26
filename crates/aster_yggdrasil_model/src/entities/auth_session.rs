//! Auth session entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(all(debug_assertions, feature = "openapi"), schema(as = AuthSessionModel))]
#[sea_orm(table_name = "auth_sessions")]
/// Web login session tracking refresh-token rotation for one user.
pub struct Model {
    /// Random session identifier serving as the primary key (not auto-incremented).
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    /// Owning user account.
    pub user_id: i64,
    /// JWT id of the refresh token currently valid for this session; unique.
    pub current_refresh_jti: String,
    /// JWT id of the refresh token rotated out during the most recent refresh, kept so a
    /// replay of the old token can be detected until it expires.
    pub previous_refresh_jti: Option<String>,
    /// Absolute expiry of the current refresh token.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub refresh_expires_at: DateTimeUtc,
    /// Client User-Agent header captured at login.
    pub user_agent: Option<String>,
    /// Client IP captured at login.
    pub ip_address: Option<String>,
    /// When the session was opened.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When this session was last observed active (login or refresh).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub last_seen_at: DateTimeUtc,
    /// When the session was explicitly revoked (for example on logout); `None` while live.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub revoked_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the owning account.
pub enum Relation {
    /// Owning user account.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
