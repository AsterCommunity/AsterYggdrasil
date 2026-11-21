//! Yggdrasil launcher token entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "yggdrasil_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    #[sea_orm(unique)]
    pub access_token_hash: String,
    pub client_token: String,
    pub selected_profile_id: Option<i64>,
    pub issued_at: DateTimeUtc,
    pub expires_at: DateTimeUtc,
    pub revoked_at: Option<DateTimeUtc>,
    pub temporarily_invalidated_at: Option<DateTimeUtc>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
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
