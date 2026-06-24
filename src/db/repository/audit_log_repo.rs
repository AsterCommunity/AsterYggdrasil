//! Audit log repository.

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, sea_query::Expr,
};

use crate::api::pagination::CursorSlice;
use crate::entities::audit_log::{self, Entity as AuditLog};
use crate::errors::{AsterError, Result};
use crate::types::AuditAction;

pub struct AuditLogQuery<'a> {
    pub user_id: Option<i64>,
    pub action: Option<&'a str>,
    pub entity_type: Option<&'a str>,
    pub entity_id: Option<i64>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub limit: u64,
    pub cursor: Option<(DateTime<Utc>, i64)>,
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    model: audit_log::ActiveModel,
) -> Result<audit_log::Model> {
    model.insert(db).await.map_err(AsterError::from)
}

pub async fn create_many<C: ConnectionTrait>(
    db: &C,
    models: Vec<audit_log::ActiveModel>,
) -> Result<()> {
    if models.is_empty() {
        return Ok(());
    }
    AuditLog::insert_many(models)
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(())
}

pub async fn find_with_filters_cursor<C: ConnectionTrait>(
    db: &C,
    query: AuditLogQuery<'_>,
) -> Result<CursorSlice<audit_log::Model>> {
    let mut q = AuditLog::find();
    let limit = query.limit.clamp(1, 200);

    if let Some(user_id) = query.user_id {
        q = q.filter(audit_log::Column::UserId.eq(user_id));
    }
    if let Some(action) = query.action {
        q = q.filter(audit_log::Column::Action.eq(action));
    }
    if let Some(entity_type) = query.entity_type {
        q = q.filter(audit_log::Column::EntityType.eq(entity_type));
    }
    if let Some(entity_id) = query.entity_id {
        q = q.filter(audit_log::Column::EntityId.eq(entity_id));
    }
    if let Some(after) = query.after {
        q = q.filter(audit_log::Column::CreatedAt.gte(after));
    }
    if let Some(before) = query.before {
        q = q.filter(audit_log::Column::CreatedAt.lte(before));
    }

    let total = q.clone().count(db).await.map_err(AsterError::from)?;
    if let Some((created_at, id)) = query.cursor {
        q = q.filter(
            Condition::any()
                .add(audit_log::Column::CreatedAt.lt(created_at))
                .add(
                    Condition::all()
                        .add(audit_log::Column::CreatedAt.eq(created_at))
                        .add(audit_log::Column::Id.lt(id)),
                ),
        );
    }

    let items = q
        .order_by_desc(audit_log::Column::CreatedAt)
        .order_by_desc(audit_log::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_err(AsterError::from)?;
    CursorSlice::from_overfetch(
        items,
        total,
        limit,
        "audit log page size",
        "audit log cursor limit",
    )
}

pub async fn count_created_between<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<u64> {
    AuditLog::find()
        .filter(audit_log::Column::CreatedAt.gte(start))
        .filter(audit_log::Column::CreatedAt.lt(end))
        .count(db)
        .await
        .map_err(AsterError::from)
}

pub async fn count_created_between_with_actions<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    actions: &[AuditAction],
) -> Result<u64> {
    if actions.is_empty() {
        return Ok(0);
    }

    AuditLog::find()
        .filter(audit_log::Column::CreatedAt.gte(start))
        .filter(audit_log::Column::CreatedAt.lt(end))
        .filter(audit_log::Column::Action.is_in(actions.iter().map(|action| action.as_str())))
        .count(db)
        .await
        .map_err(AsterError::from)
}

pub async fn count_distinct_users_created_between_with_actions<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    actions: &[AuditAction],
) -> Result<u64> {
    if actions.is_empty() {
        return Ok(0);
    }

    let count = AuditLog::find()
        .select_only()
        .column_as(
            Expr::col(audit_log::Column::UserId).count_distinct(),
            "distinct_user_count",
        )
        .filter(audit_log::Column::CreatedAt.gte(start))
        .filter(audit_log::Column::CreatedAt.lt(end))
        .filter(audit_log::Column::Action.is_in(actions.iter().map(|action| action.as_str())))
        .filter(audit_log::Column::UserId.gt(0))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(AsterError::from)?
        .unwrap_or(0);

    crate::utils::numbers::i64_to_u64(count, "distinct audit log user count")
}

pub async fn delete_before<C: ConnectionTrait>(db: &C, before: DateTime<Utc>) -> Result<u64> {
    let result = AuditLog::delete_many()
        .filter(audit_log::Column::CreatedAt.lt(before))
        .exec(db)
        .await
        .map_err(AsterError::from)?;
    Ok(result.rows_affected)
}

#[cfg(test)]
mod tests {
    use super::{
        AuditLogQuery, count_distinct_users_created_between_with_actions, create, create_many,
        delete_before, find_with_filters_cursor,
    };
    use crate::config::DatabaseConfig;
    use crate::entities::audit_log;
    use crate::types::{AuditAction, AuditEntityType};
    use chrono::{Duration, Utc};
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

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
        .expect("audit log repo test DB should connect");
        migration::Migrator::up(&db, None)
            .await
            .expect("audit log repo test migrations should succeed");
        db
    }

    fn audit_model(
        user_id: i64,
        action: AuditAction,
        entity_type: AuditEntityType,
        entity_id: Option<i64>,
        entity_name: &str,
        ip_address: &str,
        created_at: chrono::DateTime<Utc>,
    ) -> audit_log::ActiveModel {
        audit_log::ActiveModel {
            id: Default::default(),
            user_id: Set(user_id),
            action: Set(action),
            entity_type: Set(entity_type.as_str().to_string()),
            entity_id: Set(entity_id),
            entity_name: Set(Some(entity_name.to_string())),
            details: Set(Some(
                serde_json::json!({
                    "entity_name": entity_name,
                    "action": action.as_str(),
                })
                .to_string(),
            )),
            ip_address: Set(Some(ip_address.to_string())),
            user_agent: Set(Some("audit-repo-test".to_string())),
            created_at: Set(created_at),
        }
    }

    async fn all_audit_count(db: &sea_orm::DatabaseConnection) -> u64 {
        audit_log::Entity::find()
            .count(db)
            .await
            .expect("audit log count should succeed")
    }

    #[tokio::test]
    async fn create_and_empty_create_many_follow_repository_contract() {
        let db = build_test_db().await;
        create_many(&db, Vec::new()).await.unwrap();
        assert_eq!(all_audit_count(&db).await, 0);

        let created_at = Utc::now();
        let entry = create(
            &db,
            audit_model(
                42,
                AuditAction::UserLogin,
                AuditEntityType::AuthSession,
                Some(7),
                "admin",
                "127.0.0.1",
                created_at,
            ),
        )
        .await
        .unwrap();

        assert!(entry.id > 0);
        assert_eq!(entry.user_id, 42);
        assert_eq!(entry.action, AuditAction::UserLogin);
        assert_eq!(entry.entity_type, "auth_session");
        assert_eq!(entry.entity_id, Some(7));
        assert_eq!(entry.entity_name.as_deref(), Some("admin"));
        assert_eq!(entry.ip_address.as_deref(), Some("127.0.0.1"));
        assert_eq!(all_audit_count(&db).await, 1);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn count_distinct_users_created_between_with_actions_excludes_duplicates_and_system_user()
    {
        let db = build_test_db().await;
        let base = Utc::now();
        create_many(
            &db,
            vec![
                audit_model(
                    10,
                    AuditAction::UserLogin,
                    AuditEntityType::AuthSession,
                    Some(1),
                    "login-1",
                    "192.0.2.10",
                    base,
                ),
                audit_model(
                    10,
                    AuditAction::UserRefreshToken,
                    AuditEntityType::AuthSession,
                    Some(2),
                    "refresh-duplicate",
                    "192.0.2.10",
                    base + Duration::seconds(1),
                ),
                audit_model(
                    20,
                    AuditAction::MinecraftTextureUpload,
                    AuditEntityType::MinecraftTexture,
                    Some(3),
                    "texture",
                    "192.0.2.20",
                    base + Duration::seconds(2),
                ),
                audit_model(
                    0,
                    AuditAction::UserLogin,
                    AuditEntityType::AuthSession,
                    Some(4),
                    "system-context",
                    "192.0.2.30",
                    base + Duration::seconds(3),
                ),
                audit_model(
                    30,
                    AuditAction::ConfigUpdate,
                    AuditEntityType::SystemConfig,
                    Some(5),
                    "config",
                    "192.0.2.40",
                    base + Duration::seconds(4),
                ),
            ],
        )
        .await
        .unwrap();

        let count = count_distinct_users_created_between_with_actions(
            &db,
            base - Duration::seconds(1),
            base + Duration::seconds(10),
            &[
                AuditAction::UserLogin,
                AuditAction::UserRefreshToken,
                AuditAction::MinecraftTextureUpload,
            ],
        )
        .await
        .unwrap();

        assert_eq!(count, 2);
        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn find_with_filters_applies_all_supported_predicates() {
        let db = build_test_db().await;
        let base = Utc::now();
        let ignored_by_action = create(
            &db,
            audit_model(
                10,
                AuditAction::UserLogin,
                AuditEntityType::AuthSession,
                Some(1),
                "login",
                "192.0.2.10",
                base - Duration::seconds(30),
            ),
        )
        .await
        .unwrap();
        let expected = create(
            &db,
            audit_model(
                10,
                AuditAction::ConfigUpdate,
                AuditEntityType::SystemConfig,
                Some(2),
                "branding_title",
                "192.0.2.11",
                base - Duration::seconds(20),
            ),
        )
        .await
        .unwrap();
        let ignored_by_user = create(
            &db,
            audit_model(
                20,
                AuditAction::ConfigUpdate,
                AuditEntityType::SystemConfig,
                Some(2),
                "branding_title",
                "192.0.2.12",
                base - Duration::seconds(20),
            ),
        )
        .await
        .unwrap();
        let ignored_by_time = create(
            &db,
            audit_model(
                10,
                AuditAction::ConfigUpdate,
                AuditEntityType::SystemConfig,
                Some(2),
                "late",
                "192.0.2.13",
                base - Duration::seconds(5),
            ),
        )
        .await
        .unwrap();

        let page = find_with_filters_cursor(
            &db,
            AuditLogQuery {
                user_id: Some(10),
                action: Some(AuditAction::ConfigUpdate.as_str()),
                entity_type: Some(AuditEntityType::SystemConfig.as_str()),
                entity_id: Some(2),
                after: Some(base - Duration::seconds(25)),
                before: Some(base - Duration::seconds(15)),
                limit: 10,
                cursor: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(page.total, 1);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, expected.id);
        assert_ne!(page.items[0].id, ignored_by_action.id);
        assert_ne!(page.items[0].id, ignored_by_user.id);
        assert_ne!(page.items[0].id, ignored_by_time.id);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn find_with_filters_pages_by_created_at_cursor() {
        let db = build_test_db().await;
        let base = Utc::now();
        let first = create(
            &db,
            audit_model(
                20,
                AuditAction::UserLogout,
                AuditEntityType::User,
                Some(20),
                "gamma",
                "192.0.2.3",
                base + Duration::seconds(30),
            ),
        )
        .await
        .unwrap();
        let second = create(
            &db,
            audit_model(
                10,
                AuditAction::ConfigUpdate,
                AuditEntityType::SystemConfig,
                Some(10),
                "alpha",
                "192.0.2.1",
                base + Duration::seconds(10),
            ),
        )
        .await
        .unwrap();
        let third = create(
            &db,
            audit_model(
                30,
                AuditAction::AdminCleanupTasks,
                AuditEntityType::Task,
                Some(30),
                "beta",
                "192.0.2.2",
                base + Duration::seconds(20),
            ),
        )
        .await
        .unwrap();

        let first_page = find_with_filters_cursor(
            &db,
            AuditLogQuery {
                user_id: None,
                action: None,
                entity_type: None,
                entity_id: None,
                after: None,
                before: None,
                limit: 2,
                cursor: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            first_page
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![first.id, third.id]
        );
        assert_eq!(first_page.total, 3);
        assert!(first_page.has_more);

        let last = first_page.items.last().unwrap();
        let second_page = find_with_filters_cursor(
            &db,
            AuditLogQuery {
                user_id: None,
                action: None,
                entity_type: None,
                entity_id: None,
                after: None,
                before: None,
                limit: 2,
                cursor: Some((last.created_at, last.id)),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            second_page
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![second.id]
        );
        assert!(!second_page.has_more);

        db.close().await.unwrap();
    }

    #[tokio::test]
    async fn delete_before_keeps_cutoff_and_cursor_list_orders_recent_entries() {
        let db = build_test_db().await;
        let cutoff = Utc::now();
        let old = create(
            &db,
            audit_model(
                1,
                AuditAction::ServerStart,
                AuditEntityType::System,
                None,
                "old",
                "192.0.2.1",
                cutoff - Duration::seconds(1),
            ),
        )
        .await
        .unwrap();
        let at_cutoff = create(
            &db,
            audit_model(
                1,
                AuditAction::ServerShutdown,
                AuditEntityType::System,
                None,
                "cutoff",
                "192.0.2.2",
                cutoff,
            ),
        )
        .await
        .unwrap();
        let recent = create(
            &db,
            audit_model(
                1,
                AuditAction::ConfigDelete,
                AuditEntityType::SystemConfig,
                Some(3),
                "recent",
                "192.0.2.3",
                cutoff + Duration::seconds(1),
            ),
        )
        .await
        .unwrap();

        let page = find_with_filters_cursor(
            &db,
            AuditLogQuery {
                user_id: None,
                action: None,
                entity_type: None,
                entity_id: None,
                after: None,
                before: None,
                limit: 2,
                cursor: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(page.total, 3);
        assert!(page.has_more);
        assert_eq!(
            page.items
                .into_iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![recent.id, at_cutoff.id]
        );

        assert_eq!(delete_before(&db, cutoff).await.unwrap(), 1);
        assert!(
            audit_log::Entity::find()
                .filter(audit_log::Column::Id.eq(old.id))
                .one(&db)
                .await
                .unwrap()
                .is_none()
        );
        let remaining = find_with_filters_cursor(
            &db,
            AuditLogQuery {
                user_id: None,
                action: None,
                entity_type: None,
                entity_id: None,
                after: None,
                before: None,
                limit: 10,
                cursor: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(remaining.total, 2);
        assert_eq!(
            remaining
                .items
                .into_iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![recent.id, at_cutoff.id]
        );

        db.close().await.unwrap();
    }
}
