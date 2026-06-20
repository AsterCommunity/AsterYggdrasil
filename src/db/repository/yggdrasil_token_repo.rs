//! Yggdrasil token repository.

use crate::entities::yggdrasil_token::{self, Entity as YggdrasilToken};
use crate::errors::{AsterError, MapAsterErr, Result};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};

pub struct CreateYggdrasilToken<'a> {
    pub user_id: i64,
    pub access_token_hash: &'a str,
    pub client_token: &'a str,
    pub selected_profile_id: Option<i64>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    input: CreateYggdrasilToken<'_>,
) -> Result<yggdrasil_token::Model> {
    yggdrasil_token::ActiveModel {
        user_id: Set(input.user_id),
        access_token_hash: Set(input.access_token_hash.to_string()),
        client_token: Set(input.client_token.to_string()),
        selected_profile_id: Set(input.selected_profile_id),
        issued_at: Set(input.issued_at),
        expires_at: Set(input.expires_at),
        revoked_at: Set(None),
        temporarily_invalidated_at: Set(None),
        user_agent: Set(input.user_agent),
        ip_address: Set(input.ip_address),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_access_hash<C: ConnectionTrait>(
    db: &C,
    access_token_hash: &str,
) -> Result<Option<yggdrasil_token::Model>> {
    YggdrasilToken::find()
        .filter(yggdrasil_token::Column::AccessTokenHash.eq(access_token_hash))
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn list_active_hashes_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Vec<String>> {
    let tokens = YggdrasilToken::find()
        .filter(yggdrasil_token::Column::UserId.eq(user_id))
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(tokens
        .into_iter()
        .map(|token| token.access_token_hash)
        .collect())
}

pub async fn list_active_hashes_for_selected_profile<C: ConnectionTrait>(
    db: &C,
    selected_profile_id: i64,
) -> Result<Vec<String>> {
    let tokens = YggdrasilToken::find()
        .filter(yggdrasil_token::Column::SelectedProfileId.eq(selected_profile_id))
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(tokens
        .into_iter()
        .map(|token| token.access_token_hash)
        .collect())
}

pub async fn count_active<C: ConnectionTrait>(db: &C) -> Result<u64> {
    YggdrasilToken::find()
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .filter(yggdrasil_token::Column::TemporarilyInvalidatedAt.is_null())
        .filter(yggdrasil_token::Column::ExpiresAt.gt(Utc::now()))
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn revoke_by_access_hash<C: ConnectionTrait>(
    db: &C,
    access_token_hash: &str,
) -> Result<bool> {
    let Some(token) = find_by_access_hash(db, access_token_hash).await? else {
        return Ok(false);
    };
    if token.revoked_at.is_some() {
        return Ok(false);
    }

    let mut active: yggdrasil_token::ActiveModel = token.into();
    active.revoked_at = Set(Some(Utc::now()));
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(true)
}

pub async fn revoke_all_for_user<C: ConnectionTrait>(db: &C, user_id: i64) -> Result<u64> {
    let now = Utc::now();
    let result = YggdrasilToken::update_many()
        .col_expr(
            yggdrasil_token::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(yggdrasil_token::Column::UserId.eq(user_id))
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

pub async fn revoke_all_for_selected_profile<C: ConnectionTrait>(
    db: &C,
    selected_profile_id: i64,
) -> Result<u64> {
    let now = Utc::now();
    let result = YggdrasilToken::update_many()
        .col_expr(
            yggdrasil_token::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(yggdrasil_token::Column::SelectedProfileId.eq(selected_profile_id))
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

pub async fn temporarily_invalidate_all_for_selected_profile<C: ConnectionTrait>(
    db: &C,
    selected_profile_id: i64,
) -> Result<u64> {
    let now = Utc::now();
    let result = YggdrasilToken::update_many()
        .col_expr(
            yggdrasil_token::Column::TemporarilyInvalidatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(yggdrasil_token::Column::SelectedProfileId.eq(selected_profile_id))
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .filter(yggdrasil_token::Column::TemporarilyInvalidatedAt.is_null())
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

pub async fn delete_expired<C: ConnectionTrait>(db: &C, now: DateTime<Utc>) -> Result<u64> {
    let result = YggdrasilToken::delete_many()
        .filter(yggdrasil_token::Column::ExpiresAt.lt(now))
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

pub async fn delete_expired_or_revoked<C: ConnectionTrait>(
    db: &C,
    now: DateTime<Utc>,
) -> Result<u64> {
    let result = YggdrasilToken::delete_many()
        .filter(
            Condition::any()
                .add(yggdrasil_token::Column::ExpiresAt.lt(now))
                .add(yggdrasil_token::Column::RevokedAt.is_not_null()),
        )
        .exec(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(result.rows_affected)
}

pub async fn prune_oldest_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    keep_count: u64,
) -> Result<Vec<String>> {
    let keep_count = crate::utils::numbers::u64_to_usize(keep_count, "yggdrasil token keep count")?;
    let tokens = YggdrasilToken::find()
        .filter(yggdrasil_token::Column::UserId.eq(user_id))
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .order_by_desc(yggdrasil_token::Column::IssuedAt)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    let mut pruned_hashes = Vec::new();
    for token in tokens.into_iter().skip(keep_count) {
        pruned_hashes.push(token.access_token_hash.clone());
        let mut active: yggdrasil_token::ActiveModel = token.into();
        active.revoked_at = Set(Some(Utc::now()));
        active
            .update(db)
            .await
            .map_aster_err(AsterError::database_operation)?;
    }

    Ok(pruned_hashes)
}

#[cfg(test)]
mod tests {
    use super::{CreateYggdrasilToken, create, delete_expired_or_revoked, revoke_by_access_hash};
    use crate::config::DatabaseConfig;
    use crate::db::repository::user_repo;
    use crate::types::UserRole;
    use chrono::{Duration, Utc};

    async fn build_test_db() -> sea_orm::DatabaseConnection {
        let db = crate::db::connect_with_metrics(
            &DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            crate::metrics_core::NoopMetrics::arc(),
        )
        .await
        .expect("yggdrasil token repo test DB should connect");
        migration::Migrator::up(&db, None)
            .await
            .expect("yggdrasil token repo test migrations should succeed");
        db
    }

    async fn insert_user(db: &sea_orm::DatabaseConnection) -> i64 {
        user_repo::create(
            db,
            "ygg-token-cleanup-user",
            "ygg-token-cleanup@example.com",
            "password-hash",
            UserRole::User,
        )
        .await
        .expect("yggdrasil token cleanup test user should insert")
        .id
    }

    async fn insert_token(
        db: &sea_orm::DatabaseConnection,
        user_id: i64,
        hash: &str,
        expires_at: chrono::DateTime<Utc>,
    ) {
        let now = Utc::now();
        create(
            db,
            CreateYggdrasilToken {
                user_id,
                access_token_hash: hash,
                client_token: hash,
                selected_profile_id: None,
                issued_at: now,
                expires_at,
                user_agent: None,
                ip_address: None,
            },
        )
        .await
        .expect("yggdrasil token cleanup test token should insert");
    }

    #[tokio::test]
    async fn delete_expired_or_revoked_removes_only_unusable_tokens() {
        let db = build_test_db().await;
        let user_id = insert_user(&db).await;
        let now = Utc::now();
        insert_token(&db, user_id, "expired", now - Duration::seconds(1)).await;
        insert_token(&db, user_id, "revoked", now + Duration::hours(1)).await;
        insert_token(&db, user_id, "active", now + Duration::hours(1)).await;
        revoke_by_access_hash(&db, "revoked").await.unwrap();

        let removed = delete_expired_or_revoked(&db, now).await.unwrap();

        assert_eq!(removed, 2);
        assert!(
            super::find_by_access_hash(&db, "expired")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            super::find_by_access_hash(&db, "revoked")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            super::find_by_access_hash(&db, "active")
                .await
                .unwrap()
                .is_some()
        );
    }
}
