//! Administrator user management service.

use crate::api::pagination::{AdminUserSortBy, OffsetPage, SortOrder};
use crate::db::repository::{
    minecraft_profile_repo, user_operator_scope_repo, user_repo, yggdrasil_token_repo,
};
use crate::entities::user;
use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, ObjectStorageRuntimeState, RuntimeConfigRuntimeState};
use crate::services::profile_service::{self, AvatarAudience, AvatarInfo, UserProfileInfo};
use crate::services::{auth_service, texture_service, yggdrasil_service};
use crate::types::{OperatorScope, UserRole, UserStatus};
use crate::utils::email::normalize_email;
use crate::utils::hash::hash_password;
use crate::utils::numbers::u64_to_usize;
use rand::RngExt;
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

const SUPER_ADMIN_USER_ID: i64 = 1;
const GENERATED_PASSWORD_LENGTH: usize = 24;
const GENERATED_PASSWORD_UPPERCASE: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
const GENERATED_PASSWORD_LOWERCASE: &[u8] = b"abcdefghijkmnopqrstuvwxyz";
const GENERATED_PASSWORD_DIGITS: &[u8] = b"23456789";
const GENERATED_PASSWORD_SYMBOLS: &[u8] = b"!@#$%^&*-_+=";
const GENERATED_PASSWORD_CHARSET: &[u8] =
    b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_+=";

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AdminUserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub pending_email: Option<String>,
    pub role: UserRole,
    pub operator_scopes: Vec<OperatorScope>,
    pub status: UserStatus,
    pub must_change_password: bool,
    pub session_version: i64,
    pub profile_count: u64,
    pub active_session_count: u64,
    pub profile: UserProfileInfo,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateAdminUserOutput {
    pub user: AdminUserInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_password: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct DeleteAdminUserOutput {
    pub user: AdminUserInfo,
    pub deleted_profile_count: usize,
    pub deleted_profile_texture_count: usize,
    pub deleted_wardrobe_texture_count: usize,
    pub revoked_session_count: u64,
    pub revoked_yggdrasil_token_count: u64,
}

#[derive(Debug, Clone)]
pub struct AdminUpdateUserInput {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<UserRole>,
    pub operator_scopes: Option<Vec<OperatorScope>>,
    pub status: Option<UserStatus>,
    pub must_change_password: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct AdminUserListFilters {
    pub keyword: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
}

fn default_user_profile_info() -> UserProfileInfo {
    UserProfileInfo {
        display_name: None,
        avatar: AvatarInfo {
            source: crate::types::AvatarSource::None,
            url_512: None,
            url_1024: None,
            version: 0,
        },
    }
}

fn generate_temporary_password() -> String {
    let mut rng = rand::rng();
    let mut bytes = Vec::with_capacity(GENERATED_PASSWORD_LENGTH);
    for charset in [
        GENERATED_PASSWORD_UPPERCASE,
        GENERATED_PASSWORD_LOWERCASE,
        GENERATED_PASSWORD_DIGITS,
        GENERATED_PASSWORD_SYMBOLS,
    ] {
        let index = rng.random_range(0..charset.len());
        bytes.push(charset[index]);
    }
    while bytes.len() < GENERATED_PASSWORD_LENGTH {
        let index = rng.random_range(0..GENERATED_PASSWORD_CHARSET.len());
        bytes.push(GENERATED_PASSWORD_CHARSET[index]);
    }
    for index in (1..bytes.len()).rev() {
        let swap_index = rng.random_range(0..=index);
        bytes.swap(index, swap_index);
    }
    bytes.into_iter().map(char::from).collect()
}

fn normalize_role_and_scopes(
    requested_role: UserRole,
    requested_scopes: Option<Vec<OperatorScope>>,
) -> (UserRole, Vec<OperatorScope>) {
    match requested_scopes {
        Some(scopes) => match requested_role {
            UserRole::Admin => (UserRole::Operator, scopes),
            UserRole::Operator => (UserRole::Operator, scopes),
            UserRole::User => (UserRole::User, Vec::new()),
        },
        None => match requested_role {
            UserRole::Admin => (UserRole::Admin, Vec::new()),
            UserRole::User => (UserRole::User, Vec::new()),
            UserRole::Operator => (UserRole::Operator, Vec::new()),
        },
    }
}

fn normalize_update_role_and_scopes(
    existing_role: UserRole,
    requested_role: Option<UserRole>,
    requested_scopes: Option<Vec<OperatorScope>>,
    existing_scopes: Vec<OperatorScope>,
) -> (UserRole, Option<Vec<OperatorScope>>) {
    let base_role = requested_role.unwrap_or(existing_role);
    match requested_scopes {
        Some(scopes) => match base_role {
            UserRole::Admin => (UserRole::Operator, Some(scopes)),
            UserRole::Operator => (UserRole::Operator, Some(scopes)),
            UserRole::User => (UserRole::User, Some(Vec::new())),
        },
        None => match base_role {
            UserRole::Admin => (UserRole::Admin, Some(Vec::new())),
            UserRole::User => (UserRole::User, Some(Vec::new())),
            UserRole::Operator => {
                if requested_role == Some(UserRole::Operator) && existing_role != UserRole::Operator
                {
                    (UserRole::Operator, Some(Vec::new()))
                } else if existing_role == UserRole::Operator {
                    (UserRole::Operator, Some(existing_scopes))
                } else {
                    (UserRole::Operator, Some(Vec::new()))
                }
            }
        },
    }
}

fn validate_identity_input(username: &str, email: &str, password: &str) -> Result<String> {
    auth_service::validate_username(username)?;
    let email = normalize_email(email)?;
    auth_service::validate_password(password)?;
    Ok(email)
}

pub async fn list_users<S>(
    state: &S,
    limit: u64,
    offset: u64,
    filters: AdminUserListFilters,
    sort_by: AdminUserSortBy,
    sort_order: SortOrder,
) -> Result<OffsetPage<AdminUserInfo>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        limit,
        offset,
        has_keyword = filters.keyword.is_some(),
        has_role_filter = filters.role.is_some(),
        has_status_filter = filters.status.is_some(),
        sort_by = ?sort_by,
        sort_order = ?sort_order,
        "listing admin users"
    );
    let page = user_repo::list_admin_paginated(
        state.reader_db(),
        user_repo::AdminUserFilters {
            keyword: filters.keyword,
            role: filters.role,
            status: filters.status,
        },
        sort_by,
        sort_order,
        limit,
        offset,
    )
    .await?;
    let items = hydrate_users(state, page.items).await?;
    tracing::debug!(
        returned = items.len(),
        total = page.total,
        "listed admin users"
    );
    Ok(OffsetPage::new(items, page.total, page.limit, page.offset))
}

pub async fn get_user<S>(state: &S, id: i64) -> Result<AdminUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(user_id = id, "loading admin user");
    let user = user_repo::find_by_id(state.reader_db(), id).await?;
    let users = hydrate_users(state, vec![user]).await?;
    users
        .into_iter()
        .next()
        .ok_or_else(|| AsterError::internal_error("admin user hydration returned no item"))
}

pub async fn create_user<S>(
    state: &S,
    username: &str,
    email: &str,
    password: Option<&str>,
    role: UserRole,
    operator_scopes: Option<Vec<OperatorScope>>,
    status: UserStatus,
    must_change_password: Option<bool>,
) -> Result<CreateAdminUserOutput>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    tracing::debug!(
        username,
        role = ?role,
        status = ?status,
        "creating admin user"
    );
    let explicit_password = password.map(str::trim).filter(|value| !value.is_empty());
    let generated_password = explicit_password
        .is_none()
        .then(generate_temporary_password);
    let password = generated_password
        .as_deref()
        .or(explicit_password)
        .ok_or_else(|| AsterError::internal_error("temporary password generation failed"))?;
    let email = validate_identity_input(username, email, password)?;
    let password_hash = hash_password(password)?;
    let must_change_password =
        generated_password.is_some() || must_change_password.unwrap_or(false);
    let (role, operator_scopes) = normalize_role_and_scopes(role, operator_scopes);
    let user = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let user = user_repo::create_with_options(
            txn,
            username.trim(),
            &email,
            &password_hash,
            role,
            status,
            must_change_password,
        )
        .await?;
        if role == UserRole::Operator {
            user_operator_scope_repo::replace_for_user(txn, user.id, &operator_scopes).await?;
        }
        Ok(user)
    })
    .await?;
    let users = hydrate_users(state, vec![user]).await?;
    tracing::debug!(username, "admin user created");
    let user = users.into_iter().next().ok_or_else(|| {
        AsterError::internal_error("created admin user hydration returned no item")
    })?;
    Ok(CreateAdminUserOutput {
        user,
        generated_password,
    })
}

pub async fn update_user<S>(
    state: &S,
    id: i64,
    input: AdminUpdateUserInput,
) -> Result<AdminUserInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let AdminUpdateUserInput {
        username,
        email,
        password,
        role,
        operator_scopes,
        status,
        must_change_password,
    } = input;
    tracing::debug!(
        user_id = id,
        username_changed = username.is_some(),
        email_changed = email.is_some(),
        password_changed = password.is_some(),
        role_changed = role.is_some(),
        status_changed = status.is_some(),
        must_change_password_changed = must_change_password.is_some(),
        "updating admin user"
    );
    let existing = user_repo::find_by_id(state.reader_db(), id).await?;
    if id == SUPER_ADMIN_USER_ID
        && (role.is_some() || status.is_some() || operator_scopes.is_some())
    {
        let role_changed = role.is_some_and(|next| next != existing.role);
        let status_changed = status.is_some_and(|next| next != existing.status);
        if role_changed || status_changed || operator_scopes.is_some() {
            return Err(AsterError::auth_forbidden(
                "super administrator role, status, and permissions cannot be changed",
            ));
        }
    }
    let existing_scopes = user_operator_scope_repo::list_for_user(state.reader_db(), id).await?;
    let (normalized_role, normalized_operator_scopes) =
        normalize_update_role_and_scopes(existing.role, role, operator_scopes, existing_scopes);

    let normalized_username = username
        .map(|value| {
            auth_service::validate_username(&value)?;
            Ok::<_, AsterError>(value.trim().to_string())
        })
        .transpose()?;
    let normalized_email = email.map(|value| normalize_email(&value)).transpose()?;
    let password_hash = password
        .map(|password| {
            auth_service::validate_password(&password)?;
            hash_password(&password)
        })
        .transpose()?;
    let bump_session_version = password_hash.is_some()
        || status == Some(UserStatus::Disabled)
        || must_change_password.is_some();
    let user = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let user = user_repo::update_admin(
            txn,
            id,
            user_repo::AdminUpdateUserInput {
                username: normalized_username,
                email: normalized_email,
                password_hash,
                role: Some(normalized_role),
                status,
                must_change_password,
                bump_session_version,
            },
        )
        .await?;
        if let Some(scopes) = normalized_operator_scopes.as_deref() {
            user_operator_scope_repo::replace_for_user(txn, id, scopes).await?;
        }
        Ok(user)
    })
    .await?;
    let users = hydrate_users(state, vec![user]).await?;
    tracing::debug!(user_id = id, "admin user updated");
    users
        .into_iter()
        .next()
        .ok_or_else(|| AsterError::internal_error("updated admin user hydration returned no item"))
}

pub async fn revoke_user_sessions<S>(state: &S, user_id: i64) -> Result<u64>
where
    S: DatabaseRuntimeState,
{
    tracing::debug!(user_id, "revoking admin user sessions");
    user_repo::find_by_id(state.reader_db(), user_id).await?;
    user_repo::bump_session_version(state.writer_db(), user_id).await?;
    let removed = user_repo::revoke_sessions_for_user(state.writer_db(), user_id).await?;
    tracing::debug!(user_id, removed, "admin user sessions revoked");
    Ok(removed)
}

pub async fn delete_user<S>(state: &S, user_id: i64) -> Result<DeleteAdminUserOutput>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState + ObjectStorageRuntimeState,
{
    if user_id == SUPER_ADMIN_USER_ID {
        return Err(AsterError::auth_forbidden(
            "super administrator cannot be deleted",
        ));
    }

    tracing::debug!(user_id, "deleting admin user");
    let user = get_user(state, user_id).await?;
    let deleted_profile_count = u64_to_usize(user.profile_count, "deleted profile count")?;
    profile_service::delete_uploaded_avatar_for_user(state, user_id).await?;

    let profiles = minecraft_profile_repo::list_by_user(state.reader_db(), user_id).await?;
    let mut deleted_profile_texture_count = 0usize;
    let mut revoked_yggdrasil_token_count = 0u64;
    for profile in profiles {
        if let Some(result) =
            yggdrasil_service::delete_profile_for_user(state, user_id, &profile.uuid).await?
        {
            deleted_profile_texture_count += result.deleted_texture_count;
            revoked_yggdrasil_token_count += result.revoked_token_count;
        }
    }

    let deleted_wardrobe_texture_count =
        texture_service::delete_all_wardrobe_textures_for_user(state, user_id)
            .await
            .map_err(|error| AsterError::internal_error(error.protocol_message()))?
            .len();
    let revoked_session_count =
        user_repo::revoke_sessions_for_user(state.writer_db(), user_id).await?;
    revoked_yggdrasil_token_count +=
        yggdrasil_token_repo::revoke_all_for_user(state.writer_db(), user_id).await?;
    user_repo::delete_by_id(state.writer_db(), user_id).await?;

    tracing::debug!(
        user_id,
        deleted_profile_count = user.profile_count,
        deleted_profile_texture_count,
        deleted_wardrobe_texture_count,
        revoked_session_count,
        revoked_yggdrasil_token_count,
        "admin user deleted"
    );
    Ok(DeleteAdminUserOutput {
        user,
        deleted_profile_count,
        deleted_profile_texture_count,
        deleted_wardrobe_texture_count,
        revoked_session_count,
        revoked_yggdrasil_token_count,
    })
}

async fn hydrate_users<S>(state: &S, users: Vec<user::Model>) -> Result<Vec<AdminUserInfo>>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let ids = users.iter().map(|user| user.id).collect::<Vec<_>>();
    tracing::debug!(count = ids.len(), "hydrating admin user summaries");
    let profile_counts = user_repo::count_profiles_by_user_ids(state.reader_db(), &ids).await?;
    let active_session_counts =
        user_repo::count_active_sessions_by_user_ids(state.reader_db(), &ids).await?;
    let operator_scope_map =
        user_operator_scope_repo::list_for_user_ids(state.reader_db(), &ids).await?;
    let profile_infos =
        profile_service::get_profile_info_map(state, &users, AvatarAudience::AdminUser).await?;
    Ok(users
        .into_iter()
        .map(|user| AdminUserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            pending_email: user.pending_email,
            role: user.role,
            operator_scopes: if user.role == UserRole::Operator {
                operator_scope_map
                    .get(&user.id)
                    .cloned()
                    .unwrap_or_default()
            } else {
                Vec::new()
            },
            status: user.status,
            must_change_password: user.must_change_password,
            session_version: user.session_version,
            profile_count: profile_counts.get(&user.id).copied().unwrap_or(0),
            active_session_count: active_session_counts.get(&user.id).copied().unwrap_or(0),
            profile: profile_infos
                .get(&user.id)
                .cloned()
                .unwrap_or_else(default_user_profile_info),
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repository::{
        auth_session_repo, minecraft_profile_texture_repo, minecraft_texture_repo,
        user_profile_repo,
    };
    use crate::entities::{auth_session, minecraft_profile, minecraft_texture, user_profile};
    use crate::runtime::AppState;
    use crate::types::{
        AvatarSource, MinecraftTextureModel, MinecraftTextureType, MinecraftTextureVisibility,
    };
    use chrono::{Duration, Utc};
    use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
    use std::sync::Arc;

    struct TestContext {
        state: AppState,
        texture_root: std::path::PathBuf,
    }

    async fn test_context() -> TestContext {
        let suffix = uuid::Uuid::new_v4();
        let root = std::env::temp_dir().join(format!("asteryggdrasil-admin-users-{suffix}"));
        let texture_root = root.join("storage");
        let db_cfg = crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            pool_size: 1,
            retry_count: 0,
        };
        let db = crate::db::connect_with_metrics(&db_cfg, crate::metrics_core::NoopMetrics::arc())
            .await
            .expect("admin user test database should connect");
        migration::Migrator::up(&db, None)
            .await
            .expect("admin user test migrations should run");
        crate::services::system_config_service::ensure_defaults(&db)
            .await
            .expect("admin user test defaults should seed");

        let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
        runtime_config
            .reload(&db)
            .await
            .expect("admin user test runtime config should reload");
        let config = Arc::new(crate::config::Config {
            database: db_cfg,
            object_storage: crate::config::ObjectStorageConfig {
                backend: "local".to_string(),
                local_root: texture_root.to_string_lossy().to_string(),
                ..Default::default()
            },
            cache: crate::config::CacheConfig {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        });
        let cache = crate::cache::create_cache(&config.cache).await;
        let object_storage = crate::object_storage::create_object_storage(&config.object_storage)
            .expect("admin user test object storage should initialize");
        let yggdrasil_rate_limiter = AppState::new_yggdrasil_rate_limiter(&config);
        let state = AppState {
            db_handles: crate::db::DbHandles::single(db),
            config,
            runtime_config,
            cache,
            object_storage,
            mail_sender: crate::services::mail_service::memory_sender(),
            metrics: crate::metrics_core::NoopMetrics::arc(),
            started_at: AppState::new_started_at(),
            yggdrasil_rate_limiter,
            background_task_dispatch_wakeup: AppState::new_background_task_dispatch_wakeup(),
        };
        TestContext {
            state,
            texture_root,
        }
    }

    async fn insert_user(state: &AppState, username: &str) -> user::Model {
        user_repo::create(
            state.writer_db(),
            username,
            &format!("{username}@example.com"),
            "password-hash",
            UserRole::User,
        )
        .await
        .expect("admin user test user should insert")
    }

    async fn insert_profile(
        state: &AppState,
        user_id: i64,
        name: &str,
    ) -> minecraft_profile::Model {
        minecraft_profile_repo::create(
            state.writer_db(),
            user_id,
            &crate::utils::id::new_unsigned_uuid(),
            name,
            MinecraftTextureModel::Default,
            "skin,cape",
        )
        .await
        .expect("admin user test profile should insert")
    }

    async fn insert_texture(
        state: &AppState,
        user_id: i64,
        storage_key: &str,
        is_wardrobe_item: bool,
    ) -> minecraft_texture::Model {
        let source = std::env::temp_dir().join(format!(
            "asteryggdrasil-texture-{}.png",
            uuid::Uuid::new_v4()
        ));
        tokio::fs::write(&source, [1, 2, 3, 4])
            .await
            .expect("admin user test source texture should write");
        state
            .object_storage()
            .put_file(storage_key, &source)
            .await
            .expect("admin user test texture blob should store");
        let _ = tokio::fs::remove_file(&source).await;

        minecraft_texture_repo::create(
            state.writer_db(),
            minecraft_texture_repo::CreateMinecraftTexture {
                user_id,
                texture_type: MinecraftTextureType::Skin,
                hash: storage_key,
                storage_key,
                mime_type: "image/png",
                file_size: 4,
                width: 64,
                height: 64,
                texture_model: MinecraftTextureModel::Default,
                visibility: MinecraftTextureVisibility::Private,
                is_wardrobe_item,
                display_name: None,
            },
        )
        .await
        .expect("admin user test texture should insert")
    }

    async fn insert_active_session(state: &AppState, user_id: i64, id: &str) {
        let now = Utc::now();
        auth_session_repo::create(
            state.writer_db(),
            auth_session::ActiveModel {
                id: Set(id.to_string()),
                user_id: Set(user_id),
                current_refresh_jti: Set(format!("{id}-jti")),
                previous_refresh_jti: Set(None),
                refresh_expires_at: Set(now + Duration::hours(1)),
                user_agent: Set(None),
                ip_address: Set(None),
                created_at: Set(now),
                last_seen_at: Set(now),
                revoked_at: Set(None),
            },
        )
        .await
        .expect("admin user test session should insert");
    }

    async fn insert_yggdrasil_token(
        state: &AppState,
        user_id: i64,
        access_hash: &str,
        selected_profile_id: Option<i64>,
    ) {
        let now = Utc::now();
        yggdrasil_token_repo::create(
            state.writer_db(),
            yggdrasil_token_repo::CreateYggdrasilToken {
                user_id,
                access_token_hash: access_hash,
                client_token: access_hash,
                selected_profile_id,
                issued_at: now,
                expires_at: now + Duration::hours(1),
                user_agent: None,
                ip_address: None,
            },
        )
        .await
        .expect("admin user test yggdrasil token should insert");
    }

    async fn create_uploaded_avatar(ctx: &TestContext, user_id: i64) -> String {
        let now = Utc::now();
        let avatar_prefix = format!("avatar/user/{user_id}/v1");
        user_profile_repo::create(
            ctx.state.writer_db(),
            user_profile::ActiveModel {
                user_id: Set(user_id),
                display_name: Set(Some("Display Cat".to_string())),
                avatar_source: Set(AvatarSource::Upload),
                avatar_key: Set(Some(avatar_prefix.clone())),
                avatar_version: Set(1),
                created_at: Set(now),
                updated_at: Set(now),
            },
        )
        .await
        .expect("admin user test user profile should insert");
        let source = std::env::temp_dir().join(format!(
            "asteryggdrasil-avatar-{}.webp",
            uuid::Uuid::new_v4()
        ));
        tokio::fs::write(&source, b"avatar")
            .await
            .expect("admin user test avatar source should write");
        for size in [512, 1024] {
            ctx.state
                .object_storage()
                .put_file(&format!("{avatar_prefix}/{size}.webp"), &source)
                .await
                .expect("admin user test avatar object should store");
        }
        let _ = tokio::fs::remove_file(&source).await;
        avatar_prefix
    }

    async fn count_users(state: &AppState) -> u64 {
        user::Entity::find()
            .count(state.reader_db())
            .await
            .expect("admin user test user count should load")
    }

    #[tokio::test]
    async fn create_list_get_and_update_user_cover_admin_workflow() {
        let ctx = test_context().await;
        let _super_admin = insert_user(&ctx.state, "root-user").await;
        let created = create_user(
            &ctx.state,
            " new-user ",
            " NEW-USER@EXAMPLE.COM ",
            None,
            UserRole::User,
            None,
            UserStatus::Active,
            None,
        )
        .await
        .unwrap();

        assert_eq!(created.user.username, "new-user");
        assert_eq!(created.user.email, "new-user@example.com");
        assert!(created.generated_password.is_some());
        assert!(created.user.must_change_password);

        let page = list_users(
            &ctx.state,
            20,
            0,
            AdminUserListFilters {
                keyword: Some("NEW-USER".to_string()),
                role: Some(UserRole::User),
                status: Some(UserStatus::Active),
            },
            AdminUserSortBy::Username,
            SortOrder::Asc,
        )
        .await
        .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].username, "new-user");

        let updated = update_user(
            &ctx.state,
            created.user.id,
            AdminUpdateUserInput {
                username: Some("renamed-user".to_string()),
                email: Some("renamed@example.com".to_string()),
                password: Some("new-password".to_string()),
                role: Some(UserRole::Admin),
                operator_scopes: None,
                status: Some(UserStatus::Disabled),
                must_change_password: Some(true),
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.username, "renamed-user");
        assert_eq!(updated.email, "renamed@example.com");
        assert_eq!(updated.role, UserRole::Admin);
        assert_eq!(updated.status, UserStatus::Disabled);
        assert!(updated.must_change_password);

        let loaded = get_user(&ctx.state, created.user.id).await.unwrap();
        assert_eq!(loaded.username, "renamed-user");
    }

    #[tokio::test]
    async fn update_user_rejects_super_admin_role_or_status_changes() {
        let ctx = test_context().await;
        let super_admin = insert_user(&ctx.state, "root-user").await;
        assert_eq!(super_admin.id, SUPER_ADMIN_USER_ID);

        let error = update_user(
            &ctx.state,
            super_admin.id,
            AdminUpdateUserInput {
                username: None,
                email: None,
                password: None,
                role: Some(UserRole::Admin),
                operator_scopes: None,
                status: Some(UserStatus::Disabled),
                must_change_password: None,
            },
        )
        .await
        .unwrap_err();

        assert!(error.message().contains("super administrator"));
    }

    #[tokio::test]
    async fn create_user_normalizes_operator_scopes_by_role() {
        let ctx = test_context().await;
        let _super_admin = insert_user(&ctx.state, "root-user").await;

        let operator = create_user(
            &ctx.state,
            "operator-user",
            "operator@example.com",
            Some("operator-password"),
            UserRole::Operator,
            Some(vec![
                OperatorScope::Users,
                OperatorScope::TextureLibrary,
                OperatorScope::Users,
            ]),
            UserStatus::Active,
            None,
        )
        .await
        .unwrap();

        assert_eq!(operator.user.role, UserRole::Operator);
        assert_eq!(
            operator.user.operator_scopes,
            vec![OperatorScope::TextureLibrary, OperatorScope::Users]
        );

        let admin_with_scopes = create_user(
            &ctx.state,
            "scoped-admin",
            "scoped-admin@example.com",
            Some("scoped-admin-password"),
            UserRole::Admin,
            Some(vec![OperatorScope::Audit]),
            UserStatus::Active,
            None,
        )
        .await
        .unwrap();

        assert_eq!(admin_with_scopes.user.role, UserRole::Operator);
        assert_eq!(
            admin_with_scopes.user.operator_scopes,
            vec![OperatorScope::Audit]
        );

        let user_with_scopes = create_user(
            &ctx.state,
            "scoped-user",
            "scoped-user@example.com",
            Some("scoped-user-password"),
            UserRole::User,
            Some(vec![OperatorScope::Tasks]),
            UserStatus::Active,
            None,
        )
        .await
        .unwrap();

        assert_eq!(user_with_scopes.user.role, UserRole::User);
        assert!(user_with_scopes.user.operator_scopes.is_empty());
        assert!(
            user_operator_scope_repo::list_for_user(
                ctx.state.reader_db(),
                user_with_scopes.user.id
            )
            .await
            .unwrap()
            .is_empty()
        );
    }

    #[tokio::test]
    async fn update_user_normalizes_scope_changes_and_preserves_operator_scopes() {
        let ctx = test_context().await;
        let _super_admin = insert_user(&ctx.state, "root-user").await;
        let target = create_user(
            &ctx.state,
            "target-user",
            "target@example.com",
            Some("target-password"),
            UserRole::Admin,
            None,
            UserStatus::Active,
            None,
        )
        .await
        .unwrap()
        .user;

        let downgraded = update_user(
            &ctx.state,
            target.id,
            AdminUpdateUserInput {
                username: None,
                email: None,
                password: None,
                role: None,
                operator_scopes: Some(vec![OperatorScope::TextureLibrary]),
                status: None,
                must_change_password: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(downgraded.role, UserRole::Operator);
        assert_eq!(
            downgraded.operator_scopes,
            vec![OperatorScope::TextureLibrary]
        );

        let promoted = update_user(
            &ctx.state,
            target.id,
            AdminUpdateUserInput {
                username: None,
                email: None,
                password: None,
                role: Some(UserRole::Admin),
                operator_scopes: None,
                status: None,
                must_change_password: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(promoted.role, UserRole::Admin);
        assert!(promoted.operator_scopes.is_empty());
        assert!(
            user_operator_scope_repo::list_for_user(ctx.state.reader_db(), target.id)
                .await
                .unwrap()
                .is_empty()
        );

        let downgraded_again = update_user(
            &ctx.state,
            target.id,
            AdminUpdateUserInput {
                username: None,
                email: None,
                password: None,
                role: Some(UserRole::Admin),
                operator_scopes: Some(vec![OperatorScope::Audit]),
                status: None,
                must_change_password: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(downgraded_again.role, UserRole::Operator);
        assert_eq!(downgraded_again.operator_scopes, vec![OperatorScope::Audit]);

        let renamed_operator = update_user(
            &ctx.state,
            target.id,
            AdminUpdateUserInput {
                username: Some("renamed-operator".to_string()),
                email: None,
                password: None,
                role: None,
                operator_scopes: None,
                status: None,
                must_change_password: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(renamed_operator.role, UserRole::Operator);
        assert_eq!(renamed_operator.operator_scopes, vec![OperatorScope::Audit]);

        let demoted_user = update_user(
            &ctx.state,
            target.id,
            AdminUpdateUserInput {
                username: None,
                email: None,
                password: None,
                role: Some(UserRole::User),
                operator_scopes: Some(vec![OperatorScope::Tasks]),
                status: None,
                must_change_password: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(demoted_user.role, UserRole::User);
        assert!(demoted_user.operator_scopes.is_empty());
        assert!(
            user_operator_scope_repo::list_for_user(ctx.state.reader_db(), target.id)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn update_user_rejects_super_admin_scope_changes_even_when_role_is_unchanged() {
        let ctx = test_context().await;
        let super_admin = insert_user(&ctx.state, "root-user").await;
        assert_eq!(super_admin.id, SUPER_ADMIN_USER_ID);

        let error = update_user(
            &ctx.state,
            super_admin.id,
            AdminUpdateUserInput {
                username: None,
                email: None,
                password: None,
                role: None,
                operator_scopes: Some(Vec::new()),
                status: None,
                must_change_password: None,
            },
        )
        .await
        .unwrap_err();

        assert!(error.message().contains("super administrator"));
    }

    #[tokio::test]
    async fn auth_user_info_includes_operator_scopes_only_for_operator() {
        let ctx = test_context().await;
        let _super_admin = insert_user(&ctx.state, "root-user").await;
        let operator = create_user(
            &ctx.state,
            "auth-operator",
            "auth-operator@example.com",
            Some("auth-operator-password"),
            UserRole::Operator,
            Some(vec![OperatorScope::TextureLibrary]),
            UserStatus::Active,
            None,
        )
        .await
        .unwrap()
        .user;
        let user = create_user(
            &ctx.state,
            "auth-user",
            "auth-user@example.com",
            Some("auth-user-password"),
            UserRole::User,
            Some(vec![OperatorScope::Users]),
            UserStatus::Active,
            None,
        )
        .await
        .unwrap()
        .user;

        let operator_model = user_repo::find_by_id(ctx.state.reader_db(), operator.id)
            .await
            .unwrap();
        let operator_info = auth_service::auth_user_info(&ctx.state, operator_model)
            .await
            .unwrap();
        assert_eq!(
            operator_info.operator_scopes,
            vec![OperatorScope::TextureLibrary]
        );

        let user_model = user_repo::find_by_id(ctx.state.reader_db(), user.id)
            .await
            .unwrap();
        let user_info = auth_service::auth_user_info(&ctx.state, user_model)
            .await
            .unwrap();
        assert!(user_info.operator_scopes.is_empty());
    }

    #[tokio::test]
    async fn revoke_user_sessions_revokes_sessions_and_bumps_session_version() {
        let ctx = test_context().await;
        let user = insert_user(&ctx.state, "session-target").await;
        insert_active_session(&ctx.state, user.id, "session-a").await;
        insert_active_session(&ctx.state, user.id, "session-b").await;

        let removed = revoke_user_sessions(&ctx.state, user.id).await.unwrap();

        assert_eq!(removed, 2);
        let updated = user_repo::find_by_id(ctx.state.reader_db(), user.id)
            .await
            .unwrap();
        assert_eq!(updated.session_version, user.session_version + 1);
        let active_count =
            user_repo::count_active_sessions_by_user_ids(ctx.state.reader_db(), &[user.id])
                .await
                .unwrap();
        assert_eq!(active_count.get(&user.id).copied().unwrap_or(0), 0);
    }

    #[tokio::test]
    async fn delete_user_cleans_profiles_textures_avatar_sessions_and_tokens() {
        let ctx = test_context().await;
        let _super_admin = insert_user(&ctx.state, "root-user").await;
        let user = insert_user(&ctx.state, "delete-target").await;
        let avatar_prefix = create_uploaded_avatar(&ctx, user.id).await;
        let profile = insert_profile(&ctx.state, user.id, "DeleteTarget").await;
        let profile_texture =
            insert_texture(&ctx.state, user.id, "profile/delete-target-skin.png", false).await;
        minecraft_profile_texture_repo::upsert_for_profile(
            ctx.state.writer_db(),
            minecraft_profile_texture_repo::UpsertMinecraftProfileTexture {
                profile_id: profile.id,
                texture_id: profile_texture.id,
                texture_type: MinecraftTextureType::Skin,
            },
        )
        .await
        .unwrap();
        let wardrobe_texture =
            insert_texture(&ctx.state, user.id, "wardrobe/delete-target-skin.png", true).await;
        insert_active_session(&ctx.state, user.id, "delete-session").await;
        insert_yggdrasil_token(
            &ctx.state,
            user.id,
            "selected-profile-token",
            Some(profile.id),
        )
        .await;
        insert_yggdrasil_token(&ctx.state, user.id, "user-token", None).await;

        let output = delete_user(&ctx.state, user.id).await.unwrap();

        assert_eq!(output.user.id, user.id);
        assert_eq!(
            output.user.profile.display_name,
            Some("Display Cat".to_string())
        );
        assert_eq!(output.deleted_profile_count, 1);
        assert_eq!(output.deleted_profile_texture_count, 1);
        assert_eq!(output.deleted_wardrobe_texture_count, 1);
        assert_eq!(output.revoked_session_count, 1);
        assert_eq!(output.revoked_yggdrasil_token_count, 2);
        assert!(
            user_repo::find_by_id(ctx.state.reader_db(), user.id)
                .await
                .is_err()
        );
        assert!(
            minecraft_profile::Entity::find()
                .filter(minecraft_profile::Column::UserId.eq(user.id))
                .count(ctx.state.reader_db())
                .await
                .unwrap()
                == 0
        );
        assert!(
            minecraft_texture::Entity::find()
                .filter(minecraft_texture::Column::UserId.eq(user.id))
                .count(ctx.state.reader_db())
                .await
                .unwrap()
                == 0
        );
        assert!(
            !ctx.state
                .object_storage()
                .exists(&format!("{avatar_prefix}/512.webp"))
                .await
                .unwrap()
        );
        assert!(
            !ctx.state
                .object_storage()
                .exists(&format!("{avatar_prefix}/1024.webp"))
                .await
                .unwrap()
        );
        assert!(
            !ctx.state
                .object_storage()
                .exists(&profile_texture.storage_key)
                .await
                .unwrap()
        );
        assert!(
            !ctx.state
                .object_storage()
                .exists(&wardrobe_texture.storage_key)
                .await
                .unwrap()
        );
        assert_eq!(count_users(&ctx.state).await, 1);

        let _ = tokio::fs::remove_dir_all(&ctx.texture_root).await;
    }

    #[tokio::test]
    async fn delete_user_allows_user_without_related_records() {
        let ctx = test_context().await;
        let _super_admin = insert_user(&ctx.state, "root-user").await;
        let user = insert_user(&ctx.state, "plain-target").await;

        let output = delete_user(&ctx.state, user.id).await.unwrap();

        assert_eq!(output.deleted_profile_count, 0);
        assert_eq!(output.deleted_profile_texture_count, 0);
        assert_eq!(output.deleted_wardrobe_texture_count, 0);
        assert_eq!(output.revoked_session_count, 0);
        assert_eq!(output.revoked_yggdrasil_token_count, 0);
        assert!(
            user_repo::find_by_id(ctx.state.reader_db(), user.id)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn delete_user_rejects_super_admin_and_missing_user() {
        let ctx = test_context().await;
        let super_admin = insert_user(&ctx.state, "root-user").await;
        assert_eq!(super_admin.id, SUPER_ADMIN_USER_ID);

        let super_error = delete_user(&ctx.state, super_admin.id).await.unwrap_err();
        assert!(super_error.message().contains("super administrator"));
        assert!(
            user_repo::find_by_id(ctx.state.reader_db(), super_admin.id)
                .await
                .is_ok()
        );

        let missing_error = delete_user(&ctx.state, 404).await.unwrap_err();
        assert!(missing_error.message().contains("user #404"));
    }
}
