//! Add composite indexes for cursor pagination and user-scoped sorted lists.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for index in [
            Index::create()
                .name("idx_auth_sessions_user_last_seen_id")
                .table(AuthSessions::Table)
                .col(AuthSessions::UserId)
                .col(AuthSessions::LastSeenAt)
                .col(AuthSessions::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_users_created_id")
                .table(Users::Table)
                .col(Users::CreatedAt)
                .col(Users::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_users_role_created_id")
                .table(Users::Table)
                .col(Users::Role)
                .col(Users::CreatedAt)
                .col(Users::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_users_status_created_id")
                .table(Users::Table)
                .col(Users::Status)
                .col(Users::CreatedAt)
                .col(Users::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_audit_logs_created_id")
                .table(AuditLogs::Table)
                .col(AuditLogs::CreatedAt)
                .col(AuditLogs::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_audit_logs_user_created_id")
                .table(AuditLogs::Table)
                .col(AuditLogs::UserId)
                .col(AuditLogs::CreatedAt)
                .col(AuditLogs::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_audit_logs_action_created_id")
                .table(AuditLogs::Table)
                .col(AuditLogs::Action)
                .col(AuditLogs::CreatedAt)
                .col(AuditLogs::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_audit_logs_entity_type_created_id")
                .table(AuditLogs::Table)
                .col(AuditLogs::EntityType)
                .col(AuditLogs::CreatedAt)
                .col(AuditLogs::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_passkeys_user_last_used_created_id")
                .table(Passkeys::Table)
                .col(Passkeys::UserId)
                .col(Passkeys::LastUsedAt)
                .col(Passkeys::CreatedAt)
                .col(Passkeys::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_passkeys_user_created_id")
                .table(Passkeys::Table)
                .col(Passkeys::UserId)
                .col(Passkeys::CreatedAt)
                .col(Passkeys::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_external_auth_identities_user_login_created_id")
                .table(ExternalAuthIdentities::Table)
                .col(ExternalAuthIdentities::UserId)
                .col(ExternalAuthIdentities::LastLoginAt)
                .col(ExternalAuthIdentities::CreatedAt)
                .col(ExternalAuthIdentities::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_external_auth_identities_user_created_id")
                .table(ExternalAuthIdentities::Table)
                .col(ExternalAuthIdentities::UserId)
                .col(ExternalAuthIdentities::CreatedAt)
                .col(ExternalAuthIdentities::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_user_invitations_created_id")
                .table(UserInvitations::Table)
                .col(UserInvitations::CreatedAt)
                .col(UserInvitations::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_background_tasks_updated_id")
                .table(BackgroundTasks::Table)
                .col(BackgroundTasks::UpdatedAt)
                .col(BackgroundTasks::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_background_tasks_status_updated_id")
                .table(BackgroundTasks::Table)
                .col(BackgroundTasks::Status)
                .col(BackgroundTasks::UpdatedAt)
                .col(BackgroundTasks::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_minecraft_profiles_user_id_id")
                .table(MinecraftProfiles::Table)
                .col(MinecraftProfiles::UserId)
                .col(MinecraftProfiles::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_minecraft_textures_user_wardrobe_updated")
                .table(MinecraftTextures::Table)
                .col(MinecraftTextures::UserId)
                .col(MinecraftTextures::IsWardrobeItem)
                .col(MinecraftTextures::UpdatedAt)
                .col(MinecraftTextures::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_minecraft_textures_wardrobe_updated")
                .table(MinecraftTextures::Table)
                .col(MinecraftTextures::IsWardrobeItem)
                .col(MinecraftTextures::UpdatedAt)
                .col(MinecraftTextures::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_minecraft_textures_public_library_updated")
                .table(MinecraftTextures::Table)
                .col(MinecraftTextures::IsWardrobeItem)
                .col(MinecraftTextures::Visibility)
                .col(MinecraftTextures::LibraryStatus)
                .col(MinecraftTextures::UpdatedAt)
                .col(MinecraftTextures::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_minecraft_texture_reports_status_created_id")
                .table(MinecraftTextureReports::Table)
                .col(MinecraftTextureReports::Status)
                .col(MinecraftTextureReports::CreatedAt)
                .col(MinecraftTextureReports::Id)
                .if_not_exists()
                .to_owned(),
            Index::create()
                .name("idx_minecraft_texture_reports_texture_created_id")
                .table(MinecraftTextureReports::Table)
                .col(MinecraftTextureReports::TextureId)
                .col(MinecraftTextureReports::CreatedAt)
                .col(MinecraftTextureReports::Id)
                .if_not_exists()
                .to_owned(),
        ] {
            manager.create_index(index).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (table, name) in [
            (
                MinecraftTextureReports::Table.into_iden(),
                "idx_minecraft_texture_reports_texture_created_id",
            ),
            (
                MinecraftTextureReports::Table.into_iden(),
                "idx_minecraft_texture_reports_status_created_id",
            ),
            (
                MinecraftTextures::Table.into_iden(),
                "idx_minecraft_textures_public_library_updated",
            ),
            (
                MinecraftTextures::Table.into_iden(),
                "idx_minecraft_textures_wardrobe_updated",
            ),
            (
                MinecraftTextures::Table.into_iden(),
                "idx_minecraft_textures_user_wardrobe_updated",
            ),
            (
                MinecraftProfiles::Table.into_iden(),
                "idx_minecraft_profiles_user_id_id",
            ),
            (
                ExternalAuthIdentities::Table.into_iden(),
                "idx_external_auth_identities_user_created_id",
            ),
            (
                ExternalAuthIdentities::Table.into_iden(),
                "idx_external_auth_identities_user_login_created_id",
            ),
            (
                BackgroundTasks::Table.into_iden(),
                "idx_background_tasks_status_updated_id",
            ),
            (
                BackgroundTasks::Table.into_iden(),
                "idx_background_tasks_updated_id",
            ),
            (
                UserInvitations::Table.into_iden(),
                "idx_user_invitations_created_id",
            ),
            (Passkeys::Table.into_iden(), "idx_passkeys_user_created_id"),
            (
                Passkeys::Table.into_iden(),
                "idx_passkeys_user_last_used_created_id",
            ),
            (
                AuthSessions::Table.into_iden(),
                "idx_auth_sessions_user_last_seen_id",
            ),
            (
                AuditLogs::Table.into_iden(),
                "idx_audit_logs_entity_type_created_id",
            ),
            (
                AuditLogs::Table.into_iden(),
                "idx_audit_logs_action_created_id",
            ),
            (
                AuditLogs::Table.into_iden(),
                "idx_audit_logs_user_created_id",
            ),
            (AuditLogs::Table.into_iden(), "idx_audit_logs_created_id"),
            (Users::Table.into_iden(), "idx_users_status_created_id"),
            (Users::Table.into_iden(), "idx_users_role_created_id"),
            (Users::Table.into_iden(), "idx_users_created_id"),
        ] {
            manager
                .drop_index(Index::drop().name(name).table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}

#[derive(DeriveIden)]
enum AuthSessions {
    Table,
    Id,
    UserId,
    LastSeenAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Role,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AuditLogs {
    Table,
    Id,
    UserId,
    Action,
    EntityType,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Passkeys {
    Table,
    Id,
    UserId,
    LastUsedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserInvitations {
    Table,
    Id,
    CreatedAt,
}

#[derive(DeriveIden)]
enum BackgroundTasks {
    Table,
    Id,
    Status,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ExternalAuthIdentities {
    Table,
    Id,
    UserId,
    LastLoginAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum MinecraftProfiles {
    Table,
    Id,
    UserId,
}

#[derive(DeriveIden)]
enum MinecraftTextures {
    Table,
    Id,
    UserId,
    Visibility,
    IsWardrobeItem,
    LibraryStatus,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MinecraftTextureReports {
    Table,
    Id,
    TextureId,
    Status,
    CreatedAt,
}
