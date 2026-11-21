//! Minecraft texture asset and profile binding schema.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_minecraft_textures(manager).await?;
        create_minecraft_profile_textures(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MinecraftProfileTextures::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MinecraftTextures::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

async fn create_minecraft_textures(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MinecraftTextures::Table)
                .if_not_exists()
                .col(big_integer_pk(MinecraftTextures::Id))
                .col(
                    ColumnDef::new(MinecraftTextures::UserId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::TextureType)
                        .string_len(16)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::Hash)
                        .string_len(128)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::StorageKey)
                        .string_len(512)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::MimeType)
                        .string_len(64)
                        .not_null()
                        .default("image/png"),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::FileSize)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::Width)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::Height)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::TextureModel)
                        .string_len(16)
                        .not_null()
                        .default("default"),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::Visibility)
                        .string_len(16)
                        .not_null()
                        .default("private"),
                )
                .col(
                    ColumnDef::new(MinecraftTextures::IsWardrobeItem)
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .col(utc_timestamp(manager, MinecraftTextures::CreatedAt).not_null())
                .col(utc_timestamp(manager, MinecraftTextures::UpdatedAt).not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_minecraft_textures_user")
                        .from(MinecraftTextures::Table, MinecraftTextures::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

    for index in [
        Index::create()
            .name("idx_minecraft_textures_user_type")
            .table(MinecraftTextures::Table)
            .col(MinecraftTextures::UserId)
            .col(MinecraftTextures::TextureType)
            .to_owned(),
        Index::create()
            .name("idx_minecraft_textures_hash")
            .table(MinecraftTextures::Table)
            .col(MinecraftTextures::Hash)
            .to_owned(),
        Index::create()
            .name("idx_minecraft_textures_storage_key")
            .table(MinecraftTextures::Table)
            .col(MinecraftTextures::StorageKey)
            .to_owned(),
    ] {
        manager.create_index(index).await?;
    }

    Ok(())
}

async fn create_minecraft_profile_textures(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MinecraftProfileTextures::Table)
                .if_not_exists()
                .col(big_integer_pk(MinecraftProfileTextures::Id))
                .col(
                    ColumnDef::new(MinecraftProfileTextures::ProfileId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftProfileTextures::TextureId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftProfileTextures::TextureType)
                        .string_len(16)
                        .not_null(),
                )
                .col(utc_timestamp(manager, MinecraftProfileTextures::CreatedAt).not_null())
                .col(utc_timestamp(manager, MinecraftProfileTextures::UpdatedAt).not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_minecraft_profile_textures_profile")
                        .from(
                            MinecraftProfileTextures::Table,
                            MinecraftProfileTextures::ProfileId,
                        )
                        .to(MinecraftProfiles::Table, MinecraftProfiles::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_minecraft_profile_textures_texture")
                        .from(
                            MinecraftProfileTextures::Table,
                            MinecraftProfileTextures::TextureId,
                        )
                        .to(MinecraftTextures::Table, MinecraftTextures::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .index(
                    Index::create()
                        .name("idx_minecraft_profile_textures_profile_type_unique")
                        .col(MinecraftProfileTextures::ProfileId)
                        .col(MinecraftProfileTextures::TextureType)
                        .unique(),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_minecraft_profile_textures_texture")
                .table(MinecraftProfileTextures::Table)
                .col(MinecraftProfileTextures::TextureId)
                .to_owned(),
        )
        .await
}

fn big_integer_pk<T: IntoIden>(column: T) -> ColumnDef {
    let mut column = ColumnDef::new(column);
    column
        .big_integer()
        .not_null()
        .auto_increment()
        .primary_key();
    column
}

fn utc_timestamp<T: IntoIden>(manager: &SchemaManager<'_>, column: T) -> ColumnDef {
    crate::time::utc_date_time_column(manager, column)
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum MinecraftProfiles {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum MinecraftTextures {
    Table,
    Id,
    UserId,
    TextureType,
    Hash,
    StorageKey,
    MimeType,
    FileSize,
    Width,
    Height,
    TextureModel,
    Visibility,
    IsWardrobeItem,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MinecraftProfileTextures {
    Table,
    Id,
    ProfileId,
    TextureId,
    TextureType,
    CreatedAt,
    UpdatedAt,
}
