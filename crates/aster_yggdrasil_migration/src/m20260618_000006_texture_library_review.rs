//! Public texture library review metadata.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in [
            utc_timestamp(manager, MinecraftTextures::LibrarySubmittedAt)
                .null()
                .to_owned(),
            utc_timestamp(manager, MinecraftTextures::LibraryReviewedAt)
                .null()
                .to_owned(),
            ColumnDef::new(MinecraftTextures::LibraryReviewerUserId)
                .big_integer()
                .null()
                .to_owned(),
            ColumnDef::new(MinecraftTextures::LibraryReviewNote)
                .text()
                .null()
                .to_owned(),
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(MinecraftTextures::Table)
                        .add_column(column)
                        .to_owned(),
                )
                .await?;
        }

        for index in [
            Index::create()
                .name("idx_minecraft_textures_library_public")
                .table(MinecraftTextures::Table)
                .col(MinecraftTextures::Visibility)
                .col(MinecraftTextures::LibraryStatus)
                .col(MinecraftTextures::UpdatedAt)
                .col(MinecraftTextures::Id)
                .to_owned(),
            Index::create()
                .name("idx_minecraft_textures_library_review_queue")
                .table(MinecraftTextures::Table)
                .col(MinecraftTextures::LibraryStatus)
                .col(MinecraftTextures::LibrarySubmittedAt)
                .col(MinecraftTextures::Id)
                .to_owned(),
            Index::create()
                .name("idx_minecraft_textures_user_library_status")
                .table(MinecraftTextures::Table)
                .col(MinecraftTextures::UserId)
                .col(MinecraftTextures::LibraryStatus)
                .col(MinecraftTextures::UpdatedAt)
                .col(MinecraftTextures::Id)
                .to_owned(),
        ] {
            manager.create_index(index).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for index in [
            "idx_minecraft_textures_user_library_status",
            "idx_minecraft_textures_library_review_queue",
            "idx_minecraft_textures_library_public",
        ] {
            manager
                .drop_index(
                    Index::drop()
                        .name(index)
                        .table(MinecraftTextures::Table)
                        .to_owned(),
                )
                .await?;
        }

        for column in [
            MinecraftTextures::LibraryReviewNote,
            MinecraftTextures::LibraryReviewerUserId,
            MinecraftTextures::LibraryReviewedAt,
            MinecraftTextures::LibrarySubmittedAt,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(MinecraftTextures::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

fn utc_timestamp<T: IntoIden>(manager: &SchemaManager<'_>, column: T) -> ColumnDef {
    crate::time::utc_date_time_column(manager, column)
}

#[derive(DeriveIden)]
enum MinecraftTextures {
    Table,
    Id,
    UserId,
    Visibility,
    LibraryStatus,
    LibrarySubmittedAt,
    LibraryReviewedAt,
    LibraryReviewerUserId,
    LibraryReviewNote,
    UpdatedAt,
}
