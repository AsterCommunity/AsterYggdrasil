//! User-owned Minecraft texture asset entity.

use crate::types::{
    yggdrasil::MinecraftTextureLibraryStatus, yggdrasil::MinecraftTextureModel,
    yggdrasil::MinecraftTextureType, yggdrasil::MinecraftTextureVisibility,
};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    schema(as = MinecraftTextureModelEntity)
)]
#[sea_orm(table_name = "minecraft_textures")]
/// User-uploaded texture asset (skin or cape), stored by content hash, with the
/// metadata used by the owner's wardrobe and the public texture library.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Owning user account.
    pub user_id: i64,
    /// Asset kind (`skin` or `cape`).
    pub texture_type: MinecraftTextureType,
    /// Content hash of the stored file; indexed so identical uploads can be found.
    pub hash: String,
    /// Storage object key the texture bytes live under.
    pub storage_key: String,
    /// MIME type of the stored file (PNG for accepted uploads).
    pub mime_type: String,
    /// File size in bytes.
    pub file_size: i64,
    /// Image width in pixels.
    pub width: i32,
    /// Image height in pixels.
    pub height: i32,
    /// Arm model this skin is drawn with (`default` wide or `slim`).
    pub texture_model: MinecraftTextureModel,
    /// Owner-selected listing mode (`private` or `public`).
    pub visibility: MinecraftTextureVisibility,
    /// Marks the asset as a wardrobe entry that can be browsed and re-equipped,
    /// distinct from textures currently bound to profiles.
    pub is_wardrobe_item: bool,
    /// Owner-chosen label shown in their texture list.
    pub display_name: Option<String>,
    /// Moderation state in the public texture library (`private`, `pending_review`,
    /// `published`, or `rejected`).
    pub library_status: MinecraftTextureLibraryStatus,
    /// When the texture was submitted for library review.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub library_submitted_at: Option<DateTimeUtc>,
    /// When the library review was completed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub library_reviewed_at: Option<DateTimeUtc>,
    /// Admin who performed the library review.
    pub library_reviewer_user_id: Option<i64>,
    /// Note the reviewer recorded alongside the decision.
    pub library_review_note: Option<String>,
    /// When the texture was uploaded.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the texture record was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations from a texture to its owner, equipped slots, and tags.
pub enum Relation {
    /// Owning user account.
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    /// Profiles that currently have this texture equipped.
    #[sea_orm(has_many = "super::minecraft_profile_texture::Entity")]
    MinecraftProfileTextures,
    /// Library tags attached to this texture.
    #[sea_orm(has_many = "super::minecraft_texture_tag_binding::Entity")]
    MinecraftTextureTagBindings,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::minecraft_profile_texture::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftProfileTextures.def()
    }
}

impl Related<super::minecraft_texture_tag_binding::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MinecraftTextureTagBindings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
