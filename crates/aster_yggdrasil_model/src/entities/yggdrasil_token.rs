//! Yggdrasil launcher token entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "yggdrasil_tokens")]
/// Yggdrasil launcher token: one accessToken issued to a launcher client for a user.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Account the token authenticates.
    pub user_id: i64,
    /// Hash of the accessToken; unique, and the plaintext value exists only in the
    /// launcher response.
    #[sea_orm(unique)]
    pub access_token_hash: String,
    /// Client token paired with this accessToken per the Yggdrasil protocol.
    pub client_token: String,
    /// Profile the launcher selected at authentication, if any; cleared when that
    /// profile is deleted.
    pub selected_profile_id: Option<i64>,
    /// When the token was issued.
    pub issued_at: DateTimeUtc,
    /// When the token expires.
    pub expires_at: DateTimeUtc,
    /// When the token was permanently revoked (logout or invalidation); `None` while valid.
    pub revoked_at: Option<DateTimeUtc>,
    /// When the token was temporarily invalidated, for example by a profile rename;
    /// validation rejects the token until the launcher refreshes it.
    pub temporarily_invalidated_at: Option<DateTimeUtc>,
    /// Launcher User-Agent captured at issuance.
    pub user_agent: Option<String>,
    /// Launcher client IP captured at issuance.
    pub ip_address: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the authenticated account and selected profile.
pub enum Relation {
    /// Account the token authenticates.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Profile the launcher selected at authentication.
    #[sea_orm(
        belongs_to = "super::minecraft_profile::Entity",
        from = "Column::SelectedProfileId",
        to = "super::minecraft_profile::Column::Id"
    )]
    SelectedProfile,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::minecraft_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SelectedProfile.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
