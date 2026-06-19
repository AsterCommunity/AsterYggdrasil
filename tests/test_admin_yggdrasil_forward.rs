//! Integration tests for administrator Yggdrasil forwarding routes.

#[macro_use]
mod common;

use actix_web::test;
use aster_yggdrasil::entities::audit_log;
use aster_yggdrasil::services::audit_service;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;

const ROUTE: &str = "/api/v1/admin/yggdrasil/session-forward-servers";

#[actix_web::test]
async fn admin_yggdrasil_session_forward_requires_authentication() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());

    let req = test::TestRequest::get().uri(ROUTE).to_request();
    assert_service_status!(app, req, 401);
}

#[actix_web::test]
async fn admin_yggdrasil_session_forward_crud_and_local_row_contract() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let token = setup_admin!(app);

    let req = test::TestRequest::get()
        .uri(ROUTE)
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 2);
    let items = body["data"]["items"].as_array().expect("items");
    let local = items
        .iter()
        .find(|item| item["provider_kind"] == "local")
        .expect("local AY forward server");
    let local_id = local["id"].as_i64().expect("local id");
    assert_eq!(local["display_name"], "AsterYggdrasil");
    assert_eq!(local["provider_kind"], "local");
    assert_eq!(local["endpoint_kind"], "authlib_injector");
    assert_eq!(local["base_url"], Value::Null);
    assert_eq!(local["builtin"], true);
    assert_eq!(local["enabled"], true);
    assert_eq!(local["priority"], 100);
    assert_eq!(local["weight"], 1);
    assert_eq!(local["texture_forward_enabled"], false);
    assert_eq!(local["local"], true);
    assert_eq!(local["deletable"], false);
    let mojang = items
        .iter()
        .find(|item| item["display_name"] == "Mojang")
        .expect("Mojang forward server");
    assert_eq!(mojang["provider_kind"], "remote");
    assert_eq!(mojang["endpoint_kind"], "mojang_session");
    assert_eq!(mojang["base_url"], "https://sessionserver.mojang.com");
    assert_eq!(mojang["builtin"], true);
    assert_eq!(mojang["enabled"], false);
    assert_eq!(mojang["deletable"], false);

    let req = test::TestRequest::post()
        .uri(ROUTE)
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "display_name": "Backup Session",
            "base_url": " https://Remote.EXAMPLE.test/yggdrasil/ ",
            "endpoint_kind": "authlib_injector",
            "enabled": true,
            "priority": 50,
            "weight": 4,
            "timeout_ms": 2500,
            "texture_forward_enabled": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    let remote_id = body["data"]["id"].as_i64().expect("remote id");
    assert_eq!(body["data"]["display_name"], "Backup Session");
    assert_eq!(body["data"]["provider_kind"], "remote");
    assert_eq!(body["data"]["endpoint_kind"], "authlib_injector");
    assert_eq!(body["data"]["builtin"], false);
    assert_eq!(
        body["data"]["base_url"],
        "https://Remote.EXAMPLE.test/yggdrasil"
    );
    assert_eq!(body["data"]["priority"], 50);
    assert_eq!(body["data"]["weight"], 4);
    assert_eq!(body["data"]["timeout_ms"], 2500);
    assert_eq!(body["data"]["texture_forward_enabled"], true);
    assert_eq!(body["data"]["local"], false);
    assert_eq!(body["data"]["deletable"], true);

    let req = test::TestRequest::get()
        .uri(&format!("{ROUTE}/{remote_id}"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["id"], remote_id);

    let req = test::TestRequest::patch()
        .uri(&format!("{ROUTE}/{remote_id}"))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "display_name": "Weighted Remote",
            "base_url": "http://127.0.0.1:32000/api/yggdrasil/",
            "endpoint_kind": "mojang_session",
            "enabled": false,
            "priority": -10,
            "weight": 7,
            "timeout_ms": 900,
            "texture_forward_enabled": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["display_name"], "Weighted Remote");
    assert_eq!(
        body["data"]["base_url"],
        "http://127.0.0.1:32000/api/yggdrasil"
    );
    assert_eq!(body["data"]["endpoint_kind"], "mojang_session");
    assert_eq!(body["data"]["enabled"], false);
    assert_eq!(body["data"]["priority"], -10);
    assert_eq!(body["data"]["weight"], 7);
    assert_eq!(body["data"]["timeout_ms"], 900);
    assert_eq!(body["data"]["texture_forward_enabled"], false);

    let req = test::TestRequest::patch()
        .uri(&format!("{ROUTE}/{local_id}"))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "display_name": "AsterYggdrasil Local",
            "enabled": true,
            "priority": 80,
            "weight": 2,
            "timeout_ms": 1200
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["provider_kind"], "local");
    assert_eq!(body["data"]["display_name"], "AsterYggdrasil Local");
    assert_eq!(body["data"]["priority"], 80);
    assert_eq!(body["data"]["weight"], 2);
    assert_eq!(body["data"]["base_url"], Value::Null);

    let req = test::TestRequest::delete()
        .uri(&format!("{ROUTE}/{remote_id}"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri(&format!("{ROUTE}/{remote_id}"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    audit_service::flush_global_audit_log_manager().await;
    let create_entry = audit_entry_by_name(
        &state,
        audit_service::AuditAction::AdminCreateYggdrasilSessionForwardServer,
        "Backup Session",
    )
    .await;
    assert_eq!(create_entry.entity_type, "yggdrasil_session");
    assert_eq!(create_entry.entity_name.as_deref(), Some("Backup Session"));
    let details: Value = serde_json::from_str(create_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["provider_kind"], "remote");
    assert_eq!(details["endpoint_kind"], "authlib_injector");
    assert_eq!(details["base_url"], "https://Remote.EXAMPLE.test/yggdrasil");
    assert_eq!(details["texture_forward_enabled"], true);

    let update_entry = audit_entry_by_name(
        &state,
        audit_service::AuditAction::AdminUpdateYggdrasilSessionForwardServer,
        "Weighted Remote",
    )
    .await;
    assert_eq!(update_entry.entity_name.as_deref(), Some("Weighted Remote"));
    let details: Value = serde_json::from_str(update_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["priority"], -10);
    assert_eq!(details["endpoint_kind"], "mojang_session");
    assert_eq!(details["weight"], 7);

    let delete_entry = audit_entry_by_name(
        &state,
        audit_service::AuditAction::AdminDeleteYggdrasilSessionForwardServer,
        "Weighted Remote",
    )
    .await;
    assert_eq!(delete_entry.entity_name.as_deref(), Some("Weighted Remote"));
}

#[actix_web::test]
async fn admin_yggdrasil_session_forward_rejects_boundaries_and_local_mutation() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::get()
        .uri(ROUTE)
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().expect("items");
    let local_id = items
        .iter()
        .find(|item| item["provider_kind"] == "local")
        .and_then(|item| item["id"].as_i64())
        .expect("local id");
    let mojang_id = items
        .iter()
        .find(|item| item["display_name"] == "Mojang")
        .and_then(|item| item["id"].as_i64())
        .expect("mojang id");

    for payload in [
        serde_json::json!({
            "display_name": "Bad scheme",
            "base_url": "ftp://example.test/yggdrasil"
        }),
        serde_json::json!({
            "display_name": "Query",
            "base_url": "https://example.test/yggdrasil?x=1"
        }),
        serde_json::json!({
            "display_name": "Zero weight",
            "base_url": "https://weight.example.test/yggdrasil",
            "weight": 0
        }),
        serde_json::json!({
            "display_name": "Too slow",
            "base_url": "https://slow.example.test/yggdrasil",
            "timeout_ms": 10001
        }),
        serde_json::json!({
            "display_name": "Too high priority",
            "base_url": "https://priority.example.test/yggdrasil",
            "priority": 10001
        }),
        serde_json::json!({
            "display_name": "Legacy provider kind",
            "provider_kind": "local",
            "base_url": "https://legacy.example.test/yggdrasil"
        }),
        serde_json::json!({
            "display_name": "Bad endpoint kind",
            "endpoint_kind": "legacy",
            "base_url": "https://endpoint.example.test/yggdrasil"
        }),
    ] {
        let req = test::TestRequest::post()
            .uri(ROUTE)
            .insert_header(common::bearer_header(&token))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    let req = test::TestRequest::post()
        .uri(ROUTE)
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "display_name": "Duplicate One",
            "base_url": "https://duplicate.example.test/yggdrasil/"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);

    let req = test::TestRequest::post()
        .uri(ROUTE)
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "display_name": "Duplicate Two",
            "base_url": "https://duplicate.example.test/yggdrasil"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    for payload in [
        serde_json::json!({ "base_url": "https://local.example.test/yggdrasil" }),
        serde_json::json!({ "endpoint_kind": "mojang_session" }),
        serde_json::json!({ "texture_forward_enabled": true }),
    ] {
        let req = test::TestRequest::patch()
            .uri(&format!("{ROUTE}/{local_id}"))
            .insert_header(common::bearer_header(&token))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    let req = test::TestRequest::delete()
        .uri(&format!("{ROUTE}/{local_id}"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    for payload in [
        serde_json::json!({ "base_url": "https://session.example.test" }),
        serde_json::json!({ "endpoint_kind": "authlib_injector" }),
    ] {
        let req = test::TestRequest::patch()
            .uri(&format!("{ROUTE}/{mojang_id}"))
            .insert_header(common::bearer_header(&token))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    let req = test::TestRequest::delete()
        .uri(&format!("{ROUTE}/{mojang_id}"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn admin_yggdrasil_session_forward_list_supports_sort_modes() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    for payload in [
        serde_json::json!({
            "display_name": "Low Disabled",
            "base_url": "https://low-disabled.example.test/yggdrasil",
            "enabled": false,
            "priority": -100,
            "weight": 1
        }),
        serde_json::json!({
            "display_name": "Low Enabled",
            "base_url": "https://low-enabled.example.test/yggdrasil",
            "enabled": true,
            "priority": -50,
            "weight": 1
        }),
    ] {
        let req = test::TestRequest::post()
            .uri(ROUTE)
            .insert_header(common::bearer_header(&token))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);
    }

    let req = test::TestRequest::get()
        .uri(&format!("{ROUTE}?sort_by=call_order&limit=10"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let names: Vec<_> = body["data"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["display_name"].as_str().expect("display_name"))
        .collect();
    assert_eq!(names[0], "Low Enabled");
    let low_disabled_index = names
        .iter()
        .position(|name| *name == "Low Disabled")
        .expect("low disabled row");
    let mojang_index = names
        .iter()
        .position(|name| *name == "Mojang")
        .expect("mojang row");
    assert!(low_disabled_index > 0);
    assert!(mojang_index > low_disabled_index);

    let req = test::TestRequest::get()
        .uri(&format!("{ROUTE}?sort_by=id&limit=10"))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().expect("items");
    let ids: Vec<_> = items
        .iter()
        .map(|item| item["id"].as_i64().expect("id"))
        .collect();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(ids, sorted_ids);
}

async fn audit_entry_by_name(
    state: &aster_yggdrasil::runtime::AppState,
    action: audit_service::AuditAction,
    entity_name: &str,
) -> audit_log::Model {
    audit_log::Entity::find()
        .filter(audit_log::Column::Action.eq(action))
        .filter(audit_log::Column::EntityName.eq(entity_name))
        .order_by_desc(audit_log::Column::Id)
        .one(state.writer_db())
        .await
        .expect("audit query should succeed")
        .expect("audit entry should exist")
}
