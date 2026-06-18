//! Add local email activation and user invitation storage.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DbBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_users_pending_email(manager).await?;
        create_contact_verification_tokens(manager).await?;
        create_contact_verification_indexes(manager).await?;
        create_contact_verification_single_active_index(manager).await?;
        create_user_invitations(manager).await?;
        create_user_invitation_indexes(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(UserInvitations::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(ContactVerificationTokens::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_pending_email")
                    .table(Users::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::PendingEmail)
                    .to_owned(),
            )
            .await
    }
}

async fn add_users_pending_email(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(Users::Table)
                .add_column(ColumnDef::new(Users::PendingEmail).string_len(255).null())
                .to_owned(),
        )
        .await?;
    manager
        .create_index(
            Index::create()
                .name("idx_users_pending_email")
                .table(Users::Table)
                .col(Users::PendingEmail)
                .unique()
                .if_not_exists()
                .to_owned(),
        )
        .await
}

async fn create_contact_verification_tokens(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ContactVerificationTokens::Table)
                .if_not_exists()
                .col(big_integer_pk(ContactVerificationTokens::Id))
                .col(
                    ColumnDef::new(ContactVerificationTokens::UserId)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ContactVerificationTokens::Channel)
                        .string_len(16)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ContactVerificationTokens::Purpose)
                        .string_len(32)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ContactVerificationTokens::Target)
                        .string_len(255)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(ContactVerificationTokens::TokenHash)
                        .string_len(64)
                        .not_null()
                        .unique_key(),
                )
                .col(utc_timestamp(manager, ContactVerificationTokens::ExpiresAt).not_null())
                .col(utc_timestamp(manager, ContactVerificationTokens::ConsumedAt).null())
                .col(utc_timestamp(manager, ContactVerificationTokens::CreatedAt).not_null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_contact_verification_tokens_user")
                        .from(
                            ContactVerificationTokens::Table,
                            ContactVerificationTokens::UserId,
                        )
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_contact_verification_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for index in [
        Index::create()
            .name("idx_contact_verification_tokens_user_purpose")
            .table(ContactVerificationTokens::Table)
            .col(ContactVerificationTokens::UserId)
            .col(ContactVerificationTokens::Channel)
            .col(ContactVerificationTokens::Purpose)
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("idx_contact_verification_tokens_expires_at")
            .table(ContactVerificationTokens::Table)
            .col(ContactVerificationTokens::ExpiresAt)
            .if_not_exists()
            .to_owned(),
    ] {
        manager.create_index(index).await?;
    }
    Ok(())
}

async fn create_contact_verification_single_active_index(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    let statement = match manager.get_database_backend() {
        DbBackend::Sqlite | DbBackend::Postgres => {
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_contact_verification_tokens_single_active \
             ON contact_verification_tokens ( \
                user_id, \
                channel, \
                purpose, \
                (CASE WHEN consumed_at IS NULL THEN 1 ELSE NULL END) \
             );"
        }
        DbBackend::MySql => {
            "CREATE UNIQUE INDEX idx_contact_verification_tokens_single_active \
             ON contact_verification_tokens ( \
                user_id, \
                channel, \
                purpose, \
                ((CASE WHEN consumed_at IS NULL THEN 1 ELSE NULL END)) \
             );"
        }
        backend => {
            return Err(DbErr::Migration(format!(
                "unsupported database backend for contact verification active index: {backend:?}"
            )));
        }
    };

    manager
        .get_connection()
        .execute_unprepared(statement)
        .await?;
    Ok(())
}

async fn create_user_invitations(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(UserInvitations::Table)
                .if_not_exists()
                .col(big_integer_pk(UserInvitations::Id))
                .col(
                    ColumnDef::new(UserInvitations::Email)
                        .string_len(255)
                        .not_null(),
                )
                .col(
                    ColumnDef::new(UserInvitations::TokenHash)
                        .string_len(64)
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(UserInvitations::Status)
                        .string_len(16)
                        .not_null()
                        .default("pending"),
                )
                .col(
                    ColumnDef::new(UserInvitations::InvitedBy)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(UserInvitations::AcceptedUserId)
                        .big_integer()
                        .null(),
                )
                .col(utc_timestamp(manager, UserInvitations::ExpiresAt).not_null())
                .col(utc_timestamp(manager, UserInvitations::CreatedAt).not_null())
                .col(utc_timestamp(manager, UserInvitations::UpdatedAt).not_null())
                .col(utc_timestamp(manager, UserInvitations::AcceptedAt).null())
                .col(utc_timestamp(manager, UserInvitations::RevokedAt).null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_invitations_invited_by")
                        .from(UserInvitations::Table, UserInvitations::InvitedBy)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_user_invitations_accepted_user_id")
                        .from(UserInvitations::Table, UserInvitations::AcceptedUserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await
}

async fn create_user_invitation_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for index in [
        Index::create()
            .name("idx_user_invitations_email")
            .table(UserInvitations::Table)
            .col(UserInvitations::Email)
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("idx_user_invitations_status_expires_at")
            .table(UserInvitations::Table)
            .col(UserInvitations::Status)
            .col(UserInvitations::ExpiresAt)
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("idx_user_invitations_invited_by")
            .table(UserInvitations::Table)
            .col(UserInvitations::InvitedBy)
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("idx_user_invitations_accepted_user_id")
            .table(UserInvitations::Table)
            .col(UserInvitations::AcceptedUserId)
            .if_not_exists()
            .to_owned(),
    ] {
        manager.create_index(index).await?;
    }
    Ok(())
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
enum ContactVerificationTokens {
    Table,
    Id,
    UserId,
    Channel,
    Purpose,
    Target,
    TokenHash,
    ExpiresAt,
    ConsumedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserInvitations {
    Table,
    Id,
    Email,
    TokenHash,
    Status,
    InvitedBy,
    AcceptedUserId,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
    AcceptedAt,
    RevokedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    PendingEmail,
}
