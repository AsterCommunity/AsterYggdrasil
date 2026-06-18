//! Texture library metadata, administrator-managed tags, and tag bindings.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_texture_library_columns(manager).await?;
        create_texture_library_tags(manager).await?;
        create_texture_tag_bindings(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MinecraftTextureTagBindings::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MinecraftTextureTags::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MinecraftTextures::Table)
                    .drop_column(MinecraftTextures::LibraryStatus)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(MinecraftTextures::Table)
                    .drop_column(MinecraftTextures::DisplayName)
                    .to_owned(),
            )
            .await
    }
}

async fn add_texture_library_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(MinecraftTextures::Table)
                .add_column(
                    ColumnDef::new(MinecraftTextures::DisplayName)
                        .string_len(96)
                        .null(),
                )
                .to_owned(),
        )
        .await?;
    manager
        .alter_table(
            Table::alter()
                .table(MinecraftTextures::Table)
                .add_column(
                    ColumnDef::new(MinecraftTextures::LibraryStatus)
                        .string_len(24)
                        .not_null()
                        .default("private"),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_minecraft_textures_library_status")
                .table(MinecraftTextures::Table)
                .col(MinecraftTextures::LibraryStatus)
                .to_owned(),
        )
        .await
}

async fn create_texture_library_tags(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MinecraftTextureTags::Table)
                .if_not_exists()
                .col(big_integer_pk(MinecraftTextureTags::Id))
                .col(
                    ColumnDef::new(MinecraftTextureTags::Name)
                        .string_len(64)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextureTags::NormalizedName)
                        .string_len(64)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextureTags::Color)
                        .string_len(16)
                        .not_null()
                        .default("#64748b"),
                )
                .col(
                    ColumnDef::new(MinecraftTextureTags::SortOrder)
                        .integer()
                        .not_null()
                        .default(0),
                )
                .col(utc_timestamp(manager, MinecraftTextureTags::CreatedAt).not_null())
                .col(utc_timestamp(manager, MinecraftTextureTags::UpdatedAt).not_null())
                .to_owned(),
        )
        .await?;

    for index in [
        Index::create()
            .name("idx_minecraft_texture_tags_name_unique")
            .table(MinecraftTextureTags::Table)
            .col(MinecraftTextureTags::NormalizedName)
            .unique()
            .to_owned(),
        Index::create()
            .name("idx_minecraft_texture_tags_sort")
            .table(MinecraftTextureTags::Table)
            .col(MinecraftTextureTags::SortOrder)
            .col(MinecraftTextureTags::Name)
            .to_owned(),
    ] {
        manager.create_index(index).await?;
    }

    Ok(())
}

async fn create_texture_tag_bindings(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MinecraftTextureTagBindings::Table)
                .if_not_exists()
                .col(big_integer_pk(MinecraftTextureTagBindings::Id))
                .col(
                    ColumnDef::new(MinecraftTextureTagBindings::TextureId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftTextureTagBindings::TagId)
                        .big_integer()
                        .not_null(),
                )
                .col(utc_timestamp(manager, MinecraftTextureTagBindings::CreatedAt).not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_minecraft_texture_tag_bindings_texture")
                        .from(
                            MinecraftTextureTagBindings::Table,
                            MinecraftTextureTagBindings::TextureId,
                        )
                        .to(MinecraftTextures::Table, MinecraftTextures::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_minecraft_texture_tag_bindings_tag")
                        .from(
                            MinecraftTextureTagBindings::Table,
                            MinecraftTextureTagBindings::TagId,
                        )
                        .to(MinecraftTextureTags::Table, MinecraftTextureTags::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .index(
                    Index::create()
                        .name("idx_minecraft_texture_tag_bindings_unique")
                        .col(MinecraftTextureTagBindings::TextureId)
                        .col(MinecraftTextureTagBindings::TagId)
                        .unique(),
                )
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_minecraft_texture_tag_bindings_tag")
                .table(MinecraftTextureTagBindings::Table)
                .col(MinecraftTextureTagBindings::TagId)
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
enum MinecraftTextures {
    Table,
    Id,
    DisplayName,
    LibraryStatus,
}

#[derive(DeriveIden)]
enum MinecraftTextureTags {
    Table,
    Id,
    Name,
    NormalizedName,
    Color,
    SortOrder,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MinecraftTextureTagBindings {
    Table,
    Id,
    TextureId,
    TagId,
    CreatedAt,
}
