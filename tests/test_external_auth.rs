//! Integration tests for external auth routes shared by provider-specific suites.

#[macro_use]
mod common;

use actix_web::test;
use aster_yggdrasil::entities::{external_auth_login_flow, external_auth_provider};
use aster_yggdrasil::types::{
    external_auth::ExternalAuthProtocol, external_auth::ExternalAuthProviderKind,
    external_auth::StoredExternalAuthProviderOptions,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::Value;

fn oidc_provider_model(key: &str, enabled: bool) -> external_auth_provider::ActiveModel {
    let now = Utc::now();
    external_auth_provider::ActiveModel {
        key: Set(key.to_string()),
        display_name: Set("Example".to_string()),
        icon_url: Set(None),
        provider_kind: Set(ExternalAuthProviderKind::Oidc),
        protocol: Set(ExternalAuthProtocol::Oidc),
        options: Set(StoredExternalAuthProviderOptions::empty()),
        enabled: Set(enabled),
        issuer_url: Set(Some("https://id.example.test".to_string())),
        authorization_url: Set(Some("https://id.example.test/authorize".to_string())),
        token_url: Set(Some("https://id.example.test/token".to_string())),
        userinfo_url: Set(Some("https://id.example.test/userinfo".to_string())),
        client_id: Set("client-id".to_string()),
        client_secret: Set(Some("client-secret".to_string())),
        scopes: Set("openid email profile".to_string()),
        auto_provision_enabled: Set(false),
        auto_link_verified_email_enabled: Set(false),
        require_email_verified: Set(true),
        subject_claim: Set(None),
        username_claim: Set(None),
        display_name_claim: Set(None),
        email_claim: Set(None),
        email_verified_claim: Set(None),
        groups_claim: Set(None),
        avatar_url_claim: Set(None),
        allowed_domains: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

fn oauth2_provider_model(key: &str, enabled: bool) -> external_auth_provider::ActiveModel {
    external_auth_provider::ActiveModel {
        provider_kind: Set(ExternalAuthProviderKind::GenericOAuth2),
        protocol: Set(ExternalAuthProtocol::OAuth2),
        issuer_url: Set(None),
        scopes: Set("email profile".to_string()),
        require_email_verified: Set(false),
        subject_claim: Set(Some("id".to_string())),
        username_claim: Set(Some("login".to_string())),
        display_name_claim: Set(Some("name".to_string())),
        email_claim: Set(Some("email".to_string())),
        email_verified_claim: Set(Some("email_verified".to_string())),
        ..oidc_provider_model(key, enabled)
    }
}

#[actix_web::test]
async fn external_auth_lists_enabled_providers() {
    let state = common::setup().await;
    oidc_provider_model("example", true)
        .insert(state.db_handles.writer())
        .await
        .expect("external auth provider should insert");

    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/external-auth/providers")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"][0]["key"], "example");
    assert_eq!(body["data"]["items"][0]["display_name"], "Example");
    assert_eq!(body["data"]["items"][0]["kind"], "oidc");
}

#[actix_web::test]
async fn external_auth_provider_lists_are_paginated() {
    let state = common::setup().await;
    for (key, display_name, kind) in [
        ("alpha", "Alpha", ExternalAuthProviderKind::Oidc),
        ("bravo", "Bravo", ExternalAuthProviderKind::GenericOAuth2),
        ("charlie", "Charlie", ExternalAuthProviderKind::Oidc),
    ] {
        let mut provider = oidc_provider_model(key, true);
        provider.display_name = Set(display_name.to_string());
        provider.provider_kind = Set(kind);
        provider.protocol = Set(match kind {
            ExternalAuthProviderKind::GenericOAuth2 => ExternalAuthProtocol::OAuth2,
            _ => ExternalAuthProtocol::Oidc,
        });
        provider
            .insert(state.db_handles.writer())
            .await
            .expect("external auth provider should insert");
    }

    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/external-auth/providers?limit=500")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100);
    assert!(body["data"].get("offset").is_none());
    assert_eq!(body["data"]["total"], 3);
    assert_eq!(body["data"]["items"][0]["key"], "alpha");
    assert!(body["data"]["next_cursor"].is_null());

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/external-auth/oidc/providers?limit=1")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let first_page_body: Value = test::read_body_json(resp).await;
    assert_eq!(first_page_body["data"]["limit"], 1);
    assert!(first_page_body["data"].get("offset").is_none());
    assert_eq!(first_page_body["data"]["total"], 2);
    assert_eq!(first_page_body["data"]["items"][0]["key"], "alpha");
    let next_cursor = &first_page_body["data"]["next_cursor"];
    assert_eq!(next_cursor["value"], "Alpha");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/external-auth/oidc/providers?limit=1&after_display_name={}&after_id={}",
            next_cursor["value"].as_str().unwrap(),
            next_cursor["id"].as_i64().unwrap()
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let second_page_body: Value = test::read_body_json(resp).await;
    assert_eq!(second_page_body["data"]["limit"], 1);
    assert_eq!(second_page_body["data"]["total"], 2);
    assert_eq!(second_page_body["data"]["items"][0]["key"], "charlie");
}

#[actix_web::test]
async fn external_auth_start_returns_authorization_url() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
        r#"["http://localhost:8080"]"#,
    ));
    oauth2_provider_model("example", true)
        .insert(state.db_handles.writer())
        .await
        .expect("external auth provider should insert");

    let app = create_test_app!(state);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/external-auth/generic_oauth2/example/start")
        .insert_header(("Origin", "http://localhost:8080"))
        .set_json(serde_json::json!({
            "return_path": "/dashboard"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: Value = test::read_body_json(resp).await;
    let authorization_url = body["data"]["authorization_url"]
        .as_str()
        .expect("authorization URL should exist");
    assert!(authorization_url.starts_with("https://id.example.test/authorize?"));
    assert!(authorization_url.contains("client_id=client-id"));
    assert!(authorization_url.contains("state="));
}

#[actix_web::test]
async fn cleanup_expired_external_auth_flows_removes_only_expired_flows() {
    let state = common::setup().await;
    let now = Utc::now();
    let provider = oidc_provider_model("cleanup-provider", true)
        .insert(state.writer_db())
        .await
        .expect("external auth provider should insert");

    let expired = external_auth_login_flow::ActiveModel {
        provider_id: Set(provider.id),
        state_hash: Set("expired-flow-state-hash".to_string()),
        nonce: Set(None),
        pkce_verifier: Set(None),
        redirect_uri: Set("https://app.example.test/callback".to_string()),
        return_path: Set(Some("/dashboard".to_string())),
        expires_at: Set(now - Duration::minutes(1)),
        consumed_at: Set(None),
        created_at: Set(now - Duration::hours(1)),
        ..Default::default()
    }
    .insert(state.writer_db())
    .await
    .expect("expired external auth flow should insert");
    let active = external_auth_login_flow::ActiveModel {
        provider_id: Set(provider.id),
        state_hash: Set("active-flow-state-hash".to_string()),
        nonce: Set(None),
        pkce_verifier: Set(None),
        redirect_uri: Set("https://app.example.test/callback".to_string()),
        return_path: Set(Some("/dashboard".to_string())),
        expires_at: Set(now + Duration::minutes(10)),
        consumed_at: Set(None),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(state.writer_db())
    .await
    .expect("active external auth flow should insert");

    let removed = aster_yggdrasil::services::auth::external::cleanup_expired_flows(&state)
        .await
        .expect("external auth flow cleanup should succeed");

    assert_eq!(removed, 1);
    let expired_after = external_auth_login_flow::Entity::find_by_id(expired.id)
        .one(state.reader_db())
        .await
        .expect("expired flow query should succeed");
    let active_after = external_auth_login_flow::Entity::find_by_id(active.id)
        .one(state.reader_db())
        .await
        .expect("active flow query should succeed");
    assert!(expired_after.is_none());
    assert!(active_after.is_some());
}
