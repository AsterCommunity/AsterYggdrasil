//! User repository.

use crate::db::repository::search_query;
use crate::entities::{
    auth_session, minecraft_profile,
    user::{self, Entity as User},
};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::types::{UserRole, UserStatus};
use aster_forge_api::CursorSlice;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, sea_query::Expr,
};

#[derive(Debug, Clone, Default)]
pub struct AdminUserFilters {
    pub keyword: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
}

pub async fn count_all<C: ConnectionTrait>(db: &C) -> Result<u64> {
    User::find()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn count_created_between<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<u64> {
    User::find()
        .filter(user::Column::CreatedAt.gte(start))
        .filter(user::Column::CreatedAt.lt(end))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<user::Model> {
    User::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)?
        .ok_or_else(|| AsterError::record_not_found(format!("user #{id}")))
}

pub async fn find_by_ids<C: ConnectionTrait>(db: &C, ids: &[i64]) -> Result<Vec<user::Model>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    User::find()
        .filter(user::Column::Id.is_in(ids.iter().copied()))
        .order_by_asc(user::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_identifier<C: ConnectionTrait>(
    db: &C,
    identifier: &str,
) -> Result<Option<user::Model>> {
    User::find()
        .filter(
            sea_orm::Condition::any()
                .add(user::Column::Username.eq(identifier))
                .add(user::Column::Email.eq(identifier)),
        )
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_email<C: ConnectionTrait>(db: &C, email: &str) -> Result<Option<user::Model>> {
    User::find()
        .filter(user::Column::Email.eq(email))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_pending_email<C: ConnectionTrait>(
    db: &C,
    email: &str,
) -> Result<Option<user::Model>> {
    User::find()
        .filter(user::Column::PendingEmail.eq(email))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_username<C: ConnectionTrait>(
    db: &C,
    username: &str,
) -> Result<Option<user::Model>> {
    User::find()
        .filter(user::Column::Username.eq(username))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_public_uuid<C: ConnectionTrait>(
    db: &C,
    public_uuid: &str,
) -> Result<Option<user::Model>> {
    User::find()
        .filter(user::Column::PublicUuid.eq(public_uuid))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn list_admin_cursor<C: ConnectionTrait>(
    db: &C,
    filters: AdminUserFilters,
    limit: u64,
    after: Option<(DateTime<Utc>, i64)>,
) -> Result<CursorSlice<user::Model>> {
    let limit = limit.clamp(1, 100);
    let mut query = apply_admin_filters(User::find(), &filters);
    let total = query
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    if let Some((created_at, id)) = after {
        query = query.filter(
            Condition::any()
                .add(user::Column::CreatedAt.lt(created_at))
                .add(
                    Condition::all()
                        .add(user::Column::CreatedAt.eq(created_at))
                        .add(user::Column::Id.lt(id)),
                ),
        );
    }

    let items = query
        .order_by_desc(user::Column::CreatedAt)
        .order_by_desc(user::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

fn apply_admin_filters(
    mut query: sea_orm::Select<User>,
    filters: &AdminUserFilters,
) -> sea_orm::Select<User> {
    if let Some(role) = filters.role {
        query = query.filter(user::Column::Role.eq(role));
    }
    if let Some(status) = filters.status {
        query = query.filter(user::Column::Status.eq(status));
    }
    if let Some(keyword) = filters
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        query = query.filter(
            search_query::lower_like_condition(user::Column::Username, keyword).or(
                search_query::lower_like_condition(user::Column::Email, keyword),
            ),
        );
    }
    query
}

pub async fn count_profiles_by_user_ids<C: ConnectionTrait>(
    db: &C,
    user_ids: &[i64],
) -> Result<std::collections::HashMap<i64, u64>> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let rows = minecraft_profile::Entity::find()
        .select_only()
        .column(minecraft_profile::Column::UserId)
        .column_as(minecraft_profile::Column::Id.count(), "profile_count")
        .filter(minecraft_profile::Column::UserId.is_in(user_ids.iter().copied()))
        .group_by(minecraft_profile::Column::UserId)
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    rows.into_iter()
        .map(|(user_id, count)| {
            crate::utils::numbers::i64_to_u64(count, "profile count").map(|count| (user_id, count))
        })
        .collect()
}

pub async fn count_active_sessions_by_user_ids<C: ConnectionTrait>(
    db: &C,
    user_ids: &[i64],
) -> Result<std::collections::HashMap<i64, u64>> {
    if user_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let now = chrono::Utc::now();
    let rows = auth_session::Entity::find()
        .select_only()
        .column(auth_session::Column::UserId)
        .column_as(auth_session::Column::Id.count(), "session_count")
        .filter(auth_session::Column::UserId.is_in(user_ids.iter().copied()))
        .filter(auth_session::Column::RevokedAt.is_null())
        .filter(auth_session::Column::RefreshExpiresAt.gt(now))
        .group_by(auth_session::Column::UserId)
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    rows.into_iter()
        .map(|(user_id, count)| {
            crate::utils::numbers::i64_to_u64(count, "active session count")
                .map(|count| (user_id, count))
        })
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct AdminUpdateUserInput {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub must_change_password: Option<bool>,
    pub bump_session_version: bool,
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    username: &str,
    email: &str,
    password_hash: &str,
    role: UserRole,
) -> Result<user::Model> {
    create_with_options(
        db,
        username,
        email,
        password_hash,
        role,
        UserStatus::Active,
        false,
    )
    .await
}

pub async fn create_with_options<C: ConnectionTrait>(
    db: &C,
    username: &str,
    email: &str,
    password_hash: &str,
    role: UserRole,
    status: UserStatus,
    must_change_password: bool,
) -> Result<user::Model> {
    let now = chrono::Utc::now();
    let public_uuid = unique_public_uuid(db).await?;
    user::ActiveModel {
        public_uuid: Set(public_uuid),
        username: Set(username.to_string()),
        email: Set(email.to_string()),
        password_hash: Set(password_hash.to_string()),
        role: Set(role),
        status: Set(status),
        must_change_password: Set(must_change_password),
        session_version: Set(1),
        email_verified_at: Set(Some(now)),
        pending_email: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

pub async fn unique_public_uuid<C: ConnectionTrait>(db: &C) -> Result<String> {
    crate::utils::id::new_best_effort_uuid("user public UUID", |candidate| {
        let public_uuid = candidate.simple().to_string();
        async move {
            find_by_public_uuid(db, &public_uuid)
                .await
                .map(|user| user.is_some())
        }
    })
    .await
    .map(|uuid| uuid.simple().to_string())
}

pub async fn update_admin<C: ConnectionTrait>(
    db: &C,
    id: i64,
    input: AdminUpdateUserInput,
) -> Result<user::Model> {
    let existing = find_by_id(db, id).await?;
    let mut active: user::ActiveModel = existing.into();
    if let Some(username) = input.username {
        active.username = Set(username);
    }
    if let Some(email) = input.email {
        active.email = Set(email);
    }
    if let Some(password_hash) = input.password_hash {
        active.password_hash = Set(password_hash);
    }
    if let Some(role) = input.role {
        active.role = Set(role);
    }
    if let Some(status) = input.status {
        active.status = Set(status);
    }
    if let Some(must_change_password) = input.must_change_password {
        active.must_change_password = Set(must_change_password);
    }
    if input.bump_session_version {
        active.session_version = Set(active.session_version.unwrap() + 1);
    }
    active.updated_at = Set(chrono::Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn revoke_sessions_for_user<C: ConnectionTrait>(db: &C, user_id: i64) -> Result<u64> {
    let result = auth_session::Entity::update_many()
        .col_expr(
            auth_session::Column::RevokedAt,
            Expr::value(Some(chrono::Utc::now())),
        )
        .filter(auth_session::Column::UserId.eq(user_id))
        .filter(auth_session::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

pub async fn bump_session_version<C: ConnectionTrait>(db: &C, user_id: i64) -> Result<()> {
    let user = find_by_id(db, user_id).await?;
    let mut active: user::ActiveModel = user.into();
    active.session_version = Set(active.session_version.unwrap() + 1);
    active.updated_at = Set(chrono::Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(())
}

pub async fn delete_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<user::Model> {
    let user = find_by_id(db, id).await?;
    let active: user::ActiveModel = user.clone().into();
    active
        .delete(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::{
        bump_session_version, count_all, create, delete_by_id, find_by_id, find_by_identifier,
        find_by_ids, find_by_public_uuid,
    };
    use crate::config::DatabaseConfig;
    use crate::types::{UserRole, UserStatus};

    async fn build_test_db() -> sea_orm::DatabaseConnection {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("user repo test DB should connect");
        migration::Migrator::up(&db, None)
            .await
            .expect("user repo test migrations should succeed");
        db
    }

    #[tokio::test]
    async fn create_count_find_and_bump_session_version() {
        let db = build_test_db().await;
        assert_eq!(count_all(&db).await.unwrap(), 0);

        let user = create(
            &db,
            "repo-user",
            "repo-user@example.com",
            "password-hash",
            UserRole::Admin,
        )
        .await
        .unwrap();

        assert_eq!(count_all(&db).await.unwrap(), 1);
        assert_eq!(user.username, "repo-user");
        assert_eq!(user.email, "repo-user@example.com");
        assert_eq!(user.role, UserRole::Admin);
        assert_eq!(user.status, UserStatus::Active);
        assert_eq!(user.session_version, 1);
        assert_eq!(user.public_uuid.len(), 32);
        assert!(
            user.public_uuid
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
        assert!(user.email_verified_at.is_some());

        assert_eq!(
            find_by_id(&db, user.id).await.unwrap().username,
            "repo-user"
        );
        assert_eq!(
            find_by_identifier(&db, "repo-user")
                .await
                .unwrap()
                .unwrap()
                .id,
            user.id
        );
        assert_eq!(
            find_by_identifier(&db, "repo-user@example.com")
                .await
                .unwrap()
                .unwrap()
                .id,
            user.id
        );
        assert_eq!(
            find_by_public_uuid(&db, &user.public_uuid)
                .await
                .unwrap()
                .unwrap()
                .id,
            user.id
        );
        assert!(find_by_identifier(&db, "missing").await.unwrap().is_none());

        bump_session_version(&db, user.id).await.unwrap();
        let bumped = find_by_id(&db, user.id).await.unwrap();
        assert_eq!(bumped.session_version, 2);
        assert!(bumped.updated_at >= user.updated_at);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn find_by_ids_returns_empty_for_empty_input_and_orders_by_id() {
        let db = build_test_db().await;
        assert!(find_by_ids(&db, &[]).await.unwrap().is_empty());

        let first = create(
            &db,
            "repo-user-a",
            "repo-user-a@example.com",
            "password-hash",
            UserRole::User,
        )
        .await
        .unwrap();
        let second = create(
            &db,
            "repo-user-b",
            "repo-user-b@example.com",
            "password-hash",
            UserRole::User,
        )
        .await
        .unwrap();
        let third = create(
            &db,
            "repo-user-c",
            "repo-user-c@example.com",
            "password-hash",
            UserRole::User,
        )
        .await
        .unwrap();

        let users = find_by_ids(&db, &[third.id, first.id, 999_999, second.id])
            .await
            .unwrap();
        assert_eq!(
            users.into_iter().map(|user| user.id).collect::<Vec<_>>(),
            vec![first.id, second.id, third.id]
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn find_by_id_and_bump_missing_user_return_not_found() {
        let db = build_test_db().await;

        let missing = find_by_id(&db, 404).await.unwrap_err();
        assert!(missing.message().contains("user #404"));
        assert!(
            bump_session_version(&db, 404)
                .await
                .unwrap_err()
                .message()
                .contains("user #404")
        );

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn delete_by_id_removes_user_and_returns_deleted_model() {
        let db = build_test_db().await;
        let user = create(
            &db,
            "delete-user",
            "delete-user@example.com",
            "password-hash",
            UserRole::User,
        )
        .await
        .unwrap();

        let deleted = delete_by_id(&db, user.id).await.unwrap();

        assert_eq!(deleted.id, user.id);
        assert_eq!(count_all(&db).await.unwrap(), 0);
        assert!(find_by_id(&db, user.id).await.is_err());
        assert!(delete_by_id(&db, user.id).await.is_err());

        db.close().await.unwrap();
    }
}
