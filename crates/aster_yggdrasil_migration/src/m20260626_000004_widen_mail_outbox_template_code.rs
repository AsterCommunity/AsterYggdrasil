//! Widen mail outbox template codes to match Forge's shared schema.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_template_code_len(manager, 64).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        alter_template_code_len(manager, 32).await
    }
}

async fn alter_template_code_len(manager: &SchemaManager<'_>, len: u32) -> Result<(), DbErr> {
    match manager.get_database_backend() {
        DatabaseBackend::Sqlite => Ok(()),
        DatabaseBackend::MySql | DatabaseBackend::Postgres => {
            manager
                .alter_table(
                    Table::alter()
                        .table(MailOutbox::Table)
                        .modify_column(
                            ColumnDef::new(MailOutbox::TemplateCode)
                                .string_len(len)
                                .not_null(),
                        )
                        .to_owned(),
                )
                .await
        }
        backend => Err(DbErr::Migration(format!(
            "unsupported database backend for mail outbox template code widening: {backend:?}"
        ))),
    }
}

#[derive(DeriveIden)]
enum MailOutbox {
    Table,
    TemplateCode,
}
