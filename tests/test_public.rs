//! Integration tests for public frontend bootstrap routes.

#[macro_use]
mod common;

use actix_web::{http::header, test};
use aster_yggdrasil::config::{
    definitions::{
        AUTH_ALLOW_USER_REGISTRATION_KEY, BRANDING_DESCRIPTION_KEY, BRANDING_FAVICON_URL_KEY,
        BRANDING_TITLE_KEY, BRANDING_WORDMARK_DARK_URL_KEY, PUBLIC_SITE_URL_KEY,
        YGGDRASIL_ALLOW_CAPE_UPLOAD_KEY, YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY,
        YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY, YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
        YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY, YGGDRASIL_PUBLIC_BASE_URL_KEY,
        YGGDRASIL_SERVER_NAME_KEY, YGGDRASIL_SKIN_DOMAINS_KEY,
    },
    yggdrasil::{DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS, DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES},
};
use serde_json::Value;

#[actix_web::test]
async fn public_frontend_config_returns_default_bootstrap_config() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/public/frontend-config")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
        Some("public, max-age=60")
    );
    assert_eq!(
        resp.headers()
            .get(header::VARY)
            .and_then(|value| value.to_str().ok()),
        Some("Authorization, Cookie")
    );

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "success");
    assert_eq!(body["data"]["version"], 1);
    assert_eq!(body["data"]["branding"]["title"], "AsterYggdrasil");
    assert_eq!(body["data"]["branding"]["favicon_url"], "/favicon.svg");
    assert_eq!(body["data"]["branding"]["site_urls"], serde_json::json!([]));
    assert_eq!(body["data"]["branding"]["allow_user_registration"], true);
    assert_eq!(body["data"]["yggdrasil"]["server_name"], "AsterYggdrasil");
    assert_eq!(
        body["data"]["yggdrasil"]["skin_domains"],
        serde_json::json!([".minecraft.net", ".mojang.com"])
    );
    assert_eq!(
        body["data"]["yggdrasil"]["public_base_urls"],
        serde_json::json!([])
    );
    assert_eq!(body["data"]["yggdrasil"]["allow_profile_name_login"], true);
    assert_eq!(body["data"]["yggdrasil"]["allow_skin_upload"], true);
    assert_eq!(body["data"]["yggdrasil"]["allow_cape_upload"], true);
    assert_eq!(
        body["data"]["yggdrasil"]["max_texture_upload_bytes"],
        DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES
    );
    assert_eq!(
        body["data"]["yggdrasil"]["max_texture_pixels"],
        DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS
    );
}

#[actix_web::test]
async fn public_frontend_config_uses_runtime_overrides() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        BRANDING_TITLE_KEY,
        "Block Forge",
    ));
    state.runtime_config.apply(common::system_config_model(
        BRANDING_DESCRIPTION_KEY,
        "Private Minecraft identity service",
    ));
    state.runtime_config.apply(common::system_config_model(
        BRANDING_FAVICON_URL_KEY,
        "https://cdn.example.test/favicon.svg",
    ));
    state.runtime_config.apply(common::system_config_model(
        BRANDING_WORDMARK_DARK_URL_KEY,
        "/brand/wordmark-dark.svg",
    ));
    state.runtime_config.apply(common::system_config_model(
        PUBLIC_SITE_URL_KEY,
        r#"["https://panel.example.test","https://login.example.test"]"#,
    ));
    state.runtime_config.apply(common::system_config_model(
        AUTH_ALLOW_USER_REGISTRATION_KEY,
        "false",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_SERVER_NAME_KEY,
        "Block Forge Auth",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_SKIN_DOMAINS_KEY,
        r#"[".minecraft.net","skins.example.test"]"#,
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_PUBLIC_BASE_URL_KEY,
        r#"["https://skins.example.test/api"]"#,
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY,
        "false",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY,
        "false",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_ALLOW_CAPE_UPLOAD_KEY,
        "false",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
        "1234567",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
        "4096",
    ));
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/public/frontend-config")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["branding"]["title"], "Block Forge");
    assert_eq!(
        body["data"]["branding"]["description"],
        "Private Minecraft identity service"
    );
    assert_eq!(
        body["data"]["branding"]["favicon_url"],
        "https://cdn.example.test/favicon.svg"
    );
    assert_eq!(
        body["data"]["branding"]["wordmark_dark_url"],
        "/brand/wordmark-dark.svg"
    );
    assert_eq!(
        body["data"]["branding"]["site_urls"],
        serde_json::json!(["https://panel.example.test", "https://login.example.test"])
    );
    assert_eq!(body["data"]["branding"]["allow_user_registration"], false);
    assert_eq!(body["data"]["yggdrasil"]["server_name"], "Block Forge Auth");
    assert_eq!(
        body["data"]["yggdrasil"]["skin_domains"],
        serde_json::json!([".minecraft.net", ".mojang.com", "skins.example.test"])
    );
    assert_eq!(
        body["data"]["yggdrasil"]["public_base_urls"],
        serde_json::json!(["https://skins.example.test/api"])
    );
    assert_eq!(body["data"]["yggdrasil"]["allow_profile_name_login"], false);
    assert_eq!(body["data"]["yggdrasil"]["allow_skin_upload"], false);
    assert_eq!(body["data"]["yggdrasil"]["allow_cape_upload"], false);
    assert_eq!(
        body["data"]["yggdrasil"]["max_texture_upload_bytes"],
        1_234_567
    );
    assert_eq!(body["data"]["yggdrasil"]["max_texture_pixels"], 4096);
}

#[actix_web::test]
async fn public_frontend_config_exposes_minimum_positive_texture_limits() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
        "1",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
        "1",
    ));
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/public/frontend-config")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["yggdrasil"]["max_texture_upload_bytes"], 1);
    assert_eq!(body["data"]["yggdrasil"]["max_texture_pixels"], 1);
}

#[actix_web::test]
async fn public_frontend_config_falls_back_for_invalid_texture_limits() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
        "0",
    ));
    state.runtime_config.apply(common::system_config_model(
        YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
        "not-a-number",
    ));
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/public/frontend-config")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["yggdrasil"]["max_texture_upload_bytes"],
        DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES
    );
    assert_eq!(
        body["data"]["yggdrasil"]["max_texture_pixels"],
        DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS
    );
}
