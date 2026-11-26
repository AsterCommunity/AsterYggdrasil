//! Passkey credential entity.

use crate::types::passkey::StoredPasskeyCredential;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(all(debug_assertions, feature = "openapi"), schema(as = PasskeyModel))]
#[sea_orm(table_name = "passkeys")]
/// Registered WebAuthn passkey credential used for passwordless sign-in.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Account the credential belongs to.
    pub user_id: i64,
    /// WebAuthn credential id issued by the authenticator; unique.
    pub credential_id: String,
    /// WebAuthn user handle this server assigned for the account.
    pub user_handle: String,
    /// Full credential record (public key, algorithm, and related fields) stored as JSON text.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub credential: StoredPasskeyCredential,
    /// User-chosen label shown in the account's passkey list.
    pub name: String,
    /// Transports reported at registration (for example `internal` or `hybrid`),
    /// stored as a JSON array string.
    pub transports: Option<String>,
    /// Whether the authenticator declared the credential eligible for cloud backup.
    pub backup_eligible: bool,
    /// Whether the authenticator currently reports the credential as backed up.
    pub backed_up: bool,
    /// Authenticator signature counter used to detect cloned credentials.
    pub sign_count: i64,
    /// When the credential was registered.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the credential record was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
    /// When the credential was last used to sign in.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_used_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the owning account.
pub enum Relation {
    /// Account the credential belongs to.
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
