//! Minecraft profile and Yggdrasil token schema.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_minecraft_profiles(manager).await?;
        create_yggdrasil_tokens(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            YggdrasilTokens::Table.into_iden(),
            MinecraftProfiles::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).if_exists().to_owned())
                .await?;
        }
        Ok(())
    }
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

async fn create_minecraft_profiles(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MinecraftProfiles::Table)
                .if_not_exists()
                .col(big_integer_pk(MinecraftProfiles::Id))
                .col(
                    ColumnDef::new(MinecraftProfiles::UserId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftProfiles::Uuid)
                        .string_len(32)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftProfiles::Name)
                        .string_len(16)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MinecraftProfiles::TextureModel)
                        .string_len(16)
                        .not_null()
                        .default("default"),
                )
                .col(
                    ColumnDef::new(MinecraftProfiles::UploadableTextures)
                        .string_len(64)
                        .not_null()
                        .default("skin,cape"),
                )
                .col(utc_timestamp(manager, MinecraftProfiles::CreatedAt).not_null())
                .col(utc_timestamp(manager, MinecraftProfiles::UpdatedAt).not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_minecraft_profiles_user")
                        .from(MinecraftProfiles::Table, MinecraftProfiles::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .index(
                    Index::create()
                        .name("idx_minecraft_profiles_uuid_unique")
                        .col(MinecraftProfiles::Uuid)
                        .unique(),
                )
                .index(
                    Index::create()
                        .name("idx_minecraft_profiles_name_unique")
                        .col(MinecraftProfiles::Name)
                        .unique(),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_index(
            Index::create()
                .name("idx_minecraft_profiles_user_id")
                .table(MinecraftProfiles::Table)
                .col(MinecraftProfiles::UserId)
                .to_owned(),
        )
        .await
}

async fn create_yggdrasil_tokens(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(YggdrasilTokens::Table)
                .if_not_exists()
                .col(big_integer_pk(YggdrasilTokens::Id))
                .col(
                    ColumnDef::new(YggdrasilTokens::UserId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(YggdrasilTokens::AccessTokenHash)
                        .string_len(128)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(YggdrasilTokens::ClientToken)
                        .string_len(255)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(YggdrasilTokens::SelectedProfileId)
                        .big_integer()
                        .null(),
                )
                .col(utc_timestamp(manager, YggdrasilTokens::IssuedAt).not_null())
                .col(utc_timestamp(manager, YggdrasilTokens::ExpiresAt).not_null())
                .col(utc_timestamp(manager, YggdrasilTokens::RevokedAt).null())
                .col(
                    ColumnDef::new(YggdrasilTokens::UserAgent)
                        .string_len(512)
                        .null(),
                )
                .col(
                    ColumnDef::new(YggdrasilTokens::IpAddress)
                        .string_len(128)
                        .null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_yggdrasil_tokens_user")
                        .from(YggdrasilTokens::Table, YggdrasilTokens::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_yggdrasil_tokens_selected_profile")
                        .from(YggdrasilTokens::Table, YggdrasilTokens::SelectedProfileId)
                        .to(MinecraftProfiles::Table, MinecraftProfiles::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .index(
                    Index::create()
                        .name("idx_yggdrasil_tokens_access_hash_unique")
                        .col(YggdrasilTokens::AccessTokenHash)
                        .unique(),
                )
                .to_owned(),
        )
        .await?;

    for index in [
        Index::create()
            .name("idx_yggdrasil_tokens_user_issued")
            .table(YggdrasilTokens::Table)
            .col(YggdrasilTokens::UserId)
            .col(YggdrasilTokens::IssuedAt)
            .to_owned(),
        Index::create()
            .name("idx_yggdrasil_tokens_expires_at")
            .table(YggdrasilTokens::Table)
            .col(YggdrasilTokens::ExpiresAt)
            .to_owned(),
    ] {
        manager.create_index(index).await?;
    }

    Ok(())
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
    UserId,
    Uuid,
    Name,
    TextureModel,
    UploadableTextures,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum YggdrasilTokens {
    Table,
    Id,
    UserId,
    AccessTokenHash,
    ClientToken,
    SelectedProfileId,
    IssuedAt,
    ExpiresAt,
    RevokedAt,
    UserAgent,
    IpAddress,
}
