//! Audit log repository.

use chrono::{DateTime, Utc};
use sea_orm::ConnectionTrait;

use crate::entities::audit_log;
use crate::errors::{AsterError, Result};
use crate::types::audit::AuditAction;
use aster_forge_api::CursorSlice;

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

pub async fn find_with_filters_cursor<C: ConnectionTrait>(
    db: &C,
    query: AuditLogQuery<'_>,
) -> Result<CursorSlice<audit_log::Model>> {
    let limit = query.limit.clamp(1, 200);
    let page = aster_forge_db::find_audit_logs_with_filters_cursor(
        db,
        aster_forge_db::AuditLogQuery {
            user_id: query.user_id,
            action: query.action,
            entity_type: query.entity_type,
            entity_id: query.entity_id,
            after: query.after,
            before: query.before,
            limit,
            cursor: query.cursor,
        },
    )
    .await?;

    Ok(CursorSlice {
        items: page
            .items
            .into_iter()
            .map(audit_log::Model::try_from)
            .collect::<Result<Vec<_>>>()?,
        total: page.total,
        has_more: page.has_more,
    })
}

pub async fn count_created_between<C: ConnectionTrait>(
    db: &C,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<u64> {
    aster_forge_db::count_audit_logs_created_between(db, start, end)
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

    let actions = action_wire_values(actions);
    aster_forge_db::count_audit_logs_created_between_with_actions(db, start, end, &actions)
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

    let actions = action_wire_values(actions);
    aster_forge_db::count_distinct_audit_log_users_created_between_with_actions(
        db, start, end, &actions,
    )
    .await
    .map_err(AsterError::from)
}

pub async fn delete_before<C: ConnectionTrait>(db: &C, before: DateTime<Utc>) -> Result<u64> {
    aster_forge_db::delete_audit_logs_before(db, before)
        .await
        .map_err(AsterError::from)
}

fn action_wire_values(actions: &[AuditAction]) -> Vec<&str> {
    actions.iter().map(|action| action.as_str()).collect()
}

impl TryFrom<aster_forge_db::audit_log::Model> for audit_log::Model {
    type Error = AsterError;

    fn try_from(value: aster_forge_db::audit_log::Model) -> Result<Self> {
        let action = AuditAction::from_str_name(&value.action).ok_or_else(|| {
            AsterError::database_operation(format!(
                "unsupported audit action in audit log row {}: {}",
                value.id, value.action
            ))
        })?;

        Ok(Self {
            id: value.id,
            user_id: value.user_id,
            action,
            entity_type: value.entity_type,
            entity_id: value.entity_id,
            entity_name: value.entity_name,
            details: value.details,
            ip_address: value.ip_address,
            user_agent: value.user_agent,
            created_at: value.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    use super::{
        AuditLogQuery, count_distinct_users_created_between_with_actions, delete_before,
        find_with_filters_cursor,
    };
    use crate::config::DatabaseConfig;
    use crate::entities::audit_log;
    use crate::types::audit::{AuditAction, AuditEntityType};
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
        .expect("audit log repo test DB should connect");
        migration::Migrator::up(&db, None)
            .await
            .expect("audit log repo test migrations should succeed");
        db
    }

    fn audit_request(
        user_id: i64,
        action: AuditAction,
        entity_type: AuditEntityType,
        entity_id: Option<i64>,
        entity_name: &str,
        ip_address: &str,
        created_at: chrono::DateTime<Utc>,
    ) -> aster_forge_db::AuditLogCreate {
        aster_forge_db::AuditLogCreate {
            user_id,
            action: action.as_str().to_string(),
            entity_type: entity_type.as_str().to_string(),
            entity_id,
            entity_name: Some(entity_name.to_string()),
            details: Some(
                serde_json::json!({
                    "entity_name": entity_name,
                    "action": action.as_str(),
                })
                .to_string(),
            ),
            ip_address: Some(ip_address.to_string()),
            user_agent: Some("audit-repo-test".to_string()),
            created_at,
        }
    }

    async fn all_audit_count(db: &sea_orm::DatabaseConnection) -> u64 {
        audit_log::Entity::find()
            .count(db)
            .await
            .expect("audit log count should succeed")
    }

    #[tokio::test]
    async fn forge_create_requests_are_visible_through_product_query_facade() {
        let db = build_test_db().await;
        aster_forge_db::create_audit_log_requests(&db, Vec::new())
            .await
            .unwrap();
        assert_eq!(all_audit_count(&db).await, 0);

        let created_at = Utc::now();
        let entry = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        assert_eq!(entry.action, AuditAction::UserLogin.as_str());
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
        aster_forge_db::create_audit_log_requests(
            &db,
            vec![
                audit_request(
                    10,
                    AuditAction::UserLogin,
                    AuditEntityType::AuthSession,
                    Some(1),
                    "login-1",
                    "192.0.2.10",
                    base,
                ),
                audit_request(
                    10,
                    AuditAction::UserRefreshToken,
                    AuditEntityType::AuthSession,
                    Some(2),
                    "refresh-duplicate",
                    "192.0.2.10",
                    base + Duration::seconds(1),
                ),
                audit_request(
                    20,
                    AuditAction::MinecraftTextureUpload,
                    AuditEntityType::MinecraftTexture,
                    Some(3),
                    "texture",
                    "192.0.2.20",
                    base + Duration::seconds(2),
                ),
                audit_request(
                    0,
                    AuditAction::UserLogin,
                    AuditEntityType::AuthSession,
                    Some(4),
                    "system-context",
                    "192.0.2.30",
                    base + Duration::seconds(3),
                ),
                audit_request(
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
        let ignored_by_action = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let expected = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let ignored_by_user = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let ignored_by_time = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let first = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let second = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let third = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let old = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let at_cutoff = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
        let recent = aster_forge_db::create_audit_log_row(
            &db,
            audit_request(
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
