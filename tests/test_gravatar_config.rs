//! Integration tests for Gravatar runtime configuration.

#[macro_use]
mod common;

use actix_web::test;
use aster_yggdrasil::config::definitions::GRAVATAR_BASE_URL_KEY;
use aster_yggdrasil::db::repository::{system_config_repo, user_repo};
use aster_yggdrasil::entities::user;
use aster_yggdrasil::runtime::AppState;
use aster_yggdrasil::services::user::profile::{self, AvatarAudience};
use aster_yggdrasil::types::user::AvatarSource;
use serde_json::Value;

async fn load_admin_user(state: &AppState) -> user::Model {
    user_repo::find_by_id(state.writer_db(), 1)
        .await
        .expect("admin user should exist")
}

async fn set_gravatar_and_load_url(state: &AppState) -> String {
    let user = load_admin_user(state).await;
    profile::set_avatar_source(state, user.id, AvatarSource::Gravatar)
        .await
        .expect("avatar source should update");

    let user = load_admin_user(state).await;
    let info = profile::get_profile_info(state, &user, AvatarAudience::SelfUser)
        .await
        .expect("profile info should load");

    assert_eq!(info.avatar.source, AvatarSource::Gravatar);
    info.avatar
        .url_512
        .expect("gravatar profile should expose 512px URL")
}

#[actix_web::test]
async fn gravatar_config_default_and_custom_base_url_drive_avatar_urls() {
    let state = common::setup().await;
    let state_for_assert = state.clone();
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let default_url = set_gravatar_and_load_url(&state_for_assert).await;
    assert!(
        default_url.starts_with("https://www.gravatar.com/avatar/"),
        "expected default Gravatar URL, got: {default_url}"
    );
    assert!(default_url.contains("d=identicon"));
    assert!(default_url.contains("s=512"));
    assert!(default_url.contains("r=g"));

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/admin/config/{GRAVATAR_BASE_URL_KEY}"))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "value": "https://cravatar.cn/avatar"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["config"]["value"],
        "https://cravatar.cn/avatar"
    );
    assert_eq!(
        state_for_assert
            .runtime_config()
            .get(GRAVATAR_BASE_URL_KEY)
            .as_deref(),
        Some("https://cravatar.cn/avatar")
    );

    let custom_url = set_gravatar_and_load_url(&state_for_assert).await;
    assert!(
        custom_url.starts_with("https://cravatar.cn/avatar/"),
        "expected custom Gravatar URL, got: {custom_url}"
    );
    assert!(custom_url.contains("d=identicon"));
}

#[actix_web::test]
async fn gravatar_config_normalizes_empty_values_and_trailing_slashes() {
    let state = common::setup().await;
    let state_for_assert = state.clone();
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/admin/config/{GRAVATAR_BASE_URL_KEY}"))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "value": " https://mirror.example.com/avatar/ "
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["config"]["value"],
        "https://mirror.example.com/avatar"
    );

    let mirror_url = set_gravatar_and_load_url(&state_for_assert).await;
    assert!(
        mirror_url.starts_with("https://mirror.example.com/avatar/"),
        "expected normalized mirror URL, got: {mirror_url}"
    );
    let after_scheme = mirror_url
        .strip_prefix("https://")
        .expect("mirror URL should use https");
    assert!(
        !after_scheme.contains("//"),
        "URL path should not contain double slashes: {mirror_url}"
    );

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/admin/config/{GRAVATAR_BASE_URL_KEY}"))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "value": "   "
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["config"]["value"],
        "https://www.gravatar.com/avatar"
    );

    let fallback_url = set_gravatar_and_load_url(&state_for_assert).await;
    assert!(
        fallback_url.starts_with("https://www.gravatar.com/avatar/"),
        "expected fallback Gravatar URL, got: {fallback_url}"
    );
}

#[actix_web::test]
async fn gravatar_config_rejects_invalid_base_urls_without_overwriting_runtime() {
    let state = common::setup().await;
    let state_for_assert = state.clone();
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/admin/config/{GRAVATAR_BASE_URL_KEY}"))
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({
            "value": "https://mirror.example.com/avatar"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    for invalid_value in [
        "ftp://mirror.example.com/avatar",
        "https://mirror.example.com/avatar?default=identicon",
        "https://mirror.example.com/avatar#section",
        "not a url",
    ] {
        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/admin/config/{GRAVATAR_BASE_URL_KEY}"))
            .insert_header(common::bearer_header(&token))
            .set_json(serde_json::json!({
                "value": invalid_value
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            400,
            "invalid value should fail: {invalid_value}"
        );
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], "bad_request");
        assert_eq!(
            state_for_assert
                .runtime_config()
                .get(GRAVATAR_BASE_URL_KEY)
                .as_deref(),
            Some("https://mirror.example.com/avatar"),
            "invalid Gravatar base URL must not overwrite runtime config"
        );
    }
}

#[actix_web::test]
async fn gravatar_config_runtime_blank_value_falls_back_to_default() {
    let state = common::setup().await;
    let state_for_assert = state.clone();
    let app = create_test_app!(state);
    let _token = setup_admin!(app);

    let config = system_config_repo::upsert(
        state_for_assert.writer_db(),
        GRAVATAR_BASE_URL_KEY,
        "   ",
        1,
    )
    .await
    .expect("blank gravatar config should save for legacy-data simulation");
    state_for_assert.runtime_config().apply(config);

    let url = set_gravatar_and_load_url(&state_for_assert).await;
    assert!(
        url.starts_with("https://www.gravatar.com/avatar/"),
        "expected fallback for blank runtime config, got: {url}"
    );
}
