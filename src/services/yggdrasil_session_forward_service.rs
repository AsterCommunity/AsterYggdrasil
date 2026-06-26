//! Admin service for Yggdrasil session forwarding servers.

use chrono::{DateTime, Utc};
use sea_orm::{ActiveValue::Set, IntoActiveModel};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::db::repository::yggdrasil_session_forward_server_repo;
use crate::entities::yggdrasil_session_forward_server;
use crate::errors::{AsterError, Result};
use crate::runtime::{CacheRuntimeState, DatabaseRuntimeState};
use crate::services::audit_service;
use crate::types::{
    YggdrasilSessionForwardEndpointKind, YggdrasilSessionForwardProviderKind,
    YggdrasilSessionForwardServerSortBy,
};
use aster_forge_api::{CursorPage, EnabledPriorityIdCursor, IdCursor};

const MIN_PRIORITY: i32 = -10_000;
const MAX_PRIORITY: i32 = 10_000;
const MIN_WEIGHT: i32 = 1;
const MAX_WEIGHT: i32 = 1_000;
const MIN_TIMEOUT_MS: i32 = 100;
const MAX_TIMEOUT_MS: i32 = 10_000;
const LOCAL_DISPLAY_NAME: &str = "AsterYggdrasil";
const MOJANG_DISPLAY_NAME: &str = "Mojang";
const MOJANG_SESSION_SERVER_BASE_URL: &str = "https://sessionserver.mojang.com";

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AdminYggdrasilSessionForwardServerInfo {
    pub id: i64,
    pub display_name: String,
    pub provider_kind: YggdrasilSessionForwardProviderKind,
    pub endpoint_kind: YggdrasilSessionForwardEndpointKind,
    pub base_url: Option<String>,
    pub builtin: bool,
    pub enabled: bool,
    pub priority: i32,
    pub weight: i32,
    pub timeout_ms: i32,
    pub texture_forward_enabled: bool,
    pub local: bool,
    pub deletable: bool,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_failure_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateYggdrasilSessionForwardServerInput {
    pub display_name: String,
    pub base_url: String,
    pub endpoint_kind: Option<YggdrasilSessionForwardEndpointKind>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub weight: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub texture_forward_enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateYggdrasilSessionForwardServerInput {
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub endpoint_kind: Option<YggdrasilSessionForwardEndpointKind>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub weight: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub texture_forward_enabled: Option<bool>,
}

pub async fn ensure_builtin_servers(db: &sea_orm::DatabaseConnection) -> Result<usize> {
    let mut inserted = 0;

    if yggdrasil_session_forward_server_repo::find_local(db)
        .await?
        .is_none()
    {
        yggdrasil_session_forward_server_repo::create(db, local_builtin_server()).await?;
        inserted += 1;
    }

    if yggdrasil_session_forward_server_repo::find_by_base_url(db, MOJANG_SESSION_SERVER_BASE_URL)
        .await?
        .is_none()
    {
        yggdrasil_session_forward_server_repo::create(db, mojang_builtin_server()).await?;
        inserted += 1;
    }

    Ok(inserted)
}

pub async fn prepare_runtime_session_forward_servers(
    db: &sea_orm::DatabaseConnection,
) -> Result<usize> {
    ensure_builtin_servers(db).await
}

pub async fn list_servers<S>(
    state: &S,
    limit: u64,
    after_id: Option<i64>,
    after_call_order: Option<(bool, i32, i64)>,
    sort_by: YggdrasilSessionForwardServerSortBy,
) -> Result<CursorPage<AdminYggdrasilSessionForwardServerInfo, SessionForwardServerCursor>>
where
    S: DatabaseRuntimeState,
{
    let limit = limit.clamp(1, 100);
    let repo_cursor = match sort_by {
        YggdrasilSessionForwardServerSortBy::CallOrder => {
            after_call_order.map(|(enabled, priority, id)| {
                yggdrasil_session_forward_server_repo::SessionForwardServerCursor::CallOrder {
                    enabled,
                    priority,
                    id,
                }
            })
        }
        YggdrasilSessionForwardServerSortBy::Id => {
            after_id.map(yggdrasil_session_forward_server_repo::SessionForwardServerCursor::Id)
        }
    };
    let page = yggdrasil_session_forward_server_repo::find_cursor(
        state.reader_db(),
        limit,
        repo_cursor,
        sort_by,
    )
    .await?;
    let next_cursor = if page.has_more {
        page.items.last().map(|server| match sort_by {
            YggdrasilSessionForwardServerSortBy::CallOrder => {
                SessionForwardServerCursor::CallOrder(EnabledPriorityIdCursor {
                    enabled: server.enabled,
                    priority: server.priority,
                    id: server.id,
                })
            }
            YggdrasilSessionForwardServerSortBy::Id => {
                SessionForwardServerCursor::Id(IdCursor { id: server.id })
            }
        })
    } else {
        None
    };
    Ok(CursorPage::new(
        page.items.into_iter().map(server_to_admin).collect(),
        page.total,
        limit,
        next_cursor,
    ))
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum SessionForwardServerCursor {
    CallOrder(EnabledPriorityIdCursor),
    Id(IdCursor),
}

pub async fn get_server<S>(state: &S, id: i64) -> Result<AdminYggdrasilSessionForwardServerInfo>
where
    S: DatabaseRuntimeState,
{
    let server = yggdrasil_session_forward_server_repo::find_by_id(state.reader_db(), id).await?;
    Ok(server_to_admin(server))
}

pub async fn create_server<S>(
    state: &S,
    input: CreateYggdrasilSessionForwardServerInput,
) -> Result<AdminYggdrasilSessionForwardServerInfo>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let display_name = normalize_display_name(input.display_name)?;
    let base_url = normalize_base_url(input.base_url)?;
    let endpoint_kind = input
        .endpoint_kind
        .unwrap_or(YggdrasilSessionForwardEndpointKind::AuthlibInjector);
    ensure_base_url_available(state, None, &base_url).await?;
    let priority = normalize_priority(input.priority.unwrap_or(100))?;
    let weight = normalize_weight(input.weight.unwrap_or(1))?;
    let timeout_ms = normalize_timeout_ms(input.timeout_ms.unwrap_or(1500))?;
    let now = Utc::now();

    let server = yggdrasil_session_forward_server::ActiveModel {
        display_name: Set(display_name),
        provider_kind: Set(YggdrasilSessionForwardProviderKind::Remote),
        endpoint_kind: Set(endpoint_kind),
        base_url: Set(Some(base_url)),
        builtin: Set(false),
        enabled: Set(input.enabled.unwrap_or(true)),
        priority: Set(priority),
        weight: Set(weight),
        timeout_ms: Set(timeout_ms),
        texture_forward_enabled: Set(input.texture_forward_enabled.unwrap_or(false)),
        last_checked_at: Set(None),
        last_success_at: Set(None),
        last_failure_at: Set(None),
        last_failure_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let server = yggdrasil_session_forward_server_repo::create(state.writer_db(), server).await?;
    crate::services::yggdrasil_service::invalidate_session_forward_server_cache(state).await;
    Ok(server_to_admin(server))
}

pub async fn update_server<S>(
    state: &S,
    id: i64,
    input: UpdateYggdrasilSessionForwardServerInput,
) -> Result<AdminYggdrasilSessionForwardServerInfo>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let existing = yggdrasil_session_forward_server_repo::find_by_id(state.writer_db(), id).await?;
    let provider_kind = existing.provider_kind;
    let builtin = existing.builtin;
    let mut active = existing.into_active_model();

    if let Some(display_name) = input.display_name {
        active.display_name = Set(normalize_display_name(display_name)?);
    }
    if let Some(base_url) = input.base_url {
        if builtin {
            return Err(AsterError::validation_error(
                "built-in Yggdrasil forwarding server base_url cannot be changed",
            ));
        }
        let base_url = normalize_base_url(base_url)?;
        ensure_base_url_available(state, Some(id), &base_url).await?;
        active.base_url = Set(Some(base_url));
    }
    if let Some(endpoint_kind) = input.endpoint_kind {
        if builtin {
            return Err(AsterError::validation_error(
                "built-in Yggdrasil forwarding server endpoint_kind cannot be changed",
            ));
        }
        active.endpoint_kind = Set(endpoint_kind);
    }
    if let Some(enabled) = input.enabled {
        active.enabled = Set(enabled);
    }
    if let Some(priority) = input.priority {
        active.priority = Set(normalize_priority(priority)?);
    }
    if let Some(weight) = input.weight {
        active.weight = Set(normalize_weight(weight)?);
    }
    if let Some(timeout_ms) = input.timeout_ms {
        active.timeout_ms = Set(normalize_timeout_ms(timeout_ms)?);
    }
    if let Some(texture_forward_enabled) = input.texture_forward_enabled {
        if provider_kind == YggdrasilSessionForwardProviderKind::Local && texture_forward_enabled {
            return Err(AsterError::validation_error(
                "local Yggdrasil forwarding server cannot enable texture forwarding",
            ));
        }
        active.texture_forward_enabled = Set(texture_forward_enabled);
    }
    active.updated_at = Set(Utc::now());

    let server = yggdrasil_session_forward_server_repo::update(state.writer_db(), active).await?;
    crate::services::yggdrasil_service::invalidate_session_forward_server_cache(state).await;
    Ok(server_to_admin(server))
}

pub async fn delete_server<S>(state: &S, id: i64) -> Result<AdminYggdrasilSessionForwardServerInfo>
where
    S: CacheRuntimeState + DatabaseRuntimeState,
{
    let server = yggdrasil_session_forward_server_repo::find_by_id(state.writer_db(), id).await?;
    if server.builtin {
        return Err(AsterError::validation_error(
            "built-in Yggdrasil forwarding server cannot be deleted",
        ));
    }
    let info = server_to_admin(server);
    yggdrasil_session_forward_server_repo::delete(state.writer_db(), id).await?;
    crate::services::yggdrasil_service::invalidate_session_forward_server_cache(state).await;
    Ok(info)
}

async fn ensure_base_url_available<S>(
    state: &S,
    current_id: Option<i64>,
    base_url: &str,
) -> Result<()>
where
    S: DatabaseRuntimeState,
{
    let existing =
        yggdrasil_session_forward_server_repo::find_by_base_url(state.reader_db(), base_url)
            .await?;
    if existing.is_some_and(|server| Some(server.id) != current_id) {
        return Err(AsterError::validation_error(
            "Yggdrasil forwarding server base_url already exists",
        ));
    }
    Ok(())
}

fn server_to_admin(
    model: yggdrasil_session_forward_server::Model,
) -> AdminYggdrasilSessionForwardServerInfo {
    let local = model.provider_kind == YggdrasilSessionForwardProviderKind::Local;
    AdminYggdrasilSessionForwardServerInfo {
        id: model.id,
        display_name: model.display_name,
        provider_kind: model.provider_kind,
        endpoint_kind: model.endpoint_kind,
        base_url: model.base_url,
        builtin: model.builtin,
        enabled: model.enabled,
        priority: model.priority,
        weight: model.weight,
        timeout_ms: model.timeout_ms,
        texture_forward_enabled: model.texture_forward_enabled,
        local,
        deletable: !model.builtin,
        last_checked_at: model.last_checked_at,
        last_success_at: model.last_success_at,
        last_failure_at: model.last_failure_at,
        last_failure_message: model.last_failure_message,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

fn local_builtin_server() -> yggdrasil_session_forward_server::ActiveModel {
    let now = Utc::now();
    yggdrasil_session_forward_server::ActiveModel {
        display_name: Set(LOCAL_DISPLAY_NAME.to_string()),
        provider_kind: Set(YggdrasilSessionForwardProviderKind::Local),
        endpoint_kind: Set(YggdrasilSessionForwardEndpointKind::AuthlibInjector),
        base_url: Set(None),
        builtin: Set(true),
        enabled: Set(true),
        priority: Set(100),
        weight: Set(1),
        timeout_ms: Set(1500),
        texture_forward_enabled: Set(false),
        last_checked_at: Set(None),
        last_success_at: Set(None),
        last_failure_at: Set(None),
        last_failure_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

fn mojang_builtin_server() -> yggdrasil_session_forward_server::ActiveModel {
    let now = Utc::now();
    yggdrasil_session_forward_server::ActiveModel {
        display_name: Set(MOJANG_DISPLAY_NAME.to_string()),
        provider_kind: Set(YggdrasilSessionForwardProviderKind::Remote),
        endpoint_kind: Set(YggdrasilSessionForwardEndpointKind::MojangSession),
        base_url: Set(Some(MOJANG_SESSION_SERVER_BASE_URL.to_string())),
        builtin: Set(true),
        enabled: Set(false),
        priority: Set(200),
        weight: Set(1),
        timeout_ms: Set(1500),
        texture_forward_enabled: Set(false),
        last_checked_at: Set(None),
        last_success_at: Set(None),
        last_failure_at: Set(None),
        last_failure_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

pub fn server_audit_details(
    server: &AdminYggdrasilSessionForwardServerInfo,
) -> Option<serde_json::Value> {
    audit_service::details(audit_service::YggdrasilSessionForwardServerAuditDetails {
        provider_kind: server.provider_kind.as_str(),
        endpoint_kind: server.endpoint_kind.as_str(),
        base_url: server.base_url.as_deref(),
        builtin: server.builtin,
        enabled: server.enabled,
        priority: server.priority,
        weight: server.weight,
        timeout_ms: server.timeout_ms,
        texture_forward_enabled: server.texture_forward_enabled,
    })
}

fn normalize_display_name(value: String) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AsterError::validation_error("display_name cannot be empty"));
    }
    if value.len() > 128 {
        return Err(AsterError::validation_error(
            "display_name must not exceed 128 bytes",
        ));
    }
    Ok(value.to_string())
}

fn normalize_base_url(value: String) -> Result<String> {
    aster_forge_utils::url::normalize_http_base_url(
        &value,
        "base_url",
        aster_forge_utils::url::HttpBaseUrlOptions::required_without_query_fragment(),
    )
    .map_err(|error| AsterError::validation_error(error.to_string()))?
    .ok_or_else(|| AsterError::validation_error("base_url cannot be empty"))
}

fn normalize_priority(value: i32) -> Result<i32> {
    if !(MIN_PRIORITY..=MAX_PRIORITY).contains(&value) {
        return Err(AsterError::validation_error(format!(
            "priority must be between {MIN_PRIORITY} and {MAX_PRIORITY}"
        )));
    }
    Ok(value)
}

fn normalize_weight(value: i32) -> Result<i32> {
    if !(MIN_WEIGHT..=MAX_WEIGHT).contains(&value) {
        return Err(AsterError::validation_error(format!(
            "weight must be between {MIN_WEIGHT} and {MAX_WEIGHT}"
        )));
    }
    Ok(value)
}

fn normalize_timeout_ms(value: i32) -> Result<i32> {
    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&value) {
        return Err(AsterError::validation_error(format!(
            "timeout_ms must be between {MIN_TIMEOUT_MS} and {MAX_TIMEOUT_MS}"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};

    #[test]
    fn base_url_normalization_accepts_api_root_path_and_trims_slash() {
        assert_eq!(
            normalize_base_url(" https://Auth.EXAMPLE.test/yggdrasil/ ".to_string()).unwrap(),
            "https://Auth.EXAMPLE.test/yggdrasil"
        );
    }

    #[test]
    fn base_url_normalization_rejects_query_fragment_and_bad_scheme() {
        assert!(normalize_base_url("https://auth.example.test/yggdrasil?x=1".to_string()).is_err());
        assert!(normalize_base_url("https://auth.example.test/yggdrasil#x".to_string()).is_err());
        assert!(normalize_base_url("ftp://auth.example.test/yggdrasil".to_string()).is_err());
    }

    #[test]
    fn numeric_bounds_are_enforced() {
        assert!(normalize_priority(-10_000).is_ok());
        assert!(normalize_priority(10_001).is_err());
        assert!(normalize_weight(1).is_ok());
        assert!(normalize_weight(0).is_err());
        assert!(normalize_timeout_ms(100).is_ok());
        assert!(normalize_timeout_ms(99).is_err());
    }

    #[tokio::test]
    async fn ensure_builtin_servers_seeds_empty_migrated_database_once() {
        let db_cfg = crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            pool_size: 1,
            retry_count: 0,
        };
        let db = crate::db::connect_with_metrics(&db_cfg, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("test database should connect");
        migration::Migrator::up(&db, None)
            .await
            .expect("test database migrations should run");

        let before = yggdrasil_session_forward_server::Entity::find()
            .count(&db)
            .await
            .expect("forward servers should count");
        assert_eq!(before, 0);

        let inserted = ensure_builtin_servers(&db)
            .await
            .expect("builtin forward servers should seed");
        assert_eq!(inserted, 2);

        let inserted_again = ensure_builtin_servers(&db)
            .await
            .expect("builtin forward servers should be idempotent");
        assert_eq!(inserted_again, 0);

        let servers = yggdrasil_session_forward_server::Entity::find()
            .order_by_asc(yggdrasil_session_forward_server::Column::Priority)
            .all(&db)
            .await
            .expect("forward servers should list");
        assert_eq!(servers.len(), 2);
        assert_eq!(
            servers[0].provider_kind,
            YggdrasilSessionForwardProviderKind::Local
        );
        assert_eq!(servers[0].display_name, LOCAL_DISPLAY_NAME);
        assert_eq!(
            servers[1].base_url.as_deref(),
            Some(MOJANG_SESSION_SERVER_BASE_URL)
        );
    }
}
