//! AsterYggdrasil database migrations.
#![deny(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::expect_used,
        clippy::panic
    )
)]

pub use sea_orm_migration::prelude::*;

mod m20260606_000001_foundation_schema;
mod m20260615_000001_yggdrasil_profiles;
mod m20260615_000002_minecraft_textures;
mod m20260615_000003_passkeys;
mod m20260616_000001_yggdrasil_token_temporary_invalidation;
mod m20260618_000001_auth_email_registration_and_invitations;
mod m20260618_000002_add_user_must_change_password;
mod m20260618_000003_audit_log_activity_indexes;
mod m20260618_000004_texture_library_metadata;
mod m20260618_000005_operator_scopes;
mod m20260618_000006_texture_library_review;
mod m20260618_000007_texture_library_reports;
mod m20260619_000001_cursor_pagination_indexes;
mod m20260620_000001_yggdrasil_session_forward_servers;
mod time;

pub struct Migrator;

impl Migrator {
    pub async fn up(
        db: &sea_orm_migration::sea_orm::DatabaseConnection,
        steps: Option<u32>,
    ) -> Result<(), DbErr> {
        <Self as MigratorTrait>::up(db, steps).await
    }
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260606_000001_foundation_schema::Migration),
            Box::new(m20260615_000001_yggdrasil_profiles::Migration),
            Box::new(m20260615_000002_minecraft_textures::Migration),
            Box::new(m20260615_000003_passkeys::Migration),
            Box::new(m20260616_000001_yggdrasil_token_temporary_invalidation::Migration),
            Box::new(m20260618_000001_auth_email_registration_and_invitations::Migration),
            Box::new(m20260618_000002_add_user_must_change_password::Migration),
            Box::new(m20260618_000003_audit_log_activity_indexes::Migration),
            Box::new(m20260618_000004_texture_library_metadata::Migration),
            Box::new(m20260618_000005_operator_scopes::Migration),
            Box::new(m20260618_000006_texture_library_review::Migration),
            Box::new(m20260618_000007_texture_library_reports::Migration),
            Box::new(m20260619_000001_cursor_pagination_indexes::Migration),
            Box::new(m20260620_000001_yggdrasil_session_forward_servers::Migration),
        ]
    }
}
