//! Add audit log indexes for admin overview activity aggregation.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_logs_action_created_user")
                    .table(AuditLogs::Table)
                    .col(AuditLogs::Action)
                    .col(AuditLogs::CreatedAt)
                    .col(AuditLogs::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_audit_logs_action_created_user")
                    .table(AuditLogs::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum AuditLogs {
    Table,
    Action,
    CreatedAt,
    UserId,
}
