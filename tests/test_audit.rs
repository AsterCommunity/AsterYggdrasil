//! Integration tests for audit logging.

#[macro_use]
mod common;

use actix_web::test;
use aster_forge_mail::MailMessage;
use aster_yggdrasil::config::definitions::BRANDING_TITLE_KEY;
use aster_yggdrasil::db::repository::{audit_log_repo, mail_outbox_repo, user_repo};
use aster_yggdrasil::entities::{audit_log, background_task, mail_outbox};
use aster_yggdrasil::errors::{AsterError, Result as AsterResult};
use aster_yggdrasil::runtime::AppState;
use aster_yggdrasil::services::{audit_service, mail_outbox_service, mail_service::MailSender};
use aster_yggdrasil::types::{
    BackgroundTaskKind, BackgroundTaskStatus, MailOutboxStatus, MailTemplateCode,
    StoredMailPayload, StoredTaskPayload,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use sea_orm::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;
use std::any::Any;
use std::sync::Arc;

fn find_action<'a>(items: &'a [Value], action: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["action"] == action)
        .unwrap_or_else(|| {
            panic!(
                "audit log should contain {action}, got {:?}",
                items
                    .iter()
                    .map(|item| item["action"].as_str().unwrap_or("<non-string>"))
                    .collect::<Vec<_>>()
            )
        })
}

async fn insert_failed_retryable_task(
    state: &AppState,
    display_name: &str,
    attempt_count: i32,
) -> i64 {
    let now = Utc::now();
    let task = aster_yggdrasil::db::repository::background_task_repo::create(
        state.writer_db(),
        background_task::ActiveModel {
            kind: Set(BackgroundTaskKind::SystemRuntime),
            status: Set(BackgroundTaskStatus::Failed),
            creator_user_id: Set(None),
            display_name: Set(display_name.to_string()),
            payload_json: Set(StoredTaskPayload(
                serde_json::json!({ "task_name": "task-retry-audit" }).to_string(),
            )),
            result_json: Set(None),
            runtime_json: Set(None),
            steps_json: Set(None),
            progress_current: Set(0),
            progress_total: Set(1),
            status_text: Set(None),
            attempt_count: Set(attempt_count),
            max_attempts: Set(3),
            next_run_at: Set(now),
            processing_token: Set(0),
            processing_started_at: Set(None),
            last_heartbeat_at: Set(None),
            lease_expires_at: Set(None),
            started_at: Set(Some(now - Duration::minutes(5))),
            finished_at: Set(Some(now - Duration::minutes(1))),
            last_error: Set(Some("retry audit failure".to_string())),
            failure_can_retry: Set(Some(true)),
            expires_at: Set(now + Duration::hours(24)),
            created_at: Set(now - Duration::minutes(5)),
            updated_at: Set(now - Duration::minutes(1)),
            ..Default::default()
        },
    )
    .await
    .expect("test background task should insert");
    task.id
}

struct FailingMailSender;

#[async_trait]
impl MailSender for FailingMailSender {
    async fn send(&self, _message: MailMessage) -> AsterResult<()> {
        Err(AsterError::mail_delivery_failed("smtp unavailable"))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn mail_outbox_model(
    attempt_count: i32,
    payload_json: StoredMailPayload,
) -> mail_outbox::ActiveModel {
    let now = Utc::now();
    mail_outbox::ActiveModel {
        template_code: Set(MailTemplateCode::RegisterActivation),
        to_address: Set("audit-mail@example.com".to_string()),
        to_name: Set(Some("Audit Mail".to_string())),
        payload_json: Set(payload_json),
        status: Set(MailOutboxStatus::Pending),
        attempt_count: Set(attempt_count),
        next_attempt_at: Set(now),
        processing_started_at: Set(None),
        sent_at: Set(None),
        last_error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

async fn latest_audit_entry(
    state: &AppState,
    action: audit_service::AuditAction,
) -> audit_log::Model {
    audit_log::Entity::find()
        .filter(audit_log::Column::Action.eq(action))
        .order_by_desc(audit_log::Column::Id)
        .one(state.writer_db())
        .await
        .expect("audit log query should succeed")
        .expect("audit log entry should exist")
}

async fn user_id_by_username(state: &AppState, username: &str) -> i64 {
    user_repo::find_by_username(state.reader_db(), username)
        .await
        .expect("user query should succeed")
        .unwrap_or_else(|| panic!("test user {username} should exist"))
        .id
}

async fn insert_account_audit_entry(
    state: &AppState,
    user_id: i64,
    action: audit_service::AuditAction,
    entity_type: audit_service::AuditEntityType,
    entity_id: i64,
    entity_name: &str,
    created_at: chrono::DateTime<Utc>,
) -> i64 {
    audit_log_repo::create(
        state.writer_db(),
        audit_log::ActiveModel {
            user_id: Set(user_id),
            action: Set(action),
            entity_type: Set(entity_type.as_str().to_string()),
            entity_id: Set(Some(entity_id)),
            entity_name: Set(Some(entity_name.to_string())),
            details: Set(Some(
                serde_json::json!({
                    "entity_name": entity_name,
                    "source": "account-audit-test",
                })
                .to_string(),
            )),
            ip_address: Set(Some("127.0.0.1".to_string())),
            user_agent: Set(Some("account-audit-test".to_string())),
            created_at: Set(created_at),
            ..Default::default()
        },
    )
    .await
    .expect("test audit log should insert")
    .id
}

#[actix_web::test]
async fn audit_log_persists_external_auth_provider_entry() {
    let state = common::setup().await;

    audit_service::log(
        &state,
        &audit_service::AuditContext {
            user_id: 42,
            ip_address: None,
            user_agent: None,
        },
        audit_service::AuditAction::AdminTestExternalAuthProvider,
        audit_service::AuditEntityType::ExternalAuthProvider,
        None,
        Some("draft"),
        Some(serde_json::json!({
            "provider_kind": "oidc",
            "key": "draft",
            "success": true,
        })),
    )
    .await;
    audit_service::flush_global_audit_log_manager().await;

    let entry = audit_log::Entity::find()
        .filter(
            audit_log::Column::Action.eq(audit_service::AuditAction::AdminTestExternalAuthProvider),
        )
        .one(state.writer_db())
        .await
        .expect("audit log query should succeed")
        .expect("audit log should persist");

    assert_eq!(entry.user_id, 42);
    assert_eq!(entry.entity_type, "external_auth_provider");
    assert_eq!(entry.entity_name.as_deref(), Some("draft"));
    let details: Value = serde_json::from_str(entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["provider_kind"], "oidc");
    assert_eq!(details["key"], "draft");
    assert_eq!(details["success"], true);
}

#[actix_web::test]
async fn admin_audit_logs_are_admin_only() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _admin_token = setup_admin!(app);
    let user_token = register_user!(app, "audit-user", "audit-user@example.com", "password1234");

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs")
        .to_request();
    assert_service_status!(app, req, 401);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs")
        .insert_header(common::bearer_header(user_token))
        .to_request();
    assert_service_status!(app, req, 403);
}

#[actix_web::test]
async fn account_audit_logs_are_limited_to_current_user() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _admin_token = setup_admin!(app);
    let alice_token = register_user!(
        app,
        "audit-alice",
        "audit-alice@example.com",
        "password1234"
    );
    let bob_token = register_user!(app, "audit-bob", "audit-bob@example.com", "password1234");

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs")
        .to_request();
    assert_service_status!(app, req, 401);

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs?limit=20")
        .insert_header(common::bearer_header(&alice_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"]
        .as_array()
        .expect("account audit log items should be an array");
    assert!(items.iter().any(|item| item["action"] == "user_register"));
    assert!(
        items
            .iter()
            .all(|item| item["user"]["username"] == "audit-alice"),
        "account audit endpoint must not expose other users: {items:?}"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs?limit=20")
        .insert_header(common::bearer_header(&bob_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"]
        .as_array()
        .expect("account audit log items should be an array");
    assert!(
        items
            .iter()
            .all(|item| item["user"]["username"] == "audit-bob"),
        "account audit endpoint must not expose other users: {items:?}"
    );
}

#[actix_web::test]
async fn account_audit_logs_clamp_limit_and_apply_offset() {
    let state = common::setup().await;
    let state_for_insert = state.clone();
    let app = create_test_app!(state);
    let _admin_token = setup_admin!(app);
    let token = register_user!(
        app,
        "audit-page-user",
        "audit-page-user@example.com",
        "password1234"
    );
    let user_id = user_id_by_username(&state_for_insert, "audit-page-user").await;
    let base = chrono::DateTime::parse_from_rfc3339("2035-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    for index in 0..105 {
        insert_account_audit_entry(
            &state_for_insert,
            user_id,
            audit_service::AuditAction::MinecraftProfileCreate,
            audit_service::AuditEntityType::MinecraftProfile,
            10_000 + index,
            &format!("PagedProfile{index:03}"),
            base + Duration::seconds(index),
        )
        .await;
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs?action=minecraft_profile_create&limit=9999")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100);
    assert!(body["data"].get("offset").is_none());
    assert_eq!(body["data"]["total"], 105);
    assert_eq!(
        body["data"]["items"].as_array().unwrap().len(),
        100,
        "account audit list should clamp oversized page requests"
    );
    let cursor = body["data"]["next_cursor"].clone();
    assert!(cursor.is_object(), "first page should expose a next cursor");
    let cursor_value = cursor["value"].as_str().unwrap();
    let cursor_id = cursor["id"].as_i64().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/account/audit-logs?action=minecraft_profile_create&limit=9999&after_created_at={cursor_value}&after_id={cursor_id}",
        ))
        .insert_header(common::bearer_header(token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100);
    assert!(body["data"].get("offset").is_none());
    assert_eq!(body["data"]["total"], 105);
    assert_eq!(
        body["data"]["items"].as_array().unwrap().len(),
        5,
        "cursor should page within the current user's filtered audit logs"
    );
}

#[actix_web::test]
async fn account_audit_logs_filter_by_rfc3339_bounds_and_entity_fields() {
    let state = common::setup().await;
    let state_for_insert = state.clone();
    let app = create_test_app!(state);
    let _admin_token = setup_admin!(app);
    let token = register_user!(
        app,
        "audit-filter",
        "audit-filter-user@example.com",
        "password1234"
    );
    let user_id = user_id_by_username(&state_for_insert, "audit-filter").await;
    let base = chrono::DateTime::parse_from_rfc3339("2035-02-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let before_match_id = insert_account_audit_entry(
        &state_for_insert,
        user_id,
        audit_service::AuditAction::MinecraftTextureUpload,
        audit_service::AuditEntityType::MinecraftTexture,
        20_001,
        "BoundaryBefore",
        base,
    )
    .await;
    let lower_bound_id = insert_account_audit_entry(
        &state_for_insert,
        user_id,
        audit_service::AuditAction::MinecraftTextureUpload,
        audit_service::AuditEntityType::MinecraftTexture,
        20_002,
        "BoundaryLower",
        base + Duration::minutes(1),
    )
    .await;
    let upper_bound_id = insert_account_audit_entry(
        &state_for_insert,
        user_id,
        audit_service::AuditAction::MinecraftTextureUpload,
        audit_service::AuditEntityType::MinecraftTexture,
        20_003,
        "BoundaryUpper",
        base + Duration::minutes(2),
    )
    .await;
    let after_match_id = insert_account_audit_entry(
        &state_for_insert,
        user_id,
        audit_service::AuditAction::MinecraftTextureUpload,
        audit_service::AuditEntityType::MinecraftTexture,
        20_004,
        "BoundaryAfter",
        base + Duration::minutes(3),
    )
    .await;
    insert_account_audit_entry(
        &state_for_insert,
        user_id,
        audit_service::AuditAction::MinecraftProfileCreate,
        audit_service::AuditEntityType::MinecraftProfile,
        20_002,
        "WrongAction",
        base + Duration::minutes(1),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs?action=minecraft_texture_upload&entity_type=minecraft_texture&after=2035-02-01T00:01:00Z&before=2035-02-01T00:02:00Z&limit=20")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let item_ids = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(body["data"]["total"], 2);
    assert_eq!(item_ids, vec![upper_bound_id, lower_bound_id]);
    assert!(
        !item_ids.contains(&before_match_id) && !item_ids.contains(&after_match_id),
        "RFC3339 bounds should be inclusive and exclude values outside the window"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs?action=minecraft_texture_upload&entity_type=minecraft_texture&entity_id=20002&limit=20")
        .insert_header(common::bearer_header(token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["id"], lower_bound_id);
}

#[actix_web::test]
async fn account_audit_logs_accept_admin_shape_sort_query() {
    let state = common::setup().await;
    let state_for_insert = state.clone();
    let app = create_test_app!(state);
    let _admin_token = setup_admin!(app);
    let token = register_user!(
        app,
        "audit-sort-user",
        "audit-sort-user@example.com",
        "password1234"
    );
    let other_token = register_user!(
        app,
        "audit-sort-other",
        "audit-sort-other@example.com",
        "password1234"
    );
    let user_id = user_id_by_username(&state_for_insert, "audit-sort-user").await;
    let other_user_id = user_id_by_username(&state_for_insert, "audit-sort-other").await;
    let base = chrono::DateTime::parse_from_rfc3339("2035-04-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    insert_account_audit_entry(
        &state_for_insert,
        user_id,
        audit_service::AuditAction::UserLogout,
        audit_service::AuditEntityType::AuthSession,
        40_001,
        "SortLogout",
        base,
    )
    .await;
    insert_account_audit_entry(
        &state_for_insert,
        user_id,
        audit_service::AuditAction::MinecraftProfileCreate,
        audit_service::AuditEntityType::MinecraftProfile,
        40_002,
        "SortProfile",
        base + Duration::seconds(1),
    )
    .await;
    insert_account_audit_entry(
        &state_for_insert,
        other_user_id,
        audit_service::AuditAction::ConfigUpdate,
        audit_service::AuditEntityType::SystemConfig,
        40_003,
        "OtherUserConfig",
        base + Duration::seconds(2),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs?limit=20")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let actions = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["action"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(
        actions
            .iter()
            .any(|action| action == "minecraft_profile_create")
    );
    assert!(actions.iter().any(|action| action == "user_logout"));
    assert!(
        !actions.iter().any(|action| action == "config_update"),
        "query must not bypass current-user scoping: {actions:?}"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/account/audit-logs?limit=20")
        .insert_header(common::bearer_header(other_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn account_overview_returns_recent_current_user_activity() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _admin_token = setup_admin!(app);
    let token = register_user!(
        app,
        "overview-user",
        "overview-user@example.com",
        "password1234"
    );
    let _login_token = login_user!(app, "overview-user", "password1234");

    for name in ["OverviewOne", "OverviewTwo"] {
        let req = test::TestRequest::post()
            .uri("/api/v1/profiles/minecraft")
            .insert_header(common::bearer_header(&token))
            .set_json(serde_json::json!({ "name": name }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200, "profile {name} should be created");
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/account/overview")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["recent_activity"]
        .as_array()
        .expect("recent activity should be an array");
    assert_eq!(body["data"]["profile_count"], 2);
    assert!(!items.is_empty());
    assert!(
        items
            .iter()
            .all(|item| item["user"]["username"] == "overview-user"),
        "overview activity must be scoped to current user: {items:?}"
    );
    assert!(
        items.len() <= 5,
        "overview should return a small recent activity slice"
    );
}

#[actix_web::test]
async fn account_overview_returns_latest_five_current_user_activities() {
    let state = common::setup().await;
    let state_for_insert = state.clone();
    let app = create_test_app!(state);
    let _admin_token = setup_admin!(app);
    let token = register_user!(
        app,
        "overview-limit",
        "overview-limit-user@example.com",
        "password1234"
    );
    let other_token = register_user!(
        app,
        "overview-other",
        "overview-other-user@example.com",
        "password1234"
    );
    let user_id = user_id_by_username(&state_for_insert, "overview-limit").await;
    let other_user_id = user_id_by_username(&state_for_insert, "overview-other").await;
    let base = chrono::DateTime::parse_from_rfc3339("2035-03-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);

    let mut inserted_ids = Vec::new();
    for index in 0..7 {
        let id = insert_account_audit_entry(
            &state_for_insert,
            user_id,
            audit_service::AuditAction::MinecraftTextureBind,
            audit_service::AuditEntityType::MinecraftProfile,
            30_000 + index,
            &format!("OverviewProfile{index}"),
            base + Duration::seconds(index),
        )
        .await;
        inserted_ids.push(id);
    }
    let other_user_latest_id = insert_account_audit_entry(
        &state_for_insert,
        other_user_id,
        audit_service::AuditAction::MinecraftTextureBind,
        audit_service::AuditEntityType::MinecraftProfile,
        39_999,
        "OtherUserLatest",
        base + Duration::minutes(10),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/account/overview")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let item_ids = body["data"]["recent_activity"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    let expected_ids = inserted_ids
        .iter()
        .rev()
        .take(5)
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(item_ids, expected_ids);
    assert!(
        !item_ids.contains(&other_user_latest_id),
        "overview must not leak newer activity from another user"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/account/overview")
        .insert_header(common::bearer_header(other_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let other_items = body["data"]["recent_activity"].as_array().unwrap();
    assert!(
        other_items
            .iter()
            .any(|item| item["id"] == other_user_latest_id),
        "other user should still see their own activity"
    );
}

#[actix_web::test]
async fn admin_can_list_filter_and_page_audit_logs() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let _login_token = login_user!(app, "admin", "password1234");

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/admin/config/{BRANDING_TITLE_KEY}"))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "value": "Audit Test Title"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?limit=50")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;

    let items = body["data"]["items"]
        .as_array()
        .expect("audit log items should be an array");
    assert!(body["data"]["total"].as_u64().unwrap() >= 3);
    assert_eq!(body["data"]["limit"], 50);
    assert!(body["data"].get("offset").is_none());

    let setup = find_action(items, "system_setup");
    assert_eq!(setup["entity_type"], "user");
    assert_eq!(setup["entity_name"], "admin");
    assert_eq!(setup["user"]["username"], "admin");

    let login = find_action(items, "user_login");
    assert_eq!(login["entity_type"], "auth_session");
    assert_eq!(login["entity_name"], "admin");
    assert_eq!(login["presentation"]["summary"]["code"], "user_login");
    assert_eq!(login["presentation"]["target"]["code"], "auth_session");
    assert_eq!(
        login["presentation"]["detail"]["code"],
        "user_login_identifier"
    );
    assert_eq!(
        login["presentation"]["detail"]["params"]["identifier"],
        "admin"
    );
    let login_details: Value = serde_json::from_str(login["details"].as_str().unwrap()).unwrap();
    assert_eq!(login_details["identifier"], "admin");

    let config = find_action(items, "config_update");
    assert_eq!(config["entity_type"], "system_config");
    assert_eq!(config["entity_name"], BRANDING_TITLE_KEY);
    assert_eq!(config["presentation"]["summary"]["code"], "config_update");
    assert_eq!(
        config["presentation"]["summary"]["params"]["key"],
        BRANDING_TITLE_KEY
    );
    assert_eq!(
        config["presentation"]["detail"]["code"],
        "config_value_updated"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?action=config_update&limit=1")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["action"], "config_update");
    assert_eq!(body["data"]["items"][0]["entity_name"], BRANDING_TITLE_KEY);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?limit=9999")
        .insert_header(common::bearer_header(token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 200);
}

#[actix_web::test]
async fn admin_task_cleanup_records_audit_log() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);
    let finished_before = chrono::Utc::now();

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/tasks/cleanup")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "finished_before": finished_before,
            "kind": "system_runtime",
            "status": "failed",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?action=admin_cleanup_tasks&limit=1")
        .insert_header(common::bearer_header(token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);

    let cleanup_entry = &body["data"]["items"][0];
    assert_eq!(cleanup_entry["action"], "admin_cleanup_tasks");
    assert_eq!(cleanup_entry["entity_type"], "task");
    assert_eq!(
        cleanup_entry["presentation"]["detail"]["code"],
        "tasks_cleanup_finished"
    );
    let details: Value = serde_json::from_str(cleanup_entry["details"].as_str().unwrap()).unwrap();
    assert_eq!(details["removed"], 0);
    assert!(details["finished_before"].is_string());
    assert_eq!(details["kind"], "system_runtime");
    assert_eq!(details["status"], "failed");
}

#[actix_web::test]
async fn admin_task_retry_records_audit_log() {
    let state = common::setup().await;
    let state_for_insert = state.clone();
    let app = create_test_app!(state);
    let token = setup_admin!(app);
    let task_id = insert_failed_retryable_task(&state_for_insert, "Retry audit task", 2).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/tasks/{task_id}/retry"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?action=task_retry&limit=1")
        .insert_header(common::bearer_header(token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);

    let retry_entry = &body["data"]["items"][0];
    assert_eq!(retry_entry["action"], "task_retry");
    assert_eq!(retry_entry["entity_type"], "task");
    assert_eq!(retry_entry["entity_id"], task_id);
    assert_eq!(retry_entry["entity_name"], "Retry audit task");
    assert_eq!(
        retry_entry["presentation"]["detail"]["code"],
        "task_retry_scheduled"
    );
    let details: Value = serde_json::from_str(retry_entry["details"].as_str().unwrap()).unwrap();
    assert_eq!(details["kind"], "system_runtime");
    assert_eq!(details["previous_attempt_count"], 2);
}

#[actix_web::test]
async fn mail_outbox_dispatch_records_delivery_audit_logs() {
    let state = common::setup().await;
    let payload =
        aster_yggdrasil::services::mail_template::MailTemplatePayload::register_activation(
            "alice",
            "token-123",
            "AsterYggdrasil",
        )
        .to_stored()
        .expect("mail payload should serialize");
    let sent_row = mail_outbox_repo::create(state.writer_db(), mail_outbox_model(0, payload))
        .await
        .expect("mail outbox row should insert");

    let stats = mail_outbox_service::dispatch_due(&state)
        .await
        .expect("mail outbox dispatch should succeed");

    assert_eq!(stats.claimed, 1);
    assert_eq!(stats.sent, 1);
    let sent_entry = latest_audit_entry(&state, audit_service::AuditAction::MailSend).await;
    assert_eq!(sent_entry.user_id, 0);
    assert_eq!(sent_entry.entity_type, "mail");
    assert_eq!(sent_entry.entity_id, Some(sent_row.id));
    assert_eq!(sent_entry.entity_name.as_deref(), Some("mail"));
    let sent_details: Value = serde_json::from_str(sent_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(sent_details["to_address"], "audit-mail@example.com");
    assert_eq!(sent_details["template_code"], "register_activation");
    assert_eq!(sent_details["to_name"], "Audit Mail");
    assert_eq!(sent_details["outbox_id"], sent_row.id);
    assert_eq!(sent_details["attempt_count"], 1);

    let payload =
        aster_yggdrasil::services::mail_template::MailTemplatePayload::register_activation(
            "alice",
            "token-456",
            "AsterYggdrasil",
        )
        .to_stored()
        .expect("mail payload should serialize");
    let failed_row = mail_outbox_repo::create(state.writer_db(), mail_outbox_model(5, payload))
        .await
        .expect("mail outbox row should insert");
    let sender: Arc<dyn MailSender> = Arc::new(FailingMailSender);

    let stats =
        mail_outbox_service::dispatch_due_with(state.writer_db(), &state.runtime_config, &sender)
            .await
            .expect("mail outbox dispatch should handle final delivery failure");

    assert_eq!(stats.claimed, 1);
    assert_eq!(stats.failed, 1);
    let failed_entry =
        latest_audit_entry(&state, audit_service::AuditAction::MailDeliveryFailed).await;
    assert_eq!(failed_entry.user_id, 0);
    assert_eq!(failed_entry.entity_type, "mail");
    assert_eq!(failed_entry.entity_id, Some(failed_row.id));
    let failed_details: Value =
        serde_json::from_str(failed_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(failed_details["to_address"], "audit-mail@example.com");
    assert_eq!(failed_details["template_code"], "register_activation");
    assert_eq!(failed_details["outbox_id"], failed_row.id);
    assert_eq!(failed_details["attempt_count"], 6);
    assert_eq!(failed_details["error"], "smtp unavailable");
}
