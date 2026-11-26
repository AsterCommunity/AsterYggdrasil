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
/// User-submitted report about a public texture-library entry, handled by admins.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Reported texture.
    pub texture_id: i64,
    /// Account that filed the report.
    pub reporter_user_id: i64,
    /// Reported problem category (`inappropriate`, `offensive`, `copyright`,
    /// `misleading`, `broken`, `spam`, or `other`).
    pub reason: MinecraftTextureReportReason,
    /// Free-form explanation supplied by the reporter.
    pub message: Option<String>,
    /// Moderation outcome (`pending`, `accepted`, or `rejected`).
    pub status: MinecraftTextureReportStatus,
    /// Note the handling admin recorded.
    pub admin_note: Option<String>,
    /// Admin who handled the report.
    pub handled_by_user_id: Option<i64>,
    /// When the report was handled; `None` while pending.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub handled_at: Option<DateTimeUtc>,
    /// When the report was filed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the report record was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations to the reported texture and the involved users.
pub enum Relation {
    /// Reported texture.
    #[sea_orm(
        belongs_to = "super::minecraft_texture::Entity",
        from = "Column::TextureId",
        to = "super::minecraft_texture::Column::Id"
    )]
    MinecraftTexture,
    /// Account that filed the report.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ReporterUserId",
        to = "super::user::Column::Id"
    )]
    ReporterUser,
    /// Admin who handled the report.
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
