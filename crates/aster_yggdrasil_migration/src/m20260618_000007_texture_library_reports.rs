//! Public texture library user reports.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MinecraftTextureReports::Table)
                    .if_not_exists()
                    .col(big_integer_pk(MinecraftTextureReports::Id))
                    .col(
                        ColumnDef::new(MinecraftTextureReports::TextureId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MinecraftTextureReports::ReporterUserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MinecraftTextureReports::Reason)
                            .string_len(24)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(MinecraftTextureReports::Message)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(MinecraftTextureReports::Status)
                            .string_len(24)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(MinecraftTextureReports::AdminNote)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(MinecraftTextureReports::HandledByUserId)
                            .big_integer()
                            .null(),
                    )
                    .col(utc_timestamp(manager, MinecraftTextureReports::HandledAt).null())
                    .col(utc_timestamp(manager, MinecraftTextureReports::CreatedAt).not_null())
                    .col(utc_timestamp(manager, MinecraftTextureReports::UpdatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_texture_reports_texture")
                            .from(
                                MinecraftTextureReports::Table,
                                MinecraftTextureReports::TextureId,
                            )
                            .to(MinecraftTextures::Table, MinecraftTextures::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_texture_reports_reporter")
                            .from(
                                MinecraftTextureReports::Table,
                                MinecraftTextureReports::ReporterUserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_texture_reports_handler")
                            .from(
                                MinecraftTextureReports::Table,
                                MinecraftTextureReports::HandledByUserId,
                            )
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        for index in [
            Index::create()
                .name("idx_texture_reports_status_created")
                .table(MinecraftTextureReports::Table)
                .col(MinecraftTextureReports::Status)
                .col(MinecraftTextureReports::CreatedAt)
                .col(MinecraftTextureReports::Id)
                .to_owned(),
            Index::create()
                .name("idx_texture_reports_texture_status")
                .table(MinecraftTextureReports::Table)
                .col(MinecraftTextureReports::TextureId)
                .col(MinecraftTextureReports::Status)
                .to_owned(),
            Index::create()
                .name("idx_texture_reports_reporter_status")
                .table(MinecraftTextureReports::Table)
                .col(MinecraftTextureReports::ReporterUserId)
                .col(MinecraftTextureReports::Status)
                .to_owned(),
        ] {
            manager.create_index(index).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MinecraftTextureReports::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

fn utc_timestamp<T: IntoIden>(manager: &SchemaManager<'_>, column: T) -> ColumnDef {
    crate::time::utc_date_time_column(manager, column)
}

fn big_integer_pk<T: IntoIden>(column: T) -> ColumnDef {
    ColumnDef::new(column)
        .big_integer()
        .not_null()
        .auto_increment()
        .primary_key()
        .to_owned()
}

#[derive(DeriveIden)]
enum MinecraftTextureReports {
    Table,
    Id,
    TextureId,
    ReporterUserId,
    Reason,
    Message,
    Status,
    AdminNote,
    HandledByUserId,
    HandledAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MinecraftTextures {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
