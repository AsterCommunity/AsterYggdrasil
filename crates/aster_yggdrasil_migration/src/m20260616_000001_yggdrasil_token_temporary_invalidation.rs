//! Add temporary invalidation state for Yggdrasil tokens.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(YggdrasilTokens::Table)
                    .add_column(
                        crate::time::utc_date_time_column(
                            manager,
                            YggdrasilTokens::TemporarilyInvalidatedAt,
                        )
                        .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(YggdrasilTokens::Table)
                    .drop_column(YggdrasilTokens::TemporarilyInvalidatedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum YggdrasilTokens {
    Table,
    TemporarilyInvalidatedAt,
}
