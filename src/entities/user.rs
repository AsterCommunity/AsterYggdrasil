//! User entity.

use crate::types::{UserRole, UserStatus};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(all(debug_assertions, feature = "openapi"), schema(as = UserModel))]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub public_uuid: String,
    #[sea_orm(unique)]
    pub username: String,
    #[sea_orm(unique)]
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub must_change_password: bool,
    pub session_version: i64,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_verified_at: Option<DateTimeUtc>,
    pub pending_email: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::auth_session::Entity")]
    AuthSessions,
    #[sea_orm(has_many = "super::external_auth_identity::Entity")]
    ExternalAuthIdentities,
    #[sea_orm(has_many = "super::minecraft_profile::Entity")]
    MinecraftProfiles,
    #[sea_orm(has_many = "super::passkey::Entity")]
    Passkeys,
    #[sea_orm(has_one = "super::user_profile::Entity")]
    UserProfile,
    #[sea_orm(has_many = "super::contact_verification_token::Entity")]
    ContactVerificationTokens,
    #[sea_orm(has_many = "super::yggdrasil_token::Entity")]
    YggdrasilTokens,
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

impl ActiveModelBehavior for ActiveModel {}
