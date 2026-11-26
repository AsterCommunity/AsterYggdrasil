//! Auth session repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::auth_session::{self, Entity as AuthSession};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, sea_query::Expr,
};

/// Inserts a new auth session row.
pub async fn create<C: ConnectionTrait>(
    db: &C,
    model: auth_session::ActiveModel,
) -> Result<auth_session::Model> {
    model
        .insert(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Looks up a session by the jti of its current refresh token.
///
/// Returns `None` when the jti is not the session's current refresh token (rotated or revoked
/// tokens do not match); revocation and expiry are not filtered here.
pub async fn find_by_refresh_jti<C: ConnectionTrait>(
    db: &C,
    refresh_jti: &str,
) -> Result<Option<auth_session::Model>> {
    AuthSession::find()
        .filter(auth_session::Column::CurrentRefreshJti.eq(refresh_jti))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Looks up a session by the jti of its previous refresh token, i.e. one rotated away.
///
/// Used to detect refresh token reuse, which is a signal of theft; returns `None` when the
/// session never held that jti as its previous token.
pub async fn find_by_previous_refresh_jti<C: ConnectionTrait>(
    db: &C,
    refresh_jti: &str,
) -> Result<Option<auth_session::Model>> {
    AuthSession::find()
        .filter(auth_session::Column::PreviousRefreshJti.eq(refresh_jti))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Lists all sessions of a user, including revoked and expired ones, newest seen first.
pub async fn list_by_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Vec<auth_session::Model>> {
    AuthSession::find()
        .filter(auth_session::Column::UserId.eq(user_id))
        .order_by_desc(auth_session::Column::LastSeenAt)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Paginates a user's sessions using a keyset cursor over `(last_seen_at, id)`, newest seen first.
///
/// `limit` is clamped to 1..=100; `after` is the cursor of the last item of the previous page.
/// The returned [`CursorSlice`] carries the user's total session count and a next cursor, which
/// is absent on the last page.
pub async fn list_by_user_cursor<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    limit: u64,
    after: Option<(chrono::DateTime<Utc>, String)>,
) -> Result<CursorSlice<auth_session::Model>> {
    let limit = limit.clamp(1, 100);
    let mut query = AuthSession::find().filter(auth_session::Column::UserId.eq(user_id));
    let total = query
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    if let Some((last_seen_at, id)) = after {
        query = query.filter(
            Condition::any()
                .add(auth_session::Column::LastSeenAt.lt(last_seen_at))
                .add(
                    Condition::all()
                        .add(auth_session::Column::LastSeenAt.eq(last_seen_at))
                        .add(auth_session::Column::Id.lt(id)),
                ),
        );
    }
    let items = query
        .order_by_desc(auth_session::Column::LastSeenAt)
        .order_by_desc(auth_session::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

/// Lists a user's still-active sessions: neither revoked nor past `refresh_expires_at`.
///
/// Ordered by `last_seen_at` descending, most recently active first.
pub async fn list_active_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Vec<auth_session::Model>> {
    AuthSession::find()
        .filter(auth_session::Column::UserId.eq(user_id))
        .filter(auth_session::Column::RevokedAt.is_null())
        .filter(auth_session::Column::RefreshExpiresAt.gt(Utc::now()))
        .order_by_desc(auth_session::Column::LastSeenAt)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Counts currently active sessions across all users: not revoked and not past refresh expiry.
pub async fn count_active<C: ConnectionTrait>(db: &C) -> Result<u64> {
    AuthSession::find()
        .filter(auth_session::Column::RevokedAt.is_null())
        .filter(auth_session::Column::RefreshExpiresAt.gt(Utc::now()))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Loads a session by id scoped to its owner.
///
/// Returns `None` when the session does not exist or belongs to a different user.
pub async fn find_by_id_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    id: &str,
) -> Result<Option<auth_session::Model>> {
    AuthSession::find()
        .filter(auth_session::Column::UserId.eq(user_id))
        .filter(auth_session::Column::Id.eq(id))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Rotates a session's refresh token jti in a single guarded update.
///
/// The current jti moves to `previous_refresh_jti` (enabling reuse detection), the new jti and
/// expiry replace the old, and the client fingerprint fields are refreshed. Returns `false`
/// when no unrevoked session currently holds `current_refresh_jti` — a second use of the same
/// token fails here.
pub async fn rotate_refresh<C: ConnectionTrait>(
    db: &C,
    current_refresh_jti: &str,
    next_refresh_jti: &str,
    refresh_expires_at: chrono::DateTime<Utc>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    last_seen_at: chrono::DateTime<Utc>,
) -> Result<bool> {
    let result = AuthSession::update_many()
        .col_expr(
            auth_session::Column::CurrentRefreshJti,
            Expr::value(next_refresh_jti.to_string()),
        )
        .col_expr(
            auth_session::Column::PreviousRefreshJti,
            Expr::value(Some(current_refresh_jti.to_string())),
        )
        .col_expr(
            auth_session::Column::RefreshExpiresAt,
            Expr::value(refresh_expires_at),
        )
        .col_expr(
            auth_session::Column::IpAddress,
            Expr::value(ip_address.map(str::to_string)),
        )
        .col_expr(
            auth_session::Column::UserAgent,
            Expr::value(user_agent.map(str::to_string)),
        )
        .col_expr(auth_session::Column::LastSeenAt, Expr::value(last_seen_at))
        .filter(auth_session::Column::CurrentRefreshJti.eq(current_refresh_jti))
        .filter(auth_session::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected == 1)
}

/// Revokes a session owned by `user_id` if not revoked already.
///
/// Returns `true` on transition, `false` when the session is missing, foreign, or already
/// revoked.
pub async fn revoke_by_id_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    id: &str,
    now: chrono::DateTime<Utc>,
) -> Result<bool> {
    let result = AuthSession::update_many()
        .col_expr(auth_session::Column::RevokedAt, Expr::value(Some(now)))
        .filter(auth_session::Column::UserId.eq(user_id))
        .filter(auth_session::Column::Id.eq(id))
        .filter(auth_session::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected == 1)
}

/// Revokes the session currently holding `refresh_jti`, used when refresh tokens are rejected
/// or reused.
///
/// Returns `true` on transition, `false` when no unrevoked session matches.
pub async fn revoke_by_refresh_jti<C: ConnectionTrait>(db: &C, refresh_jti: &str) -> Result<bool> {
    let result = AuthSession::update_many()
        .col_expr(
            auth_session::Column::RevokedAt,
            Expr::value(Some(chrono::Utc::now())),
        )
        .filter(auth_session::Column::CurrentRefreshJti.eq(refresh_jti))
        .filter(auth_session::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected == 1)
}

/// Revokes every unrevoked session of a user except `keep_session_id`, returning how many
/// sessions were revoked.
///
/// Used by "sign out everywhere" flows that preserve the current session.
pub async fn revoke_all_for_user_except_id<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    keep_session_id: &str,
    now: chrono::DateTime<Utc>,
) -> Result<u64> {
    let result = AuthSession::update_many()
        .col_expr(auth_session::Column::RevokedAt, Expr::value(Some(now)))
        .filter(auth_session::Column::UserId.eq(user_id))
        .filter(auth_session::Column::Id.ne(keep_session_id))
        .filter(auth_session::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

/// Deletes sessions whose refresh token expired before `now`, returning the number of rows
/// removed.
///
/// Revoked sessions are only removed once their expiry passes as well, keeping them visible
/// until then.
pub async fn delete_expired<C: ConnectionTrait>(
    db: &C,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<u64> {
    let result = AuthSession::delete_many()
        .filter(auth_session::Column::RefreshExpiresAt.lt(now))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

#[cfg(test)]
mod tests {
    use super::{
        create, delete_expired, find_by_previous_refresh_jti, find_by_refresh_jti, list_by_user,
        revoke_by_refresh_jti, rotate_refresh,
    };
    use crate::config::DatabaseConfig;
    use crate::db::repository::user_repo;
    use aster_yggdrasil_model::entities::auth_session;
    use aster_yggdrasil_model::types::user::UserRole;
    use chrono::{Duration, Utc};
    use sea_orm::{ActiveValue::Set, EntityTrait};

    async fn build_test_db() -> sea_orm::DatabaseConnection {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".into(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .expect("auth session repo test DB should connect");
        aster_yggdrasil_migration::Migrator::up(&db, None)
            .await
            .expect("auth session repo test migrations should succeed");
        db
    }

    async fn insert_user(db: &sea_orm::DatabaseConnection, suffix: &str) -> i64 {
        user_repo::create(
            db,
            &format!("session-user-{suffix}"),
            &format!("session-user-{suffix}@example.com"),
            "password-hash",
            UserRole::User,
        )
        .await
        .expect("auth session test user should insert")
        .id
    }

    fn session_model(
        id: &str,
        user_id: i64,
        refresh_jti: &str,
        expires_at: chrono::DateTime<Utc>,
    ) -> auth_session::ActiveModel {
        let now = Utc::now();
        auth_session::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(user_id),
            current_refresh_jti: Set(refresh_jti.to_string()),
            previous_refresh_jti: Set(None),
            refresh_expires_at: Set(expires_at),
            user_agent: Set(Some("Firefox".to_string())),
            ip_address: Set(Some("127.0.0.1".to_string())),
            created_at: Set(now),
            last_seen_at: Set(now),
            revoked_at: Set(None),
        }
    }

    #[tokio::test]
    async fn create_find_rotate_list_and_revoke_session_by_refresh_jti() {
        let db = build_test_db().await;
        let user_id = insert_user(&db, "flow").await;
        let expires_at = Utc::now() + Duration::hours(1);

        let first = create(
            &db,
            session_model("session-one", user_id, "jti-one", expires_at),
        )
        .await
        .unwrap();
        let second = create(
            &db,
            session_model("session-two", user_id, "jti-two", expires_at),
        )
        .await
        .unwrap();

        let active = find_by_refresh_jti(&db, "jti-one").await.unwrap().unwrap();
        assert_eq!(active.id, first.id);
        assert_eq!(active.user_agent.as_deref(), Some("Firefox"));
        assert_eq!(active.ip_address.as_deref(), Some("127.0.0.1"));

        assert!(
            rotate_refresh(
                &db,
                "jti-one",
                "jti-three",
                expires_at,
                Some("127.0.0.2"),
                Some("Safari"),
                Utc::now(),
            )
            .await
            .unwrap()
        );
        assert!(find_by_refresh_jti(&db, "jti-one").await.unwrap().is_none());
        assert!(
            find_by_previous_refresh_jti(&db, "jti-one")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            find_by_refresh_jti(&db, "jti-three")
                .await
                .unwrap()
                .is_some()
        );

        let sessions = list_by_user(&db, user_id).await.unwrap();
        let session_ids = sessions
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        assert_eq!(session_ids.len(), 2);
        assert!(session_ids.contains(&first.id));
        assert!(session_ids.contains(&second.id));

        assert!(revoke_by_refresh_jti(&db, "jti-three").await.unwrap());
        assert!(!revoke_by_refresh_jti(&db, "jti-three").await.unwrap());
        assert!(!revoke_by_refresh_jti(&db, "missing-jti").await.unwrap());

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn delete_expired_removes_only_expired_sessions() {
        let db = build_test_db().await;
        let user_id = insert_user(&db, "cleanup").await;
        let now = Utc::now();
        let expired = create(
            &db,
            session_model(
                "expired-session",
                user_id,
                "expired-jti",
                now - Duration::seconds(1),
            ),
        )
        .await
        .unwrap();
        let active = create(
            &db,
            session_model(
                "active-session",
                user_id,
                "active-jti",
                now + Duration::hours(1),
            ),
        )
        .await
        .unwrap();

        assert_eq!(delete_expired(&db, now).await.unwrap(), 1);
        let sessions = list_by_user(&db, user_id).await.unwrap();
        assert_eq!(
            sessions
                .into_iter()
                .map(|session| session.id)
                .collect::<Vec<_>>(),
            vec![active.id.clone()]
        );
        assert!(
            auth_session::Entity::find_by_id(expired.id)
                .one(&db)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            auth_session::Entity::find_by_id(active.id)
                .one(&db)
                .await
                .unwrap()
                .is_some()
        );

        db.close().await.unwrap();
    }
}
