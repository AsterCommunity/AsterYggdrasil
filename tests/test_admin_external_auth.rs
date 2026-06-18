//! Integration tests for administrator external auth routes.

#[macro_use]
mod common;

use actix_web::test;
use aster_yggdrasil::services::audit_service;
use serde_json::Value;

#[actix_web::test]
async fn admin_external_auth_requires_authentication() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/external-auth/providers")
        .to_request();
    assert_service_status!(app, req, 401);
}

#[actix_web::test]
async fn admin_external_auth_crud_redacts_secret_and_exposes_public_provider() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
        r#"["http://localhost:8080"]"#,
    ));
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/external-auth/providers")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "provider_kind": "generic_oauth2",
            "display_name": "Example IdP",
            "authorization_url": "http://127.0.0.1/authorize",
            "token_url": "http://127.0.0.1/token",
            "userinfo_url": "http://127.0.0.1/userinfo",
            "client_id": "client-id",
            "client_secret": "client-secret",
            "icon_url": "/static/external-auth/example.svg",
            "scopes": "email profile",
            "enabled": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    let provider_id = body["data"]["id"]
        .as_i64()
        .expect("provider id should be returned");
    let provider_key = body["data"]["key"]
        .as_str()
        .expect("provider key should be returned")
        .to_string();
    assert_eq!(body["data"]["provider_kind"], "generic_oauth2");
    assert_eq!(body["data"]["client_secret"], "***REDACTED***");
    assert_eq!(body["data"]["client_secret_configured"], true);
    assert_eq!(
        body["data"]["icon_url"],
        "/static/external-auth/example.svg"
    );

    audit_service::flush_global_audit_log_manager().await;
    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?action=admin_create_external_auth_provider&limit=1")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    let audit_entry = &body["data"]["items"][0];
    let details: Value = serde_json::from_str(audit_entry["details"].as_str().unwrap()).unwrap();
    assert_eq!(details["kind"], "generic_oauth2");
    assert_eq!(
        audit_entry["presentation"]["detail"]["params"]["kind"],
        "generic_oauth2"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/external-auth/provider-kinds")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["data"]
            .as_array()
            .expect("provider kind list should be an array")
            .iter()
            .any(|kind| kind["kind"] == "oidc")
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/external-auth/providers")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["key"], provider_key);

    let req = test::TestRequest::patch()
        .uri(&format!(
            "/api/v1/admin/external-auth/providers/{provider_id}"
        ))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "display_name": "Example Login"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["display_name"], "Example Login");

    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/admin/external-auth/providers/{provider_id}/test"
        ))
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["provider"], "Generic OAuth2");
    assert!(
        body["data"]["checks"]
            .as_array()
            .expect("checks should be an array")
            .iter()
            .all(|check| check["success"].as_bool().unwrap_or(false))
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/external-auth/generic_oauth2/providers")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["key"], provider_key);
    assert_eq!(
        body["data"]["items"][0]["icon_url"],
        "/static/external-auth/example.svg"
    );

    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/v1/auth/external-auth/generic_oauth2/{provider_key}/start"
        ))
        .insert_header(("Origin", "http://localhost:8080"))
        .set_json(serde_json::json!({
            "return_path": "/dashboard"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let authorize_url = body["data"]["authorization_url"]
        .as_str()
        .expect("authorization URL should be returned");
    assert!(authorize_url.starts_with("http://127.0.0.1/authorize?"));
    assert!(authorize_url.contains("client_id=client-id"));
    assert!(authorize_url.contains("state="));
}

#[actix_web::test]
async fn admin_external_auth_rejects_immutable_or_legacy_request_fields() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    for payload in [
        serde_json::json!({
            "provider_kind": "generic_oauth2",
            "display_name": "Legacy key",
            "authorization_url": "http://127.0.0.1/authorize",
            "token_url": "http://127.0.0.1/token",
            "userinfo_url": "http://127.0.0.1/userinfo",
            "client_id": "client-id",
            "key": "manual-key"
        }),
        serde_json::json!({
            "provider_kind": "generic_oauth2",
            "display_name": "Legacy slug",
            "authorization_url": "http://127.0.0.1/authorize",
            "token_url": "http://127.0.0.1/token",
            "userinfo_url": "http://127.0.0.1/userinfo",
            "client_id": "client-id",
            "slug": "manual-slug"
        }),
        serde_json::json!({
            "provider_kind": "generic_oauth2",
            "display_name": "Legacy kind alias",
            "authorization_url": "http://127.0.0.1/authorize",
            "token_url": "http://127.0.0.1/token",
            "userinfo_url": "http://127.0.0.1/userinfo",
            "client_id": "client-id",
            "kind": "generic_oauth2"
        }),
    ] {
        let req = test::TestRequest::post()
            .uri("/api/v1/admin/external-auth/providers")
            .insert_header(common::bearer_header(&token))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/external-auth/providers")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "provider_kind": "generic_oauth2",
            "display_name": "Mutable fields baseline",
            "authorization_url": "http://127.0.0.1/authorize",
            "token_url": "http://127.0.0.1/token",
            "userinfo_url": "http://127.0.0.1/userinfo",
            "client_id": "client-id"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    let provider_id = body["data"]["id"].as_i64().expect("provider id");

    for payload in [
        serde_json::json!({ "key": "patched-key" }),
        serde_json::json!({ "slug": "patched-slug" }),
        serde_json::json!({ "kind": "google" }),
        serde_json::json!({ "provider_kind": "google" }),
    ] {
        let req = test::TestRequest::patch()
            .uri(&format!(
                "/api/v1/admin/external-auth/providers/{provider_id}"
            ))
            .insert_header(common::bearer_header(&token))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
