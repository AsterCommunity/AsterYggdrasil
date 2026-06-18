//! Operator role and scope bindings.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserOperatorScopes::Table)
                    .if_not_exists()
                    .col(big_integer_pk(UserOperatorScopes::Id))
                    .col(
                        ColumnDef::new(UserOperatorScopes::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserOperatorScopes::Scope)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(utc_timestamp(manager, UserOperatorScopes::CreatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_operator_scopes_user")
                            .from(UserOperatorScopes::Table, UserOperatorScopes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx_user_operator_scopes_unique")
                            .col(UserOperatorScopes::UserId)
                            .col(UserOperatorScopes::Scope)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_user_operator_scopes_scope")
                    .table(UserOperatorScopes::Table)
                    .col(UserOperatorScopes::Scope)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(UserOperatorScopes::Table)
                    .if_exists()
                    .to_owned(),
            )
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
enum UserOperatorScopes {
    Table,
    Id,
    UserId,
    Scope,
    CreatedAt,
}
