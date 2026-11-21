//! Passkey / WebAuthn credential schema.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Passkeys::Table)
                    .if_not_exists()
                    .col(big_integer_pk(Passkeys::Id))
                    .col(ColumnDef::new(Passkeys::UserId).big_integer().not_null())
                    .col(
                        ColumnDef::new(Passkeys::CredentialId)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Passkeys::UserHandle)
                            .string_len(36)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Passkeys::Credential).text().not_null())
                    .col(ColumnDef::new(Passkeys::Name).string_len(128).not_null())
                    .col(ColumnDef::new(Passkeys::Transports).text().null())
                    .col(
                        ColumnDef::new(Passkeys::BackupEligible)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Passkeys::BackedUp)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Passkeys::SignCount)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(utc_timestamp(manager, Passkeys::CreatedAt).not_null())
                    .col(utc_timestamp(manager, Passkeys::UpdatedAt).not_null())
                    .col(utc_timestamp(manager, Passkeys::LastUsedAt).null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_passkeys_user")
                            .from(Passkeys::Table, Passkeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        for index in [
            Index::create()
                .name("idx_passkeys_user_id")
                .table(Passkeys::Table)
                .col(Passkeys::UserId)
                .to_owned(),
            Index::create()
                .name("idx_passkeys_credential_id")
                .table(Passkeys::Table)
                .col(Passkeys::CredentialId)
                .unique()
                .to_owned(),
            Index::create()
                .name("idx_passkeys_user_handle_credential")
                .table(Passkeys::Table)
                .col(Passkeys::UserHandle)
                .col(Passkeys::CredentialId)
                .to_owned(),
        ] {
            manager.create_index(index).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Passkeys::Table).if_exists().to_owned())
            .await
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

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Passkeys {
    Table,
    Id,
    UserId,
    CredentialId,
    UserHandle,
    Credential,
    Name,
    Transports,
    BackupEligible,
    BackedUp,
    SignCount,
    CreatedAt,
    UpdatedAt,
    LastUsedAt,
}
