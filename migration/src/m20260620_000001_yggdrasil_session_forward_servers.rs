//! Upstream Yggdrasil session server forwarding configuration.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(YggdrasilSessionForwardServers::Table)
                    .if_not_exists()
                    .col(big_integer_pk(YggdrasilSessionForwardServers::Id))
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::DisplayName)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::ProviderKind)
                            .string_len(16)
                            .not_null()
                            .default("remote"),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::EndpointKind)
                            .string_len(32)
                            .not_null()
                            .default("authlib_injector"),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::BaseUrl)
                            .string_len(512)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::Builtin)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::Priority)
                            .integer()
                            .not_null()
                            .default(100),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::Weight)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::TimeoutMs)
                            .integer()
                            .not_null()
                            .default(1500),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::TextureForwardEnabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        utc_timestamp(manager, YggdrasilSessionForwardServers::LastCheckedAt)
                            .null(),
                    )
                    .col(
                        utc_timestamp(manager, YggdrasilSessionForwardServers::LastSuccessAt)
                            .null(),
                    )
                    .col(
                        utc_timestamp(manager, YggdrasilSessionForwardServers::LastFailureAt)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(YggdrasilSessionForwardServers::LastFailureMessage)
                            .string_len(512)
                            .null(),
                    )
                    .col(
                        utc_timestamp(manager, YggdrasilSessionForwardServers::CreatedAt)
                            .not_null(),
                    )
                    .col(
                        utc_timestamp(manager, YggdrasilSessionForwardServers::UpdatedAt)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        for index in [
            Index::create()
                .name("idx_yggdrasil_session_forward_base_url")
                .table(YggdrasilSessionForwardServers::Table)
                .col(YggdrasilSessionForwardServers::BaseUrl)
                .unique()
                .to_owned(),
            Index::create()
                .name("idx_yggdrasil_session_forward_enabled_order")
                .table(YggdrasilSessionForwardServers::Table)
                .col(YggdrasilSessionForwardServers::Enabled)
                .col(YggdrasilSessionForwardServers::Priority)
                .col(YggdrasilSessionForwardServers::Id)
                .to_owned(),
        ] {
            manager.create_index(index).await?;
        }

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(YggdrasilSessionForwardServers::Table)
                    .columns([
                        YggdrasilSessionForwardServers::DisplayName,
                        YggdrasilSessionForwardServers::ProviderKind,
                        YggdrasilSessionForwardServers::EndpointKind,
                        YggdrasilSessionForwardServers::BaseUrl,
                        YggdrasilSessionForwardServers::Builtin,
                        YggdrasilSessionForwardServers::Enabled,
                        YggdrasilSessionForwardServers::Priority,
                        YggdrasilSessionForwardServers::Weight,
                        YggdrasilSessionForwardServers::TimeoutMs,
                        YggdrasilSessionForwardServers::TextureForwardEnabled,
                        YggdrasilSessionForwardServers::CreatedAt,
                        YggdrasilSessionForwardServers::UpdatedAt,
                    ])
                    .values_panic([
                        Expr::val("AsterYggdrasil"),
                        Expr::val("local"),
                        Expr::val("authlib_injector"),
                        Expr::val(Option::<String>::None),
                        Expr::val(true),
                        Expr::val(true),
                        Expr::val(100),
                        Expr::val(1),
                        Expr::val(1500),
                        Expr::val(false),
                        Expr::current_timestamp(),
                        Expr::current_timestamp(),
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .exec_stmt(
                Query::insert()
                    .into_table(YggdrasilSessionForwardServers::Table)
                    .columns([
                        YggdrasilSessionForwardServers::DisplayName,
                        YggdrasilSessionForwardServers::ProviderKind,
                        YggdrasilSessionForwardServers::EndpointKind,
                        YggdrasilSessionForwardServers::BaseUrl,
                        YggdrasilSessionForwardServers::Builtin,
                        YggdrasilSessionForwardServers::Enabled,
                        YggdrasilSessionForwardServers::Priority,
                        YggdrasilSessionForwardServers::Weight,
                        YggdrasilSessionForwardServers::TimeoutMs,
                        YggdrasilSessionForwardServers::TextureForwardEnabled,
                        YggdrasilSessionForwardServers::CreatedAt,
                        YggdrasilSessionForwardServers::UpdatedAt,
                    ])
                    .values_panic([
                        Expr::val("Mojang"),
                        Expr::val("remote"),
                        Expr::val("mojang_session"),
                        Expr::val("https://sessionserver.mojang.com"),
                        Expr::val(true),
                        Expr::val(false),
                        Expr::val(200),
                        Expr::val(1),
                        Expr::val(1500),
                        Expr::val(false),
                        Expr::current_timestamp(),
                        Expr::current_timestamp(),
                    ])
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(YggdrasilSessionForwardServers::Table)
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
enum YggdrasilSessionForwardServers {
    Table,
    Id,
    DisplayName,
    ProviderKind,
    EndpointKind,
    BaseUrl,
    Builtin,
    Enabled,
    Priority,
    Weight,
    TimeoutMs,
    TextureForwardEnabled,
    LastCheckedAt,
    LastSuccessAt,
    LastFailureAt,
    LastFailureMessage,
    CreatedAt,
    UpdatedAt,
}
