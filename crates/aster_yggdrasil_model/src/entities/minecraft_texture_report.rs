//! User-submitted reports for public texture library entries.

use crate::types::{
    yggdrasil::MinecraftTextureReportReason, yggdrasil::MinecraftTextureReportStatus,
};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "minecraft_texture_reports")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub texture_id: i64,
    pub reporter_user_id: i64,
    pub reason: MinecraftTextureReportReason,
    pub message: Option<String>,
    pub status: MinecraftTextureReportStatus,
    pub admin_note: Option<String>,
    pub handled_by_user_id: Option<i64>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub handled_at: Option<DateTimeUtc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::minecraft_texture::Entity",
        from = "Column::TextureId",
        to = "super::minecraft_texture::Column::Id"
    )]
    MinecraftTexture,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ReporterUserId",
        to = "super::user::Column::Id"
    )]
    ReporterUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::HandledByUserId",
        to = "super::user::Column::Id"
    )]
    HandlerUser,
}

impl Related<super::minecraft_texture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTexture.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
