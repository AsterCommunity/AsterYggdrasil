//! User entity.

use crate::types::user::{UserRole, UserStatus};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(all(debug_assertions, feature = "openapi"), schema(as = UserModel))]
#[sea_orm(table_name = "users")]
/// A local user account of the skin station.
pub struct Model {
    /// Auto-incrementing surrogate primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Stable identifier exposed outside the database instead of the numeric id; unique.
    #[sea_orm(unique)]
    pub public_uuid: String,
    /// Login name; unique across accounts.
    #[sea_orm(unique)]
    pub username: String,
    /// Account email address used for verification and password reset; unique across accounts.
    #[sea_orm(unique)]
    pub email: String,
    /// Hashed password verifier; never serialized to clients.
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// Account privilege level (`admin`, `operator`, or `user`).
    pub role: UserRole,
    /// Account lifecycle state (`active` or `disabled`).
    pub status: UserStatus,
    /// Forces a password change before the account can continue after login.
    pub must_change_password: bool,
    /// Counter embedded in issued session tokens; incrementing it invalidates previously issued sessions.
    pub session_version: i64,
    /// When the account email was verified; `None` until a verification token is consumed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_verified_at: Option<DateTimeUtc>,
    /// Email awaiting confirmation after a change request; replaces the primary email once verified.
    pub pending_email: Option<String>,
    /// When the account was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the account was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations from a user account to its owned records.
pub enum Relation {
    /// Web login sessions opened by this account.
    #[sea_orm(has_many = "super::auth_session::Entity")]
    AuthSessions,
    /// External-provider identities linked to this account.
    #[sea_orm(has_many = "super::external_auth_identity::Entity")]
    ExternalAuthIdentities,
    /// Minecraft player profiles owned by this account.
    #[sea_orm(has_many = "super::minecraft_profile::Entity")]
    MinecraftProfiles,
    /// WebAuthn passkey credentials registered by this account.
    #[sea_orm(has_many = "super::passkey::Entity")]
    Passkeys,
    /// Display-profile extension row (display name, avatar).
    #[sea_orm(has_one = "super::user_profile::Entity")]
    UserProfile,
    /// Outstanding contact (email) verification tokens for this account.
    #[sea_orm(has_many = "super::contact_verification_token::Entity")]
    ContactVerificationTokens,
    /// Yggdrasil launcher tokens issued to this account.
    #[sea_orm(has_many = "super::yggdrasil_token::Entity")]
    YggdrasilTokens,
    /// Operator-panel scopes granted to this account.
    #[sea_orm(has_many = "super::user_operator_scope::Entity")]
    UserOperatorScopes,
}

impl Related<super::auth_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AuthSessions.def()
    }
}

impl Related<super::external_auth_identity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalAuthIdentities.def()
    }
}

impl Related<super::minecraft_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftProfiles.def()
    }
}

impl Related<super::passkey::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Passkeys.def()
    }
}

impl Related<super::user_profile::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserProfile.def()
    }
}

impl Related<super::contact_verification_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ContactVerificationTokens.def()
    }
}

impl Related<super::yggdrasil_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::YggdrasilTokens.def()
    }
}

impl Related<super::user_operator_scope::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserOperatorScopes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
