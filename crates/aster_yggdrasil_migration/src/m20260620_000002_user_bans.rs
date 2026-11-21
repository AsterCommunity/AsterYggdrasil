//! User capability bans and append-only ban events.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_user_bans(manager).await?;
        create_user_ban_events(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            UserBanEvents::Table.into_iden(),
            UserBans::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).if_exists().to_owned())
                .await?;
        }
        Ok(())
    }
}

async fn create_user_bans(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(UserBans::Table)
                .if_not_exists()
                .col(big_integer_pk(UserBans::Id))
                .col(ColumnDef::new(UserBans::UserId).big_integer().not_null())
                .col(ColumnDef::new(UserBans::Scopes).text().not_null())
                .col(
                    ColumnDef::new(UserBans::Status)
                        .string_len(24)
                        .not_null()
                        .default("active"),
                )
                .col(ColumnDef::new(UserBans::Reason).string_len(128).not_null())
                .col(ColumnDef::new(UserBans::PublicReason).text().null())
                .col(ColumnDef::new(UserBans::AdminNote).text().null())
                .col(
                    ColumnDef::new(UserBans::CreatedByUserId)
                        .big_integer()
                        .null(),
                )
                .col(utc_timestamp(manager, UserBans::StartsAt).not_null())
                .col(utc_timestamp(manager, UserBans::ExpiresAt).null())
                .col(utc_timestamp(manager, UserBans::RevokedAt).null())
                .col(
                    ColumnDef::new(UserBans::RevokedByUserId)
                        .big_integer()
                        .null(),
                )
                .col(ColumnDef::new(UserBans::RevokeNote).text().null())
                .col(utc_timestamp(manager, UserBans::CreatedAt).not_null())
                .col(utc_timestamp(manager, UserBans::UpdatedAt).not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_bans_user")
                        .from(UserBans::Table, UserBans::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_bans_created_by")
                        .from(UserBans::Table, UserBans::CreatedByUserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_bans_revoked_by")
                        .from(UserBans::Table, UserBans::RevokedByUserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;

    for index in [
        Index::create()
            .name("idx_user_bans_user_status")
            .table(UserBans::Table)
            .col(UserBans::UserId)
            .col(UserBans::Status)
            .to_owned(),
        Index::create()
            .name("idx_user_bans_status_created")
            .table(UserBans::Table)
            .col(UserBans::Status)
            .col(UserBans::CreatedAt)
            .col(UserBans::Id)
            .to_owned(),
        Index::create()
            .name("idx_user_bans_expires_at")
            .table(UserBans::Table)
            .col(UserBans::ExpiresAt)
            .to_owned(),
    ] {
        manager.create_index(index).await?;
    }

    Ok(())
}

async fn create_user_ban_events(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(UserBanEvents::Table)
                .if_not_exists()
                .col(big_integer_pk(UserBanEvents::Id))
                .col(
                    ColumnDef::new(UserBanEvents::BanId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(UserBanEvents::ActorUserId)
                        .big_integer()
                        .null(),
                )
                .col(
                    ColumnDef::new(UserBanEvents::EventType)
                        .string_len(32)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(UserBanEvents::PreviousStatus)
                        .string_len(24)
                        .null(),
                )
                .col(
                    ColumnDef::new(UserBanEvents::NextStatus)
                        .string_len(24)
                        .null(),
                )
                .col(ColumnDef::new(UserBanEvents::PreviousScopes).text().null())
                .col(ColumnDef::new(UserBanEvents::NextScopes).text().null())
                .col(utc_timestamp(manager, UserBanEvents::PreviousExpiresAt).null())
                .col(utc_timestamp(manager, UserBanEvents::NextExpiresAt).null())
                .col(ColumnDef::new(UserBanEvents::Note).text().null())
                .col(utc_timestamp(manager, UserBanEvents::CreatedAt).not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_ban_events_ban")
                        .from(UserBanEvents::Table, UserBanEvents::BanId)
                        .to(UserBans::Table, UserBans::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_ban_events_actor")
                        .from(UserBanEvents::Table, UserBanEvents::ActorUserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;

    for index in [
        Index::create()
            .name("idx_user_ban_events_ban_created")
            .table(UserBanEvents::Table)
            .col(UserBanEvents::BanId)
            .col(UserBanEvents::CreatedAt)
            .col(UserBanEvents::Id)
            .to_owned(),
        Index::create()
            .name("idx_user_ban_events_actor_created")
            .table(UserBanEvents::Table)
            .col(UserBanEvents::ActorUserId)
            .col(UserBanEvents::CreatedAt)
            .to_owned(),
    ] {
        manager.create_index(index).await?;
    }

    Ok(())
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
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum UserBans {
    Table,
    Id,
    UserId,
    Scopes,
    Status,
    Reason,
    PublicReason,
    AdminNote,
    CreatedByUserId,
    StartsAt,
    ExpiresAt,
    RevokedAt,
    RevokedByUserId,
    RevokeNote,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum UserBanEvents {
    Table,
    Id,
    BanId,
    ActorUserId,
    EventType,
    PreviousStatus,
    NextStatus,
    PreviousScopes,
    NextScopes,
    PreviousExpiresAt,
    NextExpiresAt,
    Note,
    CreatedAt,
}
