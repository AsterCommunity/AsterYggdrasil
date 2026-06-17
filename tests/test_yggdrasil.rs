//! Integration tests for Yggdrasil launcher authentication.

#[macro_use]
mod common;

use actix_web::{http::header, test};
use aster_yggdrasil::api::middleware::yggdrasil_rate_limit::YggdrasilRateLimiter;
use aster_yggdrasil::config::auth_runtime::AUTH_ALLOW_USER_REGISTRATION_KEY;
use aster_yggdrasil::config::definitions::PUBLIC_SITE_URL_KEY;
use aster_yggdrasil::config::yggdrasil::{
    YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY, YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES_KEY,
    YGGDRASIL_ENABLE_PROFILE_KEY_KEY, YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
    YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY, YGGDRASIL_PUBLIC_BASE_URL_KEY,
    YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY, YGGDRASIL_TOKEN_TTL_DAYS_KEY,
};
use aster_yggdrasil::config::{RateLimitConfig, RateLimitTier};
use aster_yggdrasil::db::repository::{minecraft_profile_repo, system_config_repo, user_repo};
use aster_yggdrasil::entities::{
    audit_log, minecraft_profile_texture, minecraft_texture, yggdrasil_token,
};
use aster_yggdrasil::services::audit_service;
use aster_yggdrasil::types::MinecraftTextureModel;
use aster_yggdrasil::utils::hash::sha256_hex;
use base64::Engine;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::Value;
use std::{
    io::Cursor,
    num::{NonZeroU32, NonZeroU64},
    sync::Arc,
};

const DEFAULT_STEVE_SKIN_HASH: &str =
    "082fdbe1403d09fcf030464bf754439ee79e9287bb15efbe2f54d02f60108133";
const DEFAULT_ALEX_SKIN_HASH: &str =
    "394b483392052fb28d6271c736ba0df0394223c93b6348f1f1d135fdb7412daa";

async fn setup_yggdrasil() -> aster_yggdrasil::runtime::AppState {
    let state = common::setup().await;
    configure_yggdrasil_public_site_url(&state).await;
    state
}

async fn setup_yggdrasil_with_strict_auth_rate_limit() -> aster_yggdrasil::runtime::AppState {
    let mut state = setup_yggdrasil().await;
    let config = RateLimitConfig {
        enabled: true,
        auth: RateLimitTier {
            seconds_per_request: NonZeroU64::new(60).unwrap(),
            burst_size: NonZeroU32::new(1).unwrap(),
        },
        ..Default::default()
    };
    state.yggdrasil_rate_limiter = YggdrasilRateLimiter::from_config(&config);
    state
}

async fn configure_yggdrasil_public_site_url(state: &aster_yggdrasil::runtime::AppState) {
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        PUBLIC_SITE_URL_KEY,
        r#"["http://localhost"]"#,
        None,
        None,
    )
    .await
    .expect("public_site_url config should update");
    state.runtime_config().apply(saved);
}

macro_rules! create_profile {
    ($app:expr, $access:expr, $name:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/profiles/minecraft")
            .insert_header(common::bearer_header($access))
            .set_json(serde_json::json!({ "name": $name }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        body["data"]["id"].as_str().unwrap().to_string()
    }};
}

macro_rules! ygg_login {
    ($app:expr, $username:expr, $client_token:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/yggdrasil/authserver/authenticate")
            .set_json(serde_json::json!({
                "username": $username,
                "password": "password1234",
                "clientToken": $client_token,
                "agent": { "name": "Minecraft", "version": 1 }
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        YggLogin {
            access_token: body["accessToken"].as_str().unwrap().to_string(),
        }
    }};
}

macro_rules! assert_minecraft_services_not_found {
    ($app:expr, $method:ident, $uri:expr, $path:expr) => {{
        let req = test::TestRequest::$method().uri($uri).to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 404);
        assert_eq!(
            resp.headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["path"], $path);
    }};
}

macro_rules! ygg_login_selected {
    ($app:expr, $client_token:expr, $profile_id:expr, $profile_name:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/yggdrasil/authserver/authenticate")
            .set_json(serde_json::json!({
                "username": "admin@example.com",
                "password": "password1234",
                "clientToken": $client_token,
                "agent": { "name": "Minecraft", "version": 1 }
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        let access_token = body["accessToken"].as_str().unwrap().to_string();

        let req = test::TestRequest::post()
            .uri("/api/yggdrasil/authserver/refresh")
            .set_json(serde_json::json!({
                "accessToken": access_token,
                "clientToken": $client_token,
                "selectedProfile": {
                    "id": $profile_id,
                    "name": $profile_name
                }
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["selectedProfile"]["id"], $profile_id);
        YggLogin {
            access_token: body["accessToken"].as_str().unwrap().to_string(),
        }
    }};
}

macro_rules! validate_ygg_token_status {
    ($app:expr, $access_token:expr, $client_token:expr, $status:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/yggdrasil/authserver/validate")
            .set_json(serde_json::json!({
                "accessToken": $access_token,
                "clientToken": $client_token
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), $status);
    }};
}

macro_rules! upload_texture_req {
    ($app:expr, $access_token:expr, $profile_id:expr, $texture_type:expr, $model:expr, $png:expr) => {{
        let (content_type, body) = texture_multipart_body($model, $png);
        let req = test::TestRequest::put()
            .uri(&format!(
                "/api/yggdrasil/api/user/profile/{}/{}",
                $profile_id, $texture_type
            ))
            .insert_header(("Authorization", format!("Bearer {}", $access_token)))
            .insert_header(("Content-Type", content_type))
            .set_payload(body)
            .to_request();
        test::call_service(&$app, req).await
    }};
}

macro_rules! upload_wardrobe_texture_req {
    ($app:expr, $access_token:expr, $texture_type:expr, $model:expr, $png:expr) => {{
        let (content_type, body) = texture_multipart_body($model, $png);
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/wardrobe/textures/{}", $texture_type))
            .insert_header(common::bearer_header($access_token))
            .insert_header(("Content-Type", content_type))
            .set_payload(body)
            .to_request();
        test::call_service(&$app, req).await
    }};
}

macro_rules! list_wardrobe_textures_req {
    ($app:expr, $access_token:expr, $uri:expr) => {{
        let req = test::TestRequest::get()
            .uri($uri)
            .insert_header(common::bearer_header($access_token))
            .to_request();
        test::call_service(&$app, req).await
    }};
}

macro_rules! profile_textures {
    ($app:expr, $profile_id:expr) => {{
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/yggdrasil/sessionserver/session/minecraft/profile/{}",
                $profile_id
            ))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200);
        let profile_body: Value = test::read_body_json(resp).await;
        decode_textures_property(&profile_body)
    }};
}

#[actix_web::test]
async fn yggdrasil_authenticate_validate_refresh_and_invalidate_flow() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "AsterPlayer" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    assert_eq!(profile_body["data"]["name"], "AsterPlayer");
    let profile_id = profile_body["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(profile_id.len(), 32);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "password1234",
            "clientToken": "launcher-client",
            "requestUser": true,
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let auth_body: Value = test::read_body_json(resp).await;
    assert_eq!(auth_body["clientToken"], "launcher-client");
    assert_eq!(auth_body["selectedProfile"]["id"], profile_id);
    assert_eq!(auth_body["selectedProfile"]["name"], "AsterPlayer");
    assert_eq!(auth_body["availableProfiles"].as_array().unwrap().len(), 1);
    let user_id = auth_body["user"]["id"].as_str().unwrap();
    assert_unsigned_uuid(user_id);
    assert_ne!(user_id, "1");
    let ygg_access = auth_body["accessToken"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": ygg_access,
            "clientToken": "launcher-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": ygg_access,
            "clientToken": "launcher-client",
            "requestUser": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let refresh_body: Value = test::read_body_json(resp).await;
    assert_eq!(refresh_body["clientToken"], "launcher-client");
    assert_eq!(refresh_body["selectedProfile"]["id"], profile_id);
    assert_eq!(refresh_body["user"]["id"], user_id);
    let refreshed_access = refresh_body["accessToken"].as_str().unwrap().to_string();
    assert_ne!(refreshed_access, ygg_access);

    let old_token = yggdrasil_token::Entity::find()
        .filter(yggdrasil_token::Column::AccessTokenHash.eq(sha256_hex(ygg_access.as_bytes())))
        .one(state.writer_db())
        .await
        .unwrap()
        .expect("old token row should remain for audit/history");
    assert!(
        old_token.revoked_at.is_some(),
        "refresh should revoke the old token in the same transaction that creates the replacement"
    );
    let new_token = yggdrasil_token::Entity::find()
        .filter(
            yggdrasil_token::Column::AccessTokenHash.eq(sha256_hex(refreshed_access.as_bytes())),
        )
        .one(state.writer_db())
        .await
        .unwrap()
        .expect("refreshed token row should exist");
    assert!(
        new_token.revoked_at.is_none(),
        "refreshed token should be the active replacement"
    );
    let active_token_count = yggdrasil_token::Entity::find()
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(active_token_count, 1);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": ygg_access,
            "clientToken": "launcher-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let error_body: Value = test::read_body_json(resp).await;
    assert_eq!(error_body["error"], "ForbiddenOperationException");
    assert_eq!(error_body["errorMessage"], "Invalid token.");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/invalidate")
        .set_json(serde_json::json!({
            "accessToken": refreshed_access,
            "clientToken": "ignored-by-spec"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);
}

#[actix_web::test]
async fn yggdrasil_refresh_precondition_failure_keeps_original_token_valid() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(app, &access, "RefreshBounded");
    let login = ygg_login!(app, "admin@example.com", "refresh-precondition-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "refresh-precondition-client",
            "selectedProfile": {
                "id": profile_id,
                "name": "RefreshBounded"
            }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "Access token already has a profile assigned",
    )
    .await;

    validate_ygg_token_status!(app, &login.access_token, "refresh-precondition-client", 204);
    let total_tokens = yggdrasil_token::Entity::find()
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(total_tokens, 1);
    let active_tokens = yggdrasil_token::Entity::find()
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(active_tokens, 1);
}

#[actix_web::test]
async fn yggdrasil_refresh_rolls_back_old_token_revoke_when_replacement_issue_fails() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let _profile_id = create_profile!(app, &access, "RefreshRollback");
    let login = ygg_login!(app, "admin@example.com", "refresh-rollback-client");

    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_TOKEN_TTL_DAYS_KEY,
        "9223372036854775808",
        None,
        None,
    )
    .await
    .unwrap();
    state.runtime_config().apply(saved);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "refresh-rollback-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        500,
        "InternalServerError",
        "yggdrasil token ttl days exceeds i64 range",
    )
    .await;

    validate_ygg_token_status!(app, &login.access_token, "refresh-rollback-client", 204);
    let token = yggdrasil_token::Entity::find()
        .filter(
            yggdrasil_token::Column::AccessTokenHash.eq(sha256_hex(login.access_token.as_bytes())),
        )
        .one(state.writer_db())
        .await
        .unwrap()
        .expect("original token should remain after rollback");
    assert!(
        token.revoked_at.is_none(),
        "failed refresh must roll back the old token revocation"
    );
    let total_tokens = yggdrasil_token::Entity::find()
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(total_tokens, 1);
    let active_tokens = yggdrasil_token::Entity::find()
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(active_tokens, 1);
}

#[actix_web::test]
async fn yggdrasil_refresh_old_token_cannot_be_reused_after_success() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let _profile_id = create_profile!(app, &access, "RefreshReplay");
    let login = ygg_login!(app, "admin@example.com", "refresh-replay-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "refresh-replay-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let refreshed_access = body["accessToken"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "refresh-replay-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    validate_ygg_token_status!(app, &refreshed_access, "refresh-replay-client", 204);
    let active_tokens = yggdrasil_token::Entity::find()
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(active_tokens, 1);
}

#[actix_web::test]
async fn yggdrasil_refresh_with_single_active_token_limit_keeps_replacement_valid() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let _profile_id = create_profile!(app, &access, "RefreshLimit");
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        aster_yggdrasil::config::yggdrasil::YGGDRASIL_MAX_ACTIVE_TOKENS_KEY,
        "1",
        None,
        None,
    )
    .await
    .unwrap();
    state.runtime_config().apply(saved);
    let login = ygg_login!(app, "admin@example.com", "refresh-single-limit-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "refresh-single-limit-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let refreshed_access = body["accessToken"].as_str().unwrap().to_string();

    validate_ygg_token_status!(app, &login.access_token, "refresh-single-limit-client", 403);
    validate_ygg_token_status!(app, &refreshed_access, "refresh-single-limit-client", 204);
    let active_tokens = yggdrasil_token::Entity::find()
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(active_tokens, 1);
}

#[actix_web::test]
async fn yggdrasil_profile_name_login_is_controlled_by_runtime_config() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "ConfigUser" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    assert_eq!(metadata["meta"]["feature.non_email_login"], true);
    assert_eq!(metadata["meta"]["feature.enable_profile_key"], true);
    assert_eq!(
        metadata["meta"]["feature.enable_mojang_anti_features"],
        true
    );
    assert_eq!(metadata["meta"]["feature.username_check"], true);
    assert_eq!(metadata["meta"]["links"]["homepage"], "http://localhost/");
    assert_eq!(
        metadata["meta"]["links"]["register"],
        "http://localhost/register"
    );
    assert!(
        metadata["meta"].get("feature").is_none(),
        "authlib-injector expects dotted feature keys in meta, not nested feature objects"
    );
    assert!(
        metadata["skinDomains"]
            .as_array()
            .unwrap()
            .iter()
            .any(|domain| domain == ".minecraft.net")
    );
    assert!(
        metadata["skinDomains"]
            .as_array()
            .unwrap()
            .iter()
            .any(|domain| domain == ".mojang.com")
    );
    assert!(
        metadata["skinDomains"]
            .as_array()
            .unwrap()
            .iter()
            .any(|domain| domain == "localhost")
    );

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "ConfigUser",
            "password": "password1234",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["selectedProfile"]["name"], "ConfigUser");

    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_ALLOW_PROFILE_NAME_LOGIN_KEY,
        "false",
        None,
        None,
    )
    .await
    .unwrap();
    state.runtime_config().apply(saved);

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    assert_eq!(metadata["meta"]["feature.non_email_login"], false);
    assert_eq!(metadata["meta"]["feature.username_check"], true);
    assert!(
        metadata["meta"].get("feature").is_none(),
        "authlib-injector expects dotted feature keys in meta, not nested feature objects"
    );

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "ConfigUser",
            "password": "password1234",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
}

#[actix_web::test]
async fn yggdrasil_metadata_capability_flags_follow_runtime_config() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    assert_eq!(metadata["meta"]["feature.enable_profile_key"], true);
    assert_eq!(
        metadata["meta"]["feature.enable_mojang_anti_features"],
        true
    );
    assert_eq!(metadata["meta"]["feature.username_check"], true);
    assert!(
        metadata["meta"].get("feature").is_none(),
        "authlib-injector expects dotted feature keys in meta, not nested feature objects"
    );

    for key in [
        YGGDRASIL_ENABLE_PROFILE_KEY_KEY,
        YGGDRASIL_ENABLE_MOJANG_ANTI_FEATURES_KEY,
    ] {
        let saved =
            system_config_repo::upsert_with_options(state.writer_db(), key, "false", None, None)
                .await
                .unwrap();
        state.runtime_config().apply(saved);
    }

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    assert_eq!(metadata["meta"]["feature.enable_profile_key"], false);
    assert_eq!(
        metadata["meta"]["feature.enable_mojang_anti_features"],
        false
    );
    assert_eq!(metadata["meta"]["feature.username_check"], true);

    assert_minecraft_services_not_found!(
        app,
        post,
        "/api/yggdrasil/minecraftservices/player/certificates",
        "/player/certificates"
    );
    assert_minecraft_services_not_found!(
        app,
        get,
        "/api/yggdrasil/minecraftservices/privileges",
        "/privileges"
    );
    assert_minecraft_services_not_found!(
        app,
        get,
        "/api/yggdrasil/minecraftservices/player/attributes",
        "/player/attributes"
    );
    assert_minecraft_services_not_found!(
        app,
        get,
        "/api/yggdrasil/minecraftservices/privacy/blocklist",
        "/privacy/blocklist"
    );
}

#[actix_web::test]
async fn yggdrasil_metadata_links_respect_registration_policy() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    assert_eq!(metadata["meta"]["links"]["homepage"], "http://localhost/");
    assert_eq!(
        metadata["meta"]["links"]["register"],
        "http://localhost/register"
    );

    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        AUTH_ALLOW_USER_REGISTRATION_KEY,
        "false",
        None,
        None,
    )
    .await
    .unwrap();
    state.runtime_config().apply(saved);

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    assert_eq!(metadata["meta"]["links"]["homepage"], "http://localhost/");
    assert!(
        metadata["meta"]["links"].get("register").is_none(),
        "metadata must not advertise a closed registration entrypoint"
    );
}

#[actix_web::test]
async fn minecraft_services_profile_key_certificate_uses_yggdrasil_bearer_token() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let _profile_id = create_profile!(app, &access, "CertUser");
    let login = ygg_login!(app, "admin@example.com", "cert-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/minecraftservices/player/certificates")
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;

    assert!(
        body["keyPair"]["privateKey"]
            .as_str()
            .unwrap()
            .starts_with("-----BEGIN RSA PRIVATE KEY-----")
    );
    assert!(
        body["keyPair"]["publicKey"]
            .as_str()
            .unwrap()
            .starts_with("-----BEGIN RSA PUBLIC KEY-----")
    );
    assert_eq!(body["publicKeySignature"], "AA==");
    assert_eq!(body["publicKeySignatureV2"], "AA==");
    assert!(chrono::DateTime::parse_from_rfc3339(body["expiresAt"].as_str().unwrap()).is_ok());
    assert!(chrono::DateTime::parse_from_rfc3339(body["refreshedAfter"].as_str().unwrap()).is_ok());

    let expires_at =
        chrono::DateTime::parse_from_rfc3339(body["expiresAt"].as_str().unwrap()).unwrap();
    let refreshed_after =
        chrono::DateTime::parse_from_rfc3339(body["refreshedAfter"].as_str().unwrap()).unwrap();
    assert!(
        expires_at > refreshed_after,
        "profile key certificate must expire after its refresh time"
    );
}

#[actix_web::test]
async fn minecraft_services_profile_key_certificate_rejects_missing_unselected_or_invalid_token() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let first_profile_id = create_profile!(app, &access, "CertFirst");
    let _second_profile_id = create_profile!(app, &access, "CertSecond");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/minecraftservices/player/certificates")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["path"], "/player/certificates");

    let login = ygg_login!(app, "admin@example.com", "cert-unselected-client");
    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/minecraftservices/player/certificates")
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["path"], "/player/certificates");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/minecraftservices/player/certificates")
        .insert_header(("Authorization", "Bearer not-a-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["path"], "/player/certificates");

    let selected_login = ygg_login_selected!(
        app,
        "cert-selected-client",
        first_profile_id.as_str(),
        "CertFirst"
    );
    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/minecraftservices/player/certificates")
        .insert_header((
            "Authorization",
            format!("Bearer {}", selected_login.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn minecraft_services_profile_key_certificate_rejects_revoked_and_wrong_method_requests() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let _profile_id = create_profile!(app, &access, "CertRevoked");
    let login = ygg_login!(app, "admin@example.com", "cert-revoked-client");
    let access_token = login.access_token;

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/minecraftservices/player/certificates")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/invalidate")
        .set_json(serde_json::json!({
            "accessToken": access_token,
            "clientToken": "cert-revoked-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/minecraftservices/player/certificates")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["path"], "/player/certificates");
}

#[actix_web::test]
async fn minecraft_services_anti_feature_policy_endpoints_are_served() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let _access = setup_admin!(app);
    let login = ygg_login!(app, "admin@example.com", "policy-client");
    let authorization = format!("Bearer {}", login.access_token);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/minecraftservices/privileges")
        .insert_header(("Authorization", authorization.as_str()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["privileges"]["onlineChat"]["enabled"], true);
    assert_eq!(body["privileges"]["multiplayerServer"]["enabled"], true);
    assert_eq!(body["privileges"]["multiplayerRealms"]["enabled"], true);
    assert_eq!(body["privileges"]["telemetry"]["enabled"], true);
    assert_eq!(body["privileges"]["optionalTelemetry"]["enabled"], true);
    assert!(
        body.get("profanityFilterPreferences").is_none(),
        "privileges endpoint should not include player attributes fields"
    );

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/minecraftservices/player/attributes")
        .insert_header(("Authorization", authorization.as_str()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["profanityFilterPreferences"]["profanityFilterOn"],
        false
    );
    assert_eq!(body["privileges"]["onlineChat"]["enabled"], true);
    assert_eq!(body["privileges"]["multiplayerServer"]["enabled"], true);
    assert_eq!(body["privileges"]["multiplayerRealms"]["enabled"], true);
    assert_eq!(body["privileges"]["telemetry"]["enabled"], true);
    assert_eq!(body["privileges"]["optionalTelemetry"]["enabled"], true);
    assert_eq!(body["friendsPreferences"]["friends"], "DISABLED");
    assert_eq!(body["friendsPreferences"]["acceptInvites"], "DISABLED");
    assert_eq!(body["chatPreferences"]["textCommunication"], "ENABLED");
    assert_eq!(body["banStatus"]["bannedScopes"], serde_json::json!({}));

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/minecraftservices/privacy/blocklist")
        .insert_header(("Authorization", authorization.as_str()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["blockedProfiles"], serde_json::json!([]));

    assert_minecraft_services_not_found!(
        app,
        get,
        "/api/yggdrasil/sessionserver/blockedservers",
        "/blockedservers"
    );
}

#[actix_web::test]
async fn yggdrasil_unmatched_routes_use_minecraft_services_not_found_shape() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);

    assert_minecraft_services_not_found!(
        app,
        get,
        "/api/yggdrasil/minecraftservices/asds",
        "/asds"
    );
    assert_minecraft_services_not_found!(app, get, "/api/yggdrasil/asds", "/asds");
}

#[actix_web::test]
async fn minecraft_services_anti_feature_policy_requires_valid_bearer_token() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let _access = setup_admin!(app);
    let login = ygg_login!(app, "admin@example.com", "policy-revoked-client");
    let revoked_token = login.access_token;

    for (uri, path) in [
        ("/api/yggdrasil/minecraftservices/privileges", "/privileges"),
        (
            "/api/yggdrasil/minecraftservices/player/attributes",
            "/player/attributes",
        ),
        (
            "/api/yggdrasil/minecraftservices/privacy/blocklist",
            "/privacy/blocklist",
        ),
    ] {
        let req = test::TestRequest::get().uri(uri).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401, "missing token should fail: {uri}");
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["path"], path);

        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(("Authorization", "Bearer not-a-token"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401, "invalid token should fail: {uri}");
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["path"], path);
    }

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/invalidate")
        .set_json(serde_json::json!({
            "accessToken": revoked_token,
            "clientToken": "policy-revoked-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    for (uri, path) in [
        ("/api/yggdrasil/minecraftservices/privileges", "/privileges"),
        (
            "/api/yggdrasil/minecraftservices/player/attributes",
            "/player/attributes",
        ),
        (
            "/api/yggdrasil/minecraftservices/privacy/blocklist",
            "/privacy/blocklist",
        ),
    ] {
        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(("Authorization", format!("Bearer {revoked_token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401, "revoked token should fail: {uri}");
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["path"], path);
    }
}

#[actix_web::test]
async fn minecraft_services_anti_feature_policy_wrong_methods_are_not_routed() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);

    for uri in [
        "/api/yggdrasil/minecraftservices/privileges",
        "/api/yggdrasil/minecraftservices/player/attributes",
        "/api/yggdrasil/minecraftservices/privacy/blocklist",
        "/api/yggdrasil/sessionserver/blockedservers",
    ] {
        let req = test::TestRequest::post().uri(uri).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404, "wrong method should not route: {uri}");
    }
}

#[actix_web::test]
async fn yggdrasil_dto_validation_uses_protocol_error_shape() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": "   "
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "IllegalArgumentException");
    assert!(
        body["errorMessage"]
            .as_str()
            .unwrap()
            .contains("value cannot be empty")
    );
    assert!(body["code"].is_null());
}

#[actix_web::test]
async fn yggdrasil_api_root_lives_under_subpath_and_frontend_keeps_root() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);

    for uri in ["/api/yggdrasil", "/api/yggdrasil/"] {
        let req = test::TestRequest::get().uri(uri).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-cache, no-store, must-revalidate")
        );
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["meta"]["implementationName"], "AsterYggdrasil");
        assert!(body["skinDomains"].as_array().is_some());
    }

    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("x-authlib-injector-api-location")
            .and_then(|value| value.to_str().ok()),
        Some("/api/yggdrasil/")
    );
    assert!(
        resp.headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html"))
    );
}

#[actix_web::test]
async fn minecraft_profile_management_validates_names_and_lists_profiles() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);

    for invalid_name in ["ab", "bad-name", "nonascii猫", "ABCDEFGHIJKLMNOPQ"] {
        let req = test::TestRequest::post()
            .uri("/api/v1/profiles/minecraft")
            .insert_header(common::bearer_header(&access))
            .set_json(serde_json::json!({ "name": invalid_name }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], "bad_request");
        assert!(
            body["msg"]
                .as_str()
                .unwrap()
                .contains("profile name must be 3-16 ASCII letters")
        );
    }

    let min_profile = create_profile!(app, &access, "Ab1");
    let max_profile = create_profile!(app, &access, "ABCDEFGHIJKLMNOP");

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 2);
    assert_eq!(body["data"]["limit"], 50);
    assert_eq!(body["data"]["offset"], 0);
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], min_profile);
    assert_eq!(items[0]["name"], "Ab1");
    assert_eq!(items[1]["id"], max_profile);
    assert_eq!(items[1]["name"], "ABCDEFGHIJKLMNOP");

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/minecraft?limit=9999&offset=1")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100);
    assert_eq!(body["data"]["offset"], 1);
    assert_eq!(body["data"]["total"], 2);
    let page_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(page_items.len(), 1);
    assert_eq!(page_items[0]["id"], max_profile);

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/minecraft?query=b1")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    let search_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(search_items.len(), 1);
    assert_eq!(search_items[0]["id"], min_profile);
    assert_eq!(search_items[0]["name"], "Ab1");

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/minecraft?query=mnop")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    let search_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(search_items[0]["id"], max_profile);

    let uuid_fragment = &min_profile[..12];
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/minecraft?query={uuid_fragment}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    let search_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(search_items[0]["id"], min_profile);

    let long_query = "a".repeat(65);
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/minecraft?query={long_query}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let user_id = body["data"]["id"].as_i64().unwrap();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/users/{user_id}/minecraft-profiles"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let admin_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(admin_items.len(), 2);
    assert_eq!(admin_items[0]["id"], min_profile);
    assert_eq!(admin_items[1]["id"], max_profile);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/users/{user_id}/minecraft-profiles?limit=1&offset=1"
        ))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 1);
    assert_eq!(body["data"]["offset"], 1);
    assert_eq!(body["data"]["total"], 2);
    let admin_page_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(admin_page_items.len(), 1);
    assert_eq!(admin_page_items[0]["id"], max_profile);

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/profiles/minecraft/{min_profile}"))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "NewName" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(!resp.status().is_success());

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items[0]["name"], "Ab1");
}

#[actix_web::test]
async fn minecraft_profile_duplicate_names_are_rejected_until_deleted() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);

    let first_profile = create_profile!(app, &access, "ReusedName");

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "ReusedName" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.name_taken");
    assert!(
        body["msg"]
            .as_str()
            .unwrap()
            .contains("profile name already exists")
    );

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/profiles/minecraft/{first_profile}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "ReusedName" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["name"], "ReusedName");
}

#[actix_web::test]
async fn minecraft_profile_rename_updates_name_and_temporarily_invalidates_bound_tokens() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(app, &access, "RenameOld");
    let login = ygg_login!(app, "admin@example.com", "rename-client");

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/profiles/minecraft/{profile_id}/name"))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "RenameNew" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["id"], profile_id);
    assert_eq!(body["data"]["name"], "RenameNew");

    let token = yggdrasil_token::Entity::find()
        .filter(
            yggdrasil_token::Column::AccessTokenHash.eq(sha256_hex(login.access_token.as_bytes())),
        )
        .one(state.writer_db())
        .await
        .unwrap()
        .expect("renamed profile token row should exist");
    assert!(token.revoked_at.is_none());
    assert!(
        token.temporarily_invalidated_at.is_some(),
        "bound token should become temporarily invalid after profile rename"
    );

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "rename-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "selectedProfile": profile_id,
            "serverId": "rename-server"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "rename-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["selectedProfile"]["id"], profile_id);
    assert_eq!(body["selectedProfile"]["name"], "RenameNew");
    let refreshed_access = body["accessToken"].as_str().unwrap().to_string();
    validate_ygg_token_status!(app, &refreshed_access, "rename-client", 204);

    audit_service::flush_global_audit_log_manager().await;
    let rename_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftProfileRename).await;
    assert_eq!(rename_entry.entity_type, "minecraft_profile");
    assert_eq!(rename_entry.entity_name.as_deref(), Some("RenameNew"));
    let details: Value = serde_json::from_str(rename_entry.details.as_ref().unwrap())
        .expect("profile rename audit details should be json");
    assert_eq!(details["profile_uuid"], profile_id);
    assert_eq!(details["old_profile_name"], "RenameOld");
    assert_eq!(details["new_profile_name"], "RenameNew");
    assert_eq!(details["temporarily_invalidated_token_count"], 1);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?limit=20")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let item = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "minecraft_profile_rename")
        .expect("profile rename audit entry should be listed");
    assert_eq!(
        item["presentation"]["detail"]["code"],
        "minecraft_profile_renamed"
    );
}

#[actix_web::test]
async fn minecraft_profile_rename_rejects_invalid_duplicate_missing_and_foreign_profiles() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let first_profile = create_profile!(app, &access, "RenameOne");
    let second_profile = create_profile!(app, &access, "RenameTwo");
    let user_access = register_user!(
        app,
        "rename-user",
        "rename-user@example.com",
        "password1234"
    );

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/profiles/minecraft/{first_profile}/name"))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "bad-name" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "bad_request");

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/profiles/minecraft/{first_profile}/name"))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "RenameTwo" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.name_taken");

    let req = test::TestRequest::put()
        .uri("/api/v1/profiles/minecraft/not-a-uuid/name")
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "RenameOk" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.uuid_invalid");

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/profiles/minecraft/{second_profile}/name"))
        .insert_header(common::bearer_header(&user_access))
        .set_json(serde_json::json!({ "name": "StolenName" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.not_found");

    let missing_uuid = "00000000000000000000000000000000";
    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/profiles/minecraft/{missing_uuid}/name"))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "RenameOk" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.not_found");
}

#[actix_web::test]
async fn minecraft_profile_rename_same_name_is_noop_for_tokens() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(app, &access, "SameRename");
    let login = ygg_login!(app, "admin@example.com", "same-rename-client");

    let req = test::TestRequest::put()
        .uri(&format!("/api/v1/profiles/minecraft/{profile_id}/name"))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "SameRename" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["id"], profile_id);
    assert_eq!(body["data"]["name"], "SameRename");

    let token = yggdrasil_token::Entity::find()
        .filter(
            yggdrasil_token::Column::AccessTokenHash.eq(sha256_hex(login.access_token.as_bytes())),
        )
        .one(state.writer_db())
        .await
        .unwrap()
        .expect("same-name rename token row should exist");
    assert!(token.revoked_at.is_none());
    assert!(token.temporarily_invalidated_at.is_none());
    validate_ygg_token_status!(app, &login.access_token, "same-rename-client", 204);

    audit_service::flush_global_audit_log_manager().await;
    let rename_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftProfileRename).await;
    let details: Value = serde_json::from_str(rename_entry.details.as_ref().unwrap())
        .expect("profile rename audit details should be json");
    assert_eq!(details["temporarily_invalidated_token_count"], 0);
}

#[actix_web::test]
async fn minecraft_profile_delete_unbinds_textures_keeps_wardrobe_revokes_tokens_and_writes_audit()
{
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(app, &access, "DeleteMe");
    let login = ygg_login!(&app, "admin@example.com", "delete-profile-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        None,
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);
    let textures = profile_textures!(app, &profile_id);
    let texture_hash = texture_hash_from_property(&textures, "SKIN").to_string();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/profiles/minecraft/{profile_id}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(body["data"]["items"].as_array().unwrap().is_empty());

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{texture_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let etag = resp
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("texture response should include etag")
        .to_owned();

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{texture_hash}"))
        .insert_header((header::IF_NONE_MATCH, etag))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 304);

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["limit"], 50);
    assert_eq!(body["data"]["offset"], 0);
    let wardrobe_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(wardrobe_items.len(), 1);
    assert_eq!(wardrobe_items[0]["hash"], texture_hash);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "delete-profile-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    audit_service::flush_global_audit_log_manager().await;
    let delete_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftProfileDelete).await;
    assert_eq!(delete_entry.entity_type, "minecraft_profile");
    assert_eq!(delete_entry.entity_name.as_deref(), Some("DeleteMe"));
    let details: Value = serde_json::from_str(delete_entry.details.as_ref().unwrap())
        .expect("profile delete details should be json");
    assert_eq!(details["profile_uuid"], profile_id);
    assert_eq!(details["profile_name"], "DeleteMe");
    assert_eq!(details["deleted_texture_count"], 1);
    assert_eq!(details["revoked_token_count"], 1);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?limit=20")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let item = body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "minecraft_profile_delete")
        .expect("profile delete audit entry should be listed");
    assert_eq!(
        item["presentation"]["detail"]["code"],
        "minecraft_profile_deleted"
    );
    assert_eq!(item["presentation"]["target"]["code"], "minecraft_profile");
}

#[actix_web::test]
async fn yggdrasil_join_has_joined_and_profile_query_use_cache_session() {
    let state = common::setup_with_memory_cache().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "JoinUser" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let profile_id = profile_body["data"]["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "password1234",
            "clientToken": "join-client",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let auth_body: Value = test::read_body_json(resp).await;
    let ygg_access = auth_body["accessToken"].as_str().unwrap().to_string();
    assert_eq!(auth_body["selectedProfile"]["id"], profile_id);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .peer_addr("127.0.0.1:23456".parse().unwrap())
        .set_json(serde_json::json!({
            "accessToken": ygg_access,
            "selectedProfile": profile_id,
            "serverId": "server-hash"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=JoinUser&serverId=server-hash")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let joined_body: Value = test::read_body_json(resp).await;
    assert_eq!(joined_body["id"], profile_id);
    assert_eq!(joined_body["name"], "JoinUser");
    let joined_textures = decode_textures_property(&joined_body);
    assert_default_skin_textures(&joined_textures, &profile_id, "JoinUser");
    let uploadable_property = joined_body["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["name"] == "uploadableTextures")
        .expect("uploadableTextures property should exist");
    assert!(
        uploadable_property["signature"]
            .as_str()
            .is_some_and(|signature| !signature.is_empty())
    );

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=JoinUser&serverId=server-hash&ip=127.0.0.1")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=JoinUser&serverId=server-hash&ip=127.0.0.2")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=JoinUser&serverId=missing")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}?unsigned=false"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    assert_eq!(profile_body["id"], profile_id);
    let default_textures = decode_textures_property(&profile_body);
    let default_skin_hash =
        assert_default_skin_textures(&default_textures, &profile_id, "JoinUser");
    assert!(
        profile_body["properties"]
            .as_array()
            .unwrap()
            .iter()
            .any(|property| property["name"] == "uploadableTextures"
                && property["value"] == "skin,cape")
    );
    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{default_skin_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("image/png")
    );
}

#[actix_web::test]
async fn yggdrasil_join_records_forwarded_ip_from_trusted_proxy() {
    let mut state = common::setup_with_memory_cache().await;
    let mut config = state.config.as_ref().clone();
    config.network_trust.trusted_proxies = vec!["10.0.0.0/8".to_string()];
    state.config = Arc::new(config);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(app, &access, "ProxyJoin");
    let login = ygg_login!(app, "admin@example.com", "trusted-proxy-join-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .peer_addr("10.0.0.5:23456".parse().unwrap())
        .insert_header(("X-Forwarded-For", "203.0.113.10, 198.51.100.2"))
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "selectedProfile": profile_id,
            "serverId": "trusted-proxy-server-hash"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=ProxyJoin&serverId=trusted-proxy-server-hash&ip=203.0.113.10")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=ProxyJoin&serverId=trusted-proxy-server-hash&ip=10.0.0.5")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);
}

#[actix_web::test]
async fn yggdrasil_join_ignores_forwarded_ip_from_untrusted_peer() {
    let mut state = common::setup_with_memory_cache().await;
    let mut config = state.config.as_ref().clone();
    config.network_trust.trusted_proxies = vec!["10.0.0.0/8".to_string()];
    state.config = Arc::new(config);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(app, &access, "UntrustedJoin");
    let login = ygg_login!(app, "admin@example.com", "untrusted-proxy-join-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .peer_addr("198.51.100.50:23456".parse().unwrap())
        .insert_header(("X-Forwarded-For", "203.0.113.10, 198.51.100.2"))
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "selectedProfile": profile_id,
            "serverId": "untrusted-proxy-server-hash"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=UntrustedJoin&serverId=untrusted-proxy-server-hash&ip=203.0.113.10")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=UntrustedJoin&serverId=untrusted-proxy-server-hash&ip=198.51.100.50")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn yggdrasil_has_joined_signs_texture_properties_for_server_validation() {
    let state = common::setup_with_memory_cache().await;
    configure_yggdrasil_public_site_url(&state).await;
    let private_key =
        aster_yggdrasil::services::yggdrasil_signature::generate_private_key_pem(2048)
            .expect("test signature key should generate");
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
        &private_key,
        None,
        None,
    )
    .await
    .expect("signature key config should update");
    state.runtime_config().apply(saved);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "JoinSkin");
    let login = ygg_login!(&app, "admin@example.com", "has-joined-signed-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        Some("slim"),
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    let public_key_pem = metadata["signaturePublickey"]
        .as_str()
        .expect("metadata public key should be a string");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .peer_addr("127.0.0.1:23456".parse().unwrap())
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "selectedProfile": profile_id,
            "serverId": "signed-server-hash"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=JoinSkin&serverId=signed-server-hash&ip=127.0.0.1")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let joined_body: Value = test::read_body_json(resp).await;
    assert_eq!(joined_body["id"], profile_id);
    assert_eq!(joined_body["name"], "JoinSkin");

    let properties = joined_body["properties"]
        .as_array()
        .expect("hasJoined profile should include properties");
    let textures_property = properties
        .iter()
        .find(|property| property["name"] == "textures")
        .expect("textures property should exist");
    let value = textures_property["value"].as_str().unwrap();
    let signature = textures_property["signature"]
        .as_str()
        .expect("textures property should include signature");
    verify_property_signature(public_key_pem, value, signature);

    let uploadable_property = properties
        .iter()
        .find(|property| property["name"] == "uploadableTextures")
        .expect("uploadableTextures property should exist");
    assert_eq!(uploadable_property["value"], "skin,cape");
    let uploadable_signature = uploadable_property["signature"]
        .as_str()
        .expect("uploadableTextures property should include signature");
    verify_property_signature(public_key_pem, "skin,cape", uploadable_signature);
}

#[actix_web::test]
async fn yggdrasil_texture_upload_public_read_profile_property_and_delete_flow() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "SkinUser");
    let login = ygg_login!(&app, "admin@example.com", "texture-client");

    let (content_type, body) = texture_multipart_body(Some("slim"), &png_texture(64, 64));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let textures_property = profile_body["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["name"] == "textures")
        .expect("textures property should exist");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(textures_property["value"].as_str().unwrap())
        .expect("textures property should be valid base64");
    let textures: Value =
        serde_json::from_slice(&decoded).expect("textures property should be valid json");
    assert_eq!(textures["profileId"], profile_id);
    assert_eq!(textures["profileName"], "SkinUser");
    assert_eq!(textures["textures"]["SKIN"]["metadata"]["model"], "slim");
    let texture_url = textures["textures"]["SKIN"]["url"].as_str().unwrap();
    assert!(texture_url.starts_with("http://localhost/api/yggdrasil/textures/"));
    let texture_hash = texture_url
        .rsplit('/')
        .next()
        .expect("texture url should end with hash");
    assert_eq!(texture_hash.len(), 64);

    let texture_path = texture_url
        .strip_prefix("http://localhost")
        .expect("test texture URL should use configured public_site_url origin");
    let req = test::TestRequest::get().uri(texture_path).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("image/png")
    );
    assert_eq!(
        resp.headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("public, max-age=31536000, immutable")
    );
    let body = test::read_body(resp).await;
    assert!(!body.is_empty());

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let textures = decode_textures_property(&profile_body);
    assert_default_skin_textures(&textures, &profile_id, "SkinUser");

    audit_service::flush_global_audit_log_manager().await;
    let upload_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftTextureUpload).await;
    assert_eq!(upload_entry.entity_type, "minecraft_texture");
    let upload_details: Value = serde_json::from_str(upload_entry.details.as_ref().unwrap())
        .expect("texture upload details should be json");
    assert_eq!(upload_details["texture_type"], "skin");
    assert_eq!(upload_details["texture_model"], "slim");
    assert_eq!(upload_details["texture_hash"], texture_hash);
    let delete_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftTextureDelete).await;
    assert_eq!(delete_entry.entity_type, "minecraft_texture");
    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let audit_body: Value = test::read_body_json(resp).await;
    let delete_presentation = audit_body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "minecraft_texture_delete")
        .and_then(|item| item.get("presentation"))
        .expect("delete audit presentation should exist");
    assert_eq!(
        delete_presentation["detail"]["code"],
        "minecraft_texture_deleted"
    );
}

#[actix_web::test]
async fn yggdrasil_startup_generates_persistent_signature_key_once() {
    let state = setup_yggdrasil().await;

    let stored =
        system_config_repo::find_by_key(state.writer_db(), YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY)
            .await
            .expect("signature key config query should succeed")
            .expect("signature key config should exist");

    assert!(stored.value.contains("BEGIN PRIVATE KEY"));
    assert_eq!(
        state
            .runtime_config()
            .get(YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY)
            .as_deref(),
        Some(stored.value.as_str())
    );
}

#[actix_web::test]
async fn yggdrasil_profile_textures_are_signed_with_persistent_runtime_key() {
    let state = setup_yggdrasil().await;
    let private_key =
        aster_yggdrasil::services::yggdrasil_signature::generate_private_key_pem(2048)
            .expect("test signature key should generate");
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
        &private_key,
        None,
        None,
    )
    .await
    .expect("signature key config should update");
    state.runtime_config().apply(saved);
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_PUBLIC_BASE_URL_KEY,
        r#"["https://skin.example.test/yggdrasil","https://fallback.example.test"]"#,
        None,
        None,
    )
    .await
    .expect("public base URL config should update");
    state.runtime_config().apply(saved);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "SignedSkin");
    let login = ygg_login!(&app, "admin@example.com", "signed-texture-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        Some("slim"),
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get().uri("/api/yggdrasil/").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata: Value = test::read_body_json(resp).await;
    let public_key_pem = metadata["signaturePublickey"]
        .as_str()
        .expect("metadata public key should be a string");
    assert!(public_key_pem.contains("BEGIN PUBLIC KEY"));

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}?unsigned=false"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let textures_property = profile_body["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["name"] == "textures")
        .expect("textures property should exist");
    let value = textures_property["value"].as_str().unwrap();
    let signature = textures_property["signature"]
        .as_str()
        .expect("textures property should include signature");
    let textures = decode_textures_property(&profile_body);
    assert!(
        textures["textures"]["SKIN"]["url"]
            .as_str()
            .unwrap()
            .starts_with("https://skin.example.test/yggdrasil/textures/")
    );

    verify_property_signature(public_key_pem, value, signature);
}

#[actix_web::test]
async fn yggdrasil_profile_textures_unsigned_queries_omit_signature() {
    let state = setup_yggdrasil().await;
    let private_key =
        aster_yggdrasil::services::yggdrasil_signature::generate_private_key_pem(2048)
            .expect("test signature key should generate");
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
        &private_key,
        None,
        None,
    )
    .await
    .expect("signature key config should update");
    state.runtime_config().apply(saved);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "UnsignedSkin");
    let login = ygg_login!(&app, "admin@example.com", "unsigned-texture-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        None,
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);

    for uri in [
        format!("/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"),
        format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}?unsigned=true"
        ),
    ] {
        let req = test::TestRequest::get().uri(&uri).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let profile_body: Value = test::read_body_json(resp).await;
        let textures_property = profile_body["properties"]
            .as_array()
            .unwrap()
            .iter()
            .find(|property| property["name"] == "textures")
            .expect("textures property should exist");
        assert!(
            textures_property.get("signature").is_none()
                || textures_property["signature"].is_null()
        );
    }
}

#[actix_web::test]
async fn yggdrasil_profile_textures_invalid_runtime_key_returns_protocol_error() {
    let state = setup_yggdrasil().await;
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
        "not a pem",
        None,
        None,
    )
    .await
    .expect("signature key config should update");
    state.runtime_config().apply(saved);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "BrokenKeySkin");
    let login = ygg_login!(&app, "admin@example.com", "broken-key-texture-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        None,
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}?unsigned=false"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        500,
        "InternalServerError",
        "invalid yggdrasil signature private key PEM",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_profile_textures_invalid_public_base_urls_fall_back_to_public_site_url() {
    let state = setup_yggdrasil().await;
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_PUBLIC_BASE_URL_KEY,
        r#"["ftp://invalid.example.test","not-a-url",""]"#,
        None,
        None,
    )
    .await
    .expect("public base URL config should update");
    state.runtime_config().apply(saved);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "RelativeSkin");
    let login = ygg_login!(&app, "admin@example.com", "relative-texture-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        None,
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let textures = decode_textures_property(&profile_body);

    assert!(
        textures["textures"]["SKIN"]["url"]
            .as_str()
            .unwrap()
            .starts_with("http://localhost/api/yggdrasil/textures/")
    );
}

#[actix_web::test]
async fn yggdrasil_profile_textures_require_public_url_configuration() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "MissingUrlSkin");
    let login = ygg_login!(
        &app,
        "admin@example.com",
        "missing-public-url-texture-client"
    );

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        None,
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        500,
        "InternalServerError",
        "public_site_url or yggdrasil_public_base_url must be configured",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_texture_upload_rejects_invalid_png_dimensions() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "BadSkin");
    let login = ygg_login!(&app, "admin@example.com", "bad-texture-client");

    let (content_type, body) = texture_multipart_body(None, &png_texture(63, 64));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "invalid skin texture dimensions",
    )
    .await;

    let (content_type, body) = texture_multipart_body(None, &png_texture(22, 17));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "invalid skin texture dimensions",
    )
    .await;

    let (content_type, body) = texture_multipart_body(None, &png_texture(64, 64));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/cape"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "invalid cape texture dimensions",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_texture_upload_rejects_streams_over_runtime_size_limit() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "LargeSkin");
    let login = ygg_login!(&app, "admin@example.com", "large-texture-client");
    let png = png_texture(64, 64);
    let max_upload_bytes =
        aster_yggdrasil::utils::numbers::usize_to_u64(png.len(), "test png size").unwrap() - 1;
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
        &max_upload_bytes.to_string(),
        None,
        None,
    )
    .await
    .expect("texture upload size config should update");
    state.runtime_config().apply(saved);

    let (content_type, body) = texture_multipart_body(None, &png);
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "Texture upload exceeds",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_texture_upload_rejects_png_header_over_runtime_pixel_limit() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "BombSkin");
    let login = ygg_login!(&app, "admin@example.com", "pixel-limit-client");
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_MAX_TEXTURE_PIXELS_KEY,
        &(64 * 64 - 1).to_string(),
        None,
        None,
    )
    .await
    .expect("texture pixel limit config should update");
    state.runtime_config().apply(saved);

    let (content_type, body) = texture_multipart_body(None, &png_texture(64, 64));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "texture dimensions exceed",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_texture_upload_rejects_auth_profile_and_multipart_edges() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "EdgeSkin");
    let login = ygg_login!(&app, "admin@example.com", "edge-client");
    let other_profile_id = create_profile!(&app, &access, "OtherSkin");

    let (content_type, body) = texture_multipart_body(None, &png_texture(64, 64));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Content-Type", content_type.clone()))
        .set_payload(body.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 401, "ForbiddenOperationException", "Invalid token").await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{other_profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type.clone()))
        .set_payload(body.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        403,
        "ForbiddenOperationException",
        "Profile does not belong",
    )
    .await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/elytra"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type.clone()))
        .set_payload(body.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "Invalid texture type",
    )
    .await;

    let (text_content_type, text_body) =
        texture_multipart_body_with_file_content_type(None, &png_texture(64, 64), "text/plain");
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", text_content_type))
        .set_payload(text_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "Texture file must be image/png",
    )
    .await;

    let (missing_file_content_type, missing_file_body) = texture_multipart_body_without_file();
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", missing_file_content_type))
        .set_payload(missing_file_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "Texture upload file is missing",
    )
    .await;

    let (bad_model_content_type, bad_model_body) =
        texture_multipart_body(Some("wide"), &png_texture(64, 64));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", bad_model_content_type))
        .set_payload(bad_model_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 400, "IllegalArgumentException", "Invalid skin model").await;

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 401, "ForbiddenOperationException", "Invalid token").await;

    let req = test::TestRequest::delete()
        .uri("/api/yggdrasil/api/user/profile/not-a-uuid/skin")
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "uuid must be a 32-character unsigned hexadecimal UUID",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_profile_without_skin_gets_embedded_default_skin_property() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let admin = user_repo::find_by_email(state.reader_db(), "admin@example.com")
        .await
        .expect("admin lookup should succeed")
        .expect("admin should exist");
    let steve_profile = minecraft_profile_repo::create(
        state.writer_db(),
        admin.id,
        "00000000000000000000000000000000",
        "DefaultSteve",
        MinecraftTextureModel::Default,
        "skin,cape",
    )
    .await
    .expect("fixed Steve profile should create");
    let alex_profile = minecraft_profile_repo::create(
        state.writer_db(),
        admin.id,
        "00000000000000000000000000000001",
        "DefaultAlex",
        MinecraftTextureModel::Default,
        "skin,cape",
    )
    .await
    .expect("fixed Alex profile should create");

    for (profile, expected_name, expected_hash) in [
        (&steve_profile, "DefaultSteve", DEFAULT_STEVE_SKIN_HASH),
        (&alex_profile, "DefaultAlex", DEFAULT_ALEX_SKIN_HASH),
    ] {
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/yggdrasil/sessionserver/session/minecraft/profile/{}",
                profile.uuid
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let profile_body: Value = test::read_body_json(resp).await;
        let textures = decode_textures_property(&profile_body);
        let actual_hash = assert_default_skin_textures(&textures, &profile.uuid, expected_name);
        assert_eq!(actual_hash, expected_hash);
        assert!(textures["textures"].get("CAPE").is_none());

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/v1/profiles/minecraft/{}/textures",
                profile.uuid
            ))
            .insert_header(common::bearer_header(&access))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        let items = body["data"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_default_skin_metadata(&items[0], &profile.uuid, expected_name, expected_hash);

        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/v1/admin/minecraft-profiles/{}/textures",
                profile.uuid
            ))
            .insert_header(common::bearer_header(&access))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        let items = body["data"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_default_skin_metadata(&items[0], &profile.uuid, expected_name, expected_hash);
    }
}

#[actix_web::test]
async fn yggdrasil_existing_skin_is_not_overwritten_by_default_skin() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "CustomSkin");
    let login = ygg_login!(&app, "admin@example.com", "custom-skin-client");

    let skin = png_texture_with_color(64, 64, image::Rgba([9, 18, 27, 255]));
    let resp = upload_texture_req!(app, &login.access_token, &profile_id, "skin", None, &skin);
    assert_eq!(resp.status(), 204);
    let textures = profile_textures!(app, &profile_id);
    let skin_hash = texture_hash_from_property(&textures, "SKIN");

    assert_ne!(skin_hash, DEFAULT_STEVE_SKIN_HASH);
    assert_ne!(skin_hash, DEFAULT_ALEX_SKIN_HASH);
    assert_eq!(textures["textures"].as_object().unwrap().len(), 1);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/minecraft/{profile_id}/textures"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["hash"], skin_hash);
    assert_eq!(items[0]["source"], "bound");
}

#[actix_web::test]
async fn yggdrasil_cape_only_profile_keeps_cape_and_adds_default_skin() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "CapeOnly");
    let login = ygg_login!(&app, "admin@example.com", "cape-only-client");

    let cape = png_texture_with_color(64, 32, image::Rgba([44, 55, 66, 255]));
    let resp = upload_texture_req!(app, &login.access_token, &profile_id, "cape", None, &cape);
    assert_eq!(resp.status(), 204);
    let textures = profile_textures!(app, &profile_id);
    let cape_hash = texture_hash_from_property(&textures, "CAPE");
    let default_hash = assert_default_skin_textures(&textures, &profile_id, "CapeOnly");

    assert_ne!(cape_hash, default_hash);
    assert!(textures["textures"]["CAPE"]["url"].as_str().is_some());
    assert_eq!(textures["textures"].as_object().unwrap().len(), 2);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/minecraft/{profile_id}/textures"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let skin = items
        .iter()
        .find(|item| item["texture_type"] == "skin")
        .expect("default skin metadata should be listed");
    assert_default_skin_metadata(skin, &profile_id, "CapeOnly", default_hash);
    let cape = items
        .iter()
        .find(|item| item["texture_type"] == "cape")
        .expect("cape metadata should be listed");
    assert_eq!(cape["hash"], cape_hash);
    assert_eq!(cape["source"], "bound");
}

#[actix_web::test]
async fn yggdrasil_embedded_default_skin_downloads_and_unknown_hashes_stay_404() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);

    for hash in [DEFAULT_STEVE_SKIN_HASH, DEFAULT_ALEX_SKIN_HASH] {
        let req = test::TestRequest::get()
            .uri(&format!("/api/yggdrasil/textures/{hash}"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("image/png")
        );
        assert_eq!(
            resp.headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("public, max-age=31536000, immutable")
        );
        let etag = resp
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .expect("embedded default skin response should include etag")
            .to_owned();
        let body = test::read_body(resp).await;
        assert_eq!(sha256_hex(&body), hash);
        image::load_from_memory(&body).expect("embedded default skin should decode");

        let req = test::TestRequest::get()
            .uri(&format!("/api/yggdrasil/textures/{hash}"))
            .insert_header((header::IF_NONE_MATCH, etag))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 304);
    }

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/textures/ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/textures/not-a-valid-hash")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn yggdrasil_default_skin_property_requires_public_texture_url_configuration() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "NoUrlDefault");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        500,
        "InternalServerError",
        "public_site_url or yggdrasil_public_base_url must be configured",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_texture_upload_obeys_runtime_upload_switches() {
    let state = setup_yggdrasil().await;
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        aster_yggdrasil::config::yggdrasil::YGGDRASIL_ALLOW_SKIN_UPLOAD_KEY,
        "false",
        None,
        None,
    )
    .await
    .expect("skin upload config should update");
    state.runtime_config().apply(saved);

    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "NoUpload");
    let login = ygg_login!(&app, "admin@example.com", "no-upload-client");

    let (content_type, body) = texture_multipart_body(None, &png_texture(64, 64));
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        403,
        "ForbiddenOperationException",
        "Texture upload is disabled",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_texture_cape_and_reupload_upsert_edges() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "CapeUser");
    let login = ygg_login!(&app, "admin@example.com", "cape-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "cape",
        None,
        &png_texture(22, 17)
    );
    assert_eq!(resp.status(), 204);
    let textures = profile_textures!(app, &profile_id);
    assert_eq!(
        textures["textures"]["CAPE"]["metadata"],
        Value::Null,
        "cape should not emit skin model metadata"
    );
    assert!(textures["textures"]["CAPE"]["url"].as_str().is_some());
    let cape_hash = texture_hash_from_property(&textures, "CAPE").to_string();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/profiles/minecraft/{profile_id}/textures"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let metadata_body: Value = test::read_body_json(resp).await;
    let cape_metadata = metadata_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["texture_type"] == "cape")
        .expect("cape metadata should be listed");
    assert_eq!(cape_metadata["hash"], cape_hash);
    assert_eq!(cape_metadata["width"], 64);
    assert_eq!(cape_metadata["height"], 32);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{cape_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let decoded_cape = image::load_from_memory(&body)
        .expect("served cape should decode")
        .to_rgba8();
    assert_eq!(decoded_cape.dimensions(), (64, 32));
    assert_eq!(
        *decoded_cape.get_pixel(0, 0),
        image::Rgba([128, 64, 32, 255])
    );
    assert_eq!(
        *decoded_cape.get_pixel(21, 16),
        image::Rgba([128, 64, 32, 255])
    );
    assert_eq!(*decoded_cape.get_pixel(22, 17), image::Rgba([0, 0, 0, 0]));
    assert_eq!(*decoded_cape.get_pixel(63, 31), image::Rgba([0, 0, 0, 0]));

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "cape",
        None,
        &png_texture(23, 17)
    );
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "invalid cape texture dimensions",
    )
    .await;

    let first_skin = png_texture(64, 32);
    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        None,
        &first_skin
    );
    assert_eq!(resp.status(), 204);
    let first = profile_textures!(app, &profile_id);
    let first_hash = texture_hash_from_property(&first, "SKIN");

    let second_skin = png_texture_with_color(64, 64, image::Rgba([1, 2, 3, 255]));
    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        None,
        &second_skin
    );
    assert_eq!(resp.status(), 204);
    let second = profile_textures!(app, &profile_id);
    let second_hash = texture_hash_from_property(&second, "SKIN");
    assert_ne!(first_hash, second_hash);

    let non_wardrobe_texture_count = minecraft_texture::Entity::find()
        .filter(minecraft_texture::Column::UserId.is_not_null())
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(false))
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(non_wardrobe_texture_count, 0);
    let wardrobe_texture_count = minecraft_texture::Entity::find()
        .filter(minecraft_texture::Column::UserId.is_not_null())
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(wardrobe_texture_count, 3);
    let profile_texture_count = minecraft_profile_texture::Entity::find()
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(profile_texture_count, 2);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{first_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{profile_id}/skin"
        ))
        .insert_header(("Authorization", format!("Bearer {}", login.access_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/textures/not-a-valid-hash")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/textures/0000000000000000000000000000000000000000000000000000000000000000")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn yggdrasil_shared_texture_blob_cleanup_keeps_referenced_hashes() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let first_profile = create_profile!(app, &access, "SharedOne");
    let second_profile = create_profile!(app, &access, "SharedTwo");
    let first_login = ygg_login_selected!(app, "shared-one", first_profile.as_str(), "SharedOne");
    let second_login = ygg_login_selected!(app, "shared-two", second_profile.as_str(), "SharedTwo");

    let shared_skin = png_texture_with_color(64, 64, image::Rgba([11, 22, 33, 255]));
    let resp = upload_texture_req!(
        app,
        &first_login.access_token,
        &first_profile,
        "skin",
        None,
        &shared_skin
    );
    assert_eq!(resp.status(), 204);
    let resp = upload_texture_req!(
        app,
        &second_login.access_token,
        &second_profile,
        "skin",
        None,
        &shared_skin
    );
    assert_eq!(resp.status(), 204);

    let first_textures = profile_textures!(app, &first_profile);
    let second_textures = profile_textures!(app, &second_profile);
    let shared_hash = texture_hash_from_property(&first_textures, "SKIN").to_string();
    assert_eq!(
        shared_hash,
        texture_hash_from_property(&second_textures, "SKIN")
    );

    let replacement_skin = png_texture_with_color(64, 64, image::Rgba([44, 55, 66, 255]));
    let resp = upload_texture_req!(
        app,
        &first_login.access_token,
        &first_profile,
        "skin",
        None,
        &replacement_skin
    );
    assert_eq!(resp.status(), 204);
    let first_textures = profile_textures!(app, &first_profile);
    let replacement_hash = texture_hash_from_property(&first_textures, "SKIN").to_string();
    assert_ne!(shared_hash, replacement_hash);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{shared_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{second_profile}/skin"
        ))
        .insert_header((
            "Authorization",
            format!("Bearer {}", second_login.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{shared_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{replacement_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{first_profile}/skin"
        ))
        .insert_header((
            "Authorization",
            format!("Bearer {}", first_login.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{replacement_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn yggdrasil_launcher_upload_registers_and_deduplicates_wardrobe_textures() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let first_profile = create_profile!(app, &access, "LauncherWardOne");
    let second_profile = create_profile!(app, &access, "LauncherWardTwo");
    let first_login = ygg_login_selected!(
        app,
        "launcher-ward-one",
        first_profile.as_str(),
        "LauncherWardOne"
    );
    let second_login = ygg_login_selected!(
        app,
        "launcher-ward-two",
        second_profile.as_str(),
        "LauncherWardTwo"
    );
    let skin = png_texture_with_color(64, 64, image::Rgba([91, 92, 93, 255]));

    let resp = upload_texture_req!(
        app,
        &first_login.access_token,
        &first_profile,
        "skin",
        Some("slim"),
        &skin
    );
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    let wardrobe_id = items[0]["id"].as_i64().unwrap();
    let wardrobe_hash = items[0]["hash"].as_str().unwrap().to_string();
    assert_eq!(items[0]["texture_type"], "skin");
    assert_eq!(items[0]["texture_model"], "slim");
    assert_eq!(items[0]["visibility"], "private");

    let resp = upload_texture_req!(
        app,
        &second_login.access_token,
        &second_profile,
        "skin",
        Some("slim"),
        &skin
    );
    assert_eq!(resp.status(), 204);
    let second_textures = profile_textures!(app, &second_profile);
    assert_eq!(
        texture_hash_from_property(&second_textures, "SKIN"),
        wardrobe_hash
    );
    assert_eq!(
        second_textures["textures"]["SKIN"]["metadata"]["model"],
        "slim"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], wardrobe_id);

    let texture_rows = minecraft_texture::Entity::find()
        .filter(minecraft_texture::Column::Hash.eq(&wardrobe_hash))
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(texture_rows, 1);
    let binding_count = minecraft_profile_texture::Entity::find()
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(binding_count, 2);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/yggdrasil/api/user/profile/{first_profile}/skin"
        ))
        .insert_header((
            "Authorization",
            format!("Bearer {}", first_login.access_token),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{wardrobe_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/wardrobe/textures/{wardrobe_id}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{second_profile}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let textures = decode_textures_property(&body);
    assert_default_skin_textures(&textures, &second_profile, "LauncherWardTwo");
}

#[actix_web::test]
async fn launcher_upload_wardrobe_dedupe_keeps_model_and_user_boundaries() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let admin_access = setup_admin!(app);
    let admin_profile = create_profile!(app, &admin_access, "BoundaryAdmin");
    let admin_login = ygg_login!(app, "admin@example.com", "boundary-admin");

    let user_access = register_user!(
        app,
        "boundary-user",
        "boundary-user@example.com",
        "password1234"
    );
    let user_profile = create_profile!(app, &user_access, "BoundaryUser");
    let user_login = ygg_login!(app, "boundary-user@example.com", "boundary-user");

    let skin = png_texture_with_color(64, 64, image::Rgba([12, 34, 56, 255]));
    for model in [Some("default"), Some("slim")] {
        let resp = upload_texture_req!(
            app,
            &admin_login.access_token,
            &admin_profile,
            "skin",
            model,
            &skin
        );
        assert_eq!(resp.status(), 204);
    }
    let resp = upload_texture_req!(
        app,
        &user_login.access_token,
        &user_profile,
        "skin",
        Some("default"),
        &skin
    );
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&admin_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let admin_body: Value = test::read_body_json(resp).await;
    let admin_items = admin_body["data"]["items"].as_array().unwrap();
    assert_eq!(admin_items.len(), 2);
    assert!(
        admin_items
            .iter()
            .any(|item| item["texture_model"] == "default")
    );
    assert!(
        admin_items
            .iter()
            .any(|item| item["texture_model"] == "slim")
    );
    let shared_hash = admin_items[0]["hash"].as_str().unwrap().to_string();
    assert!(admin_items.iter().all(|item| item["hash"] == shared_hash));

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures?limit=1&offset=1")
        .insert_header(common::bearer_header(&admin_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let page_body: Value = test::read_body_json(resp).await;
    assert_eq!(page_body["data"]["limit"], 1);
    assert_eq!(page_body["data"]["offset"], 1);
    assert_eq!(page_body["data"]["total"], 2);
    assert_eq!(page_body["data"]["items"].as_array().unwrap().len(), 1);

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&user_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let user_body: Value = test::read_body_json(resp).await;
    let user_items = user_body["data"]["items"].as_array().unwrap();
    assert_eq!(user_items.len(), 1);
    assert_eq!(user_items[0]["hash"], shared_hash);
    assert_eq!(user_items[0]["texture_model"], "default");

    let all_wardrobe_rows = minecraft_texture::Entity::find()
        .filter(minecraft_texture::Column::Hash.eq(&shared_hash))
        .filter(minecraft_texture::Column::IsWardrobeItem.eq(true))
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(all_wardrobe_rows, 3);
}

#[actix_web::test]
async fn wardrobe_texture_list_filters_by_type_keyword_pagination_and_user_scope() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let admin_access = setup_admin!(app);
    let user_access = register_user!(
        app,
        "wardrobe-filter-user",
        "wardrobe-filter-user@example.com",
        "password1234"
    );

    let default_skin = png_texture_with_color(64, 64, image::Rgba([201, 40, 51, 255]));
    let slim_skin = png_texture_with_color(64, 64, image::Rgba([52, 151, 88, 255]));
    let cape = png_texture_with_color(22, 17, image::Rgba([42, 82, 212, 255]));
    let other_user_skin = png_texture_with_color(64, 64, image::Rgba([11, 12, 13, 255]));

    let resp =
        upload_wardrobe_texture_req!(app, &admin_access, "skin", Some("default"), &default_skin);
    assert_eq!(resp.status(), 200);
    let default_body: Value = test::read_body_json(resp).await;
    let default_id = default_body["data"]["id"].as_i64().unwrap();
    let default_hash = default_body["data"]["hash"].as_str().unwrap().to_string();

    let resp = upload_wardrobe_texture_req!(app, &admin_access, "skin", Some("slim"), &slim_skin);
    assert_eq!(resp.status(), 200);
    let slim_body: Value = test::read_body_json(resp).await;
    let slim_id = slim_body["data"]["id"].as_i64().unwrap();
    let slim_hash = slim_body["data"]["hash"].as_str().unwrap().to_string();

    let resp = upload_wardrobe_texture_req!(app, &admin_access, "cape", Some("slim"), &cape);
    assert_eq!(resp.status(), 200);
    let cape_body: Value = test::read_body_json(resp).await;
    let cape_id = cape_body["data"]["id"].as_i64().unwrap();
    let cape_hash = cape_body["data"]["hash"].as_str().unwrap().to_string();
    assert_eq!(cape_body["data"]["texture_model"], "default");

    let resp =
        upload_wardrobe_texture_req!(app, &user_access, "skin", Some("default"), &other_user_skin);
    assert_eq!(resp.status(), 200);

    let resp =
        list_wardrobe_textures_req!(app, &admin_access, "/api/v1/wardrobe/textures?limit=100");
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 3);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 3);

    let resp = list_wardrobe_textures_req!(
        app,
        &admin_access,
        "/api/v1/wardrobe/textures?texture_type=skin"
    );
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["total"], 2);
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|item| item["texture_type"] == "skin"));

    let resp = list_wardrobe_textures_req!(
        app,
        &admin_access,
        "/api/v1/wardrobe/textures?texture_type=cape"
    );
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(items[0]["id"], cape_id);
    assert_eq!(items[0]["hash"], cape_hash);

    let hash_query = format!("/api/v1/wardrobe/textures?keyword={}", &default_hash[..16]);
    let resp = list_wardrobe_textures_req!(app, &admin_access, &hash_query);
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(items[0]["id"], default_id);

    let resp =
        list_wardrobe_textures_req!(app, &admin_access, "/api/v1/wardrobe/textures?keyword=slim");
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(items[0]["id"], slim_id);
    assert_eq!(items[0]["hash"], slim_hash);
    assert_eq!(items[0]["texture_model"], "slim");

    let resp =
        list_wardrobe_textures_req!(app, &admin_access, "/api/v1/wardrobe/textures?keyword=skin");
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["total"], 2);
    assert!(items.iter().all(|item| item["texture_type"] == "skin"));

    let resp = list_wardrobe_textures_req!(
        app,
        &admin_access,
        "/api/v1/wardrobe/textures?texture_type=skin&keyword=default"
    );
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(items[0]["id"], default_id);
    assert_eq!(items[0]["texture_model"], "default");

    let resp = list_wardrobe_textures_req!(
        app,
        &admin_access,
        "/api/v1/wardrobe/textures?texture_type=skin&keyword=%20%20"
    );
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 2);

    let resp = list_wardrobe_textures_req!(
        app,
        &admin_access,
        "/api/v1/wardrobe/textures?texture_type=skin&limit=1&offset=1"
    );
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["limit"], 1);
    assert_eq!(body["data"]["offset"], 1);
    assert_eq!(body["data"]["total"], 2);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["texture_type"], "skin");

    let resp =
        list_wardrobe_textures_req!(app, &user_access, "/api/v1/wardrobe/textures?keyword=skin");
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(body["data"]["total"], 1);
    assert_ne!(items[0]["id"], default_id);
    assert_ne!(items[0]["id"], slim_id);
    assert_ne!(items[0]["id"], cape_id);
}

#[actix_web::test]
async fn wardrobe_texture_list_filter_rejects_invalid_query_edges() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);

    let resp = list_wardrobe_textures_req!(
        app,
        &access,
        "/api/v1/wardrobe/textures?texture_type=elytra"
    );
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "request.malformed");
    assert!(body["msg"].as_str().unwrap().contains("elytra"));

    let long_keyword = "a".repeat(97);
    let uri = format!("/api/v1/wardrobe/textures?keyword={long_keyword}");
    let resp = list_wardrobe_textures_req!(app, &access, &uri);
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "bad_request");
    assert!(
        body["msg"]
            .as_str()
            .unwrap()
            .contains("keyword must not exceed 96 characters")
    );
}

#[actix_web::test]
async fn minecraft_texture_metadata_apis_list_current_user_and_admin_views() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "MetaSkin");
    let login = ygg_login!(&app, "admin@example.com", "metadata-client");

    let resp = upload_texture_req!(
        app,
        &login.access_token,
        &profile_id,
        "skin",
        Some("slim"),
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 204);
    let textures = profile_textures!(app, &profile_id);
    let texture_hash = texture_hash_from_property(&textures, "SKIN").to_string();

    for uri in [
        format!("/api/v1/profiles/minecraft/{profile_id}/textures"),
        format!("/api/v1/admin/minecraft-profiles/{profile_id}/textures"),
    ] {
        let req = test::TestRequest::get()
            .uri(&uri)
            .insert_header(common::bearer_header(&access))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        let items = body["data"]
            .as_array()
            .expect("textures should be an array");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["profile_uuid"], profile_id);
        assert_eq!(items[0]["profile_name"], "MetaSkin");
        assert_eq!(items[0]["hash"], texture_hash);
        assert_eq!(items[0]["texture_type"], "skin");
        assert_eq!(items[0]["texture_model"], "slim");
        assert_eq!(items[0]["width"], 64);
        assert_eq!(items[0]["height"], 64);
        assert_eq!(items[0]["mime_type"], "image/png");
        assert!(items[0]["file_size"].as_i64().unwrap() > 0);
        assert_eq!(
            items[0]["url"].as_str().unwrap(),
            format!("http://localhost/api/yggdrasil/textures/{texture_hash}")
        );
        assert!(items[0]["created_at"].as_str().is_some());
        assert!(items[0]["updated_at"].as_str().is_some());
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/profiles/minecraft/not-a-uuid/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/minecraft-profiles/not-a-uuid/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn admin_minecraft_profiles_can_be_listed_filtered_and_deleted() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let alpha = create_profile!(app, &access, "AdminAlpha");
    let beta = create_profile!(app, &access, "AdminBeta");
    let login = ygg_login_selected!(app, "admin-profile-delete", beta.as_str(), "AdminBeta");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let user_id = body["data"]["id"].as_i64().unwrap();

    for uri in [
        "/api/v1/admin/minecraft-profiles?query=alpha".to_string(),
        "/api/v1/admin/minecraft-profiles?name=AdminAlpha".to_string(),
        format!("/api/v1/admin/minecraft-profiles?uuid={alpha}"),
        format!("/api/v1/admin/minecraft-profiles?user_id={user_id}&limit=1"),
    ] {
        let req = test::TestRequest::get()
            .uri(&uri)
            .insert_header(common::bearer_header(&access))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        assert!(body["data"]["total"].as_u64().unwrap() >= 1);
        assert!(body["data"]["limit"].as_u64().unwrap() >= 1);
        assert!(
            body["data"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["uuid"] == alpha
                    && item["name"] == "AdminAlpha"
                    && item["user_id"] == user_id
                    && item["uploadable_textures"] == "skin,cape")
        );
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/minecraft-profiles?uuid=bad-uuid")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/admin/minecraft-profiles/{beta}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/minecraft-profiles?uuid={beta}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 0);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "admin-profile-delete"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    audit_service::flush_global_audit_log_manager().await;
    let delete_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftProfileDelete).await;
    assert_eq!(delete_entry.entity_name.as_deref(), Some("AdminBeta"));
    let details: Value = serde_json::from_str(delete_entry.details.as_ref().unwrap())
        .expect("profile delete audit details should be json");
    assert_eq!(details["profile_uuid"], beta);
    assert_eq!(details["profile_name"], "AdminBeta");
    assert_eq!(details["deleted_texture_count"], 0);
    assert_eq!(details["revoked_token_count"], 1);
}

#[actix_web::test]
async fn admin_can_view_single_minecraft_profile_details() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(app, &access, "DetailUser");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/minecraft-profiles/{profile_id}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["uuid"], profile_id);
    assert_eq!(body["data"]["name"], "DetailUser");
    assert_eq!(body["data"]["uploadable_textures"], "skin,cape");
    assert_eq!(body["data"]["texture_model"], "default");

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/minecraft-profiles/not-a-uuid")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.uuid_invalid");

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/admin/minecraft-profiles/{profile_id}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/minecraft-profiles/{profile_id}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.not_found");
}

#[actix_web::test]
async fn admin_can_rename_any_minecraft_profile_and_duplicate_names_are_rejected() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let first_profile = create_profile!(app, &access, "AdminRenameA");
    let second_profile = create_profile!(app, &access, "AdminRenameB");
    let login = ygg_login_selected!(
        app,
        "admin-rename-client",
        first_profile.as_str(),
        "AdminRenameA"
    );

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/admin/minecraft-profiles/{first_profile}/name"
        ))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "AdminRenamed" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["uuid"], first_profile);
    assert_eq!(body["data"]["name"], "AdminRenamed");

    audit_service::flush_global_audit_log_manager().await;
    let rename_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftProfileRename).await;
    assert_eq!(rename_entry.entity_name.as_deref(), Some("AdminRenamed"));

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "admin-rename-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/admin/minecraft-profiles/{first_profile}/name"
        ))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "AdminRenameB" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_profile.name_taken");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/admin/minecraft-profiles/{second_profile}/name"
        ))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "AdminRenameB" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["uuid"], second_profile);
    assert_eq!(body["data"]["name"], "AdminRenameB");
}

#[actix_web::test]
async fn admin_texture_deletes_by_profile_type_and_hash_with_audit_and_blob_cleanup() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let first_profile = create_profile!(app, &access, "AdminTexOne");
    let second_profile = create_profile!(app, &access, "AdminTexTwo");
    let first_login = ygg_login_selected!(
        app,
        "admin-texture-one",
        first_profile.as_str(),
        "AdminTexOne"
    );
    let second_login = ygg_login_selected!(
        app,
        "admin-texture-two",
        second_profile.as_str(),
        "AdminTexTwo"
    );

    let skin = png_texture_with_color(64, 64, image::Rgba([7, 8, 9, 255]));
    let resp = upload_texture_req!(
        app,
        &first_login.access_token,
        &first_profile,
        "skin",
        None,
        &skin
    );
    assert_eq!(resp.status(), 204);
    let resp = upload_texture_req!(
        app,
        &second_login.access_token,
        &second_profile,
        "skin",
        None,
        &skin
    );
    assert_eq!(resp.status(), 204);
    let shared_hash =
        texture_hash_from_property(&profile_textures!(app, &first_profile), "SKIN").to_string();

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/admin/minecraft-profiles/{first_profile}/textures/skin"
        ))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/minecraft-profiles/{first_profile}/textures"
        ))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(body["data"].as_array().unwrap().is_empty());

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{shared_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/admin/minecraft-profiles/{first_profile}/textures/skin"
        ))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/admin/minecraft-textures/{shared_hash}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let remaining_count = minecraft_texture::Entity::find()
        .filter(minecraft_texture::Column::Hash.eq(&shared_hash))
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(remaining_count, 0);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{shared_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let req = test::TestRequest::delete()
        .uri("/api/v1/admin/minecraft-profiles/00000000000000000000000000000000/textures/skin")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    audit_service::flush_global_audit_log_manager().await;
    let delete_count = audit_log::Entity::find()
        .filter(audit_log::Column::Action.eq(audit_service::AuditAction::MinecraftTextureDelete))
        .count(state.writer_db())
        .await
        .expect("audit delete count should query");
    assert_eq!(delete_count, 2);
}

#[actix_web::test]
async fn yggdrasil_actions_write_audit_entries_with_presentation() {
    let state = common::setup_with_memory_cache().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/profiles/minecraft")
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "name": "AuditMc" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let profile_id = profile_body["data"]["id"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "password1234",
            "clientToken": "audit-client",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let auth_body: Value = test::read_body_json(resp).await;
    let ygg_access = auth_body["accessToken"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": ygg_access,
            "clientToken": "audit-client",
            "requestUser": true
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let refresh_body: Value = test::read_body_json(resp).await;
    let refreshed_access = refresh_body["accessToken"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .peer_addr("127.0.0.1:23456".parse().unwrap())
        .set_json(serde_json::json!({
            "accessToken": refreshed_access,
            "selectedProfile": profile_id,
            "serverId": "audit-server"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/invalidate")
        .set_json(serde_json::json!({
            "accessToken": refreshed_access,
            "clientToken": "audit-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    audit_service::flush_global_audit_log_manager().await;

    let profile_entry =
        audit_entry(&state, audit_service::AuditAction::MinecraftProfileCreate).await;
    assert_eq!(profile_entry.entity_type, "minecraft_profile");
    assert_eq!(profile_entry.entity_name.as_deref(), Some("AuditMc"));
    let details: Value = serde_json::from_str(profile_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["profile_uuid"], profile_id);
    assert_eq!(details["profile_name"], "AuditMc");

    let auth_entry = audit_entry(&state, audit_service::AuditAction::YggdrasilAuthenticate).await;
    assert_eq!(auth_entry.entity_type, "yggdrasil_token");
    let details: Value = serde_json::from_str(auth_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["identifier"], "admin@example.com");
    assert_eq!(details["selected_profile_uuid"], profile_id);

    let refresh_entry =
        audit_entry(&state, audit_service::AuditAction::YggdrasilRefreshToken).await;
    assert_eq!(refresh_entry.entity_type, "yggdrasil_token");
    let details: Value = serde_json::from_str(refresh_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["profile_uuid"], profile_id);
    assert_eq!(details["profile_name"], "AuditMc");

    let join_entry = audit_entry(&state, audit_service::AuditAction::YggdrasilJoinServer).await;
    assert_eq!(join_entry.entity_type, "yggdrasil_session");
    let details: Value = serde_json::from_str(join_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["profile_name"], "AuditMc");
    assert!(details["server_id_hash"].as_str().unwrap().len() >= 32);

    let invalidate_entry =
        audit_entry(&state, audit_service::AuditAction::YggdrasilInvalidateToken).await;
    assert_eq!(invalidate_entry.entity_type, "yggdrasil_token");
    let details: Value =
        serde_json::from_str(invalidate_entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["profile_uuid"], profile_id);
    assert_eq!(details["profile_name"], "AuditMc");

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs?limit=50")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let items = body["data"]["items"].as_array().unwrap();
    let join = items
        .iter()
        .find(|item| item["action"] == "yggdrasil_join_server")
        .expect("join audit entry should be listed");
    assert_eq!(
        join["presentation"]["detail"]["code"],
        "yggdrasil_joined_server"
    );
    assert_eq!(join["presentation"]["target"]["code"], "yggdrasil_session");

    for (action, detail_code, target_code) in [
        (
            "minecraft_profile_create",
            "minecraft_profile_created",
            "minecraft_profile",
        ),
        (
            "yggdrasil_authenticate",
            "yggdrasil_authenticated",
            "yggdrasil_token",
        ),
        (
            "yggdrasil_refresh_token",
            "yggdrasil_token_refreshed",
            "yggdrasil_token",
        ),
        (
            "yggdrasil_invalidate_token",
            "yggdrasil_token_invalidated",
            "yggdrasil_token",
        ),
        (
            "yggdrasil_join_server",
            "yggdrasil_joined_server",
            "yggdrasil_session",
        ),
    ] {
        let item = items
            .iter()
            .find(|item| item["action"] == action)
            .unwrap_or_else(|| panic!("{action} audit entry should be listed"));
        assert_eq!(item["presentation"]["detail"]["code"], detail_code);
        assert_eq!(item["presentation"]["target"]["code"], target_code);
    }
}

#[actix_web::test]
async fn yggdrasil_authenticate_handles_no_profiles_and_multiple_profile_selection() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "password1234",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["availableProfiles"].as_array().unwrap().len(), 0);
    assert!(body.get("selectedProfile").is_none());
    let unselected_access = body["accessToken"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .set_json(serde_json::json!({
            "accessToken": unselected_access,
            "selectedProfile": "00000000000000000000000000000000",
            "serverId": "no-selected-profile"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    let first = create_profile!(app, &access, "MultiOne");
    let second = create_profile!(app, &access, "MultiTwo");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "password1234",
            "clientToken": "multi-client",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["availableProfiles"].as_array().unwrap().len(), 2);
    assert!(body.get("selectedProfile").is_none());
    let access_token = body["accessToken"].as_str().unwrap().to_string();

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": access_token,
            "clientToken": "multi-client",
            "selectedProfile": {
                "id": second,
                "name": "MultiTwo"
            }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["selectedProfile"]["id"], second);
    assert_ne!(body["selectedProfile"]["id"], first);
}

#[actix_web::test]
async fn yggdrasil_rejects_invalid_credentials_agent_client_token_and_bad_profile_selection() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile = create_profile!(app, &access, "RejectUser");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "wrong-password",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        403,
        "ForbiddenOperationException",
        "Invalid credentials",
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "password1234",
            "agent": { "name": "NotMinecraft", "version": 1 }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "agent name must be Minecraft",
    )
    .await;

    let login = ygg_login!(app, "admin@example.com", "reject-client");
    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/validate")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "wrong-client"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 403, "ForbiddenOperationException", "Invalid token").await;

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/refresh")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "clientToken": "reject-client",
            "selectedProfile": {
                "id": profile,
                "name": "RejectUser"
            }
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "Access token already has a profile assigned",
    )
    .await;
}

#[actix_web::test]
async fn yggdrasil_signout_revokes_all_tokens_and_records_audit() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let _profile = create_profile!(app, &access, "SignoutUser");

    let first = ygg_login!(app, "admin@example.com", "signout-one");
    let second = ygg_login!(app, "admin@example.com", "signout-two");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/signout")
        .set_json(serde_json::json!({
            "username": "admin@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    for (token, client) in [
        (first.access_token, "signout-one"),
        (second.access_token, "signout-two"),
    ] {
        let req = test::TestRequest::post()
            .uri("/api/yggdrasil/authserver/validate")
            .set_json(serde_json::json!({
                "accessToken": token,
                "clientToken": client
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    audit_service::flush_global_audit_log_manager().await;
    let entry = audit_entry(&state, audit_service::AuditAction::YggdrasilSignout).await;
    assert_eq!(entry.entity_type, "user");
    let details: Value = serde_json::from_str(entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["identifier"], "admin@example.com");
}

#[actix_web::test]
async fn yggdrasil_signout_accepts_profile_name_when_profile_name_login_is_enabled() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let _profile = create_profile!(app, &access, "SignoutProfile");

    let first = ygg_login!(app, "admin@example.com", "signout-profile-one");
    let second = ygg_login!(app, "admin@example.com", "signout-profile-two");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/signout")
        .set_json(serde_json::json!({
            "username": "SignoutProfile",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    for (token, client) in [
        (first.access_token, "signout-profile-one"),
        (second.access_token, "signout-profile-two"),
    ] {
        let req = test::TestRequest::post()
            .uri("/api/yggdrasil/authserver/validate")
            .set_json(serde_json::json!({
                "accessToken": token,
                "clientToken": client
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 403);
    }

    audit_service::flush_global_audit_log_manager().await;
    let entry = audit_entry(&state, audit_service::AuditAction::YggdrasilSignout).await;
    let details: Value = serde_json::from_str(entry.details.as_deref().unwrap()).unwrap();
    assert_eq!(details["identifier"], "SignoutProfile");
}

#[actix_web::test]
async fn yggdrasil_authenticate_rate_limit_uses_protocol_error_body() {
    let state = setup_yggdrasil_with_strict_auth_rate_limit().await;
    let app = create_test_app!(state);

    let first_resp = ygg_authenticate_attempt(&app, "limited@example.com").await;
    assert_yggdrasil_error(first_resp, 403, "ForbiddenOperationException").await;

    let different_user_resp = ygg_authenticate_attempt(&app, "other@example.com").await;
    assert_yggdrasil_error(different_user_resp, 403, "ForbiddenOperationException").await;

    let limited_resp = ygg_authenticate_attempt(&app, " LIMITED@example.com ").await;
    assert_yggdrasil_rate_limited(limited_resp).await;
}

#[actix_web::test]
async fn yggdrasil_signout_rate_limit_uses_protocol_error_body() {
    let state = setup_yggdrasil_with_strict_auth_rate_limit().await;
    let app = create_test_app!(state);

    let first_resp = ygg_signout_attempt(&app, "limited@example.com").await;
    assert_yggdrasil_error(first_resp, 403, "ForbiddenOperationException").await;

    let different_user_resp = ygg_signout_attempt(&app, "other@example.com").await;
    assert_yggdrasil_error(different_user_resp, 403, "ForbiddenOperationException").await;

    let limited_resp = ygg_signout_attempt(&app, " LIMITED@example.com ").await;
    assert_yggdrasil_rate_limited(limited_resp).await;
}

#[actix_web::test]
async fn yggdrasil_authenticate_and_signout_rate_limits_are_independent() {
    let state = setup_yggdrasil_with_strict_auth_rate_limit().await;
    let app = create_test_app!(state);

    let authenticate_first = ygg_authenticate_attempt(&app, "shared@example.com").await;
    assert_yggdrasil_error(authenticate_first, 403, "ForbiddenOperationException").await;

    let signout_first = ygg_signout_attempt(&app, "shared@example.com").await;
    assert_yggdrasil_error(signout_first, 403, "ForbiddenOperationException").await;

    let authenticate_limited = ygg_authenticate_attempt(&app, "shared@example.com").await;
    assert_yggdrasil_rate_limited(authenticate_limited).await;

    let signout_limited = ygg_signout_attempt(&app, "shared@example.com").await;
    assert_yggdrasil_rate_limited(signout_limited).await;
}

#[actix_web::test]
async fn yggdrasil_rate_limit_can_be_disabled_without_changing_protocol_errors() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);

    let first = ygg_authenticate_attempt(&app, "disabled@example.com").await;
    assert_yggdrasil_error(first, 403, "ForbiddenOperationException").await;

    let second = ygg_authenticate_attempt(&app, "disabled@example.com").await;
    assert_yggdrasil_error(second, 403, "ForbiddenOperationException").await;
}

#[actix_web::test]
async fn yggdrasil_authenticate_validation_errors_count_toward_account_rate_limit() {
    let state = setup_yggdrasil_with_strict_auth_rate_limit().await;
    let app = create_test_app!(state);

    let invalid_agent = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": "edge@example.com",
            "password": "wrong-password",
            "agent": { "name": "Minecraft", "version": 2 }
        }))
        .to_request();
    let invalid_agent_resp = test::call_service(&app, invalid_agent).await;
    assert_yggdrasil_error(invalid_agent_resp, 400, "IllegalArgumentException").await;

    let limited = ygg_authenticate_attempt(&app, "edge@example.com").await;
    assert_yggdrasil_rate_limited(limited).await;
}

async fn ygg_authenticate_attempt<S, B>(
    app: &S,
    username: &str,
) -> actix_web::dev::ServiceResponse<B>
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody + 'static,
{
    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/authenticate")
        .set_json(serde_json::json!({
            "username": username,
            "password": "wrong-password",
            "agent": { "name": "Minecraft", "version": 1 }
        }))
        .to_request();
    test::call_service(app, req).await
}

async fn ygg_signout_attempt<S, B>(app: &S, username: &str) -> actix_web::dev::ServiceResponse<B>
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody + 'static,
{
    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/signout")
        .set_json(serde_json::json!({
            "username": username,
            "password": "wrong-password"
        }))
        .to_request();
    test::call_service(app, req).await
}

async fn assert_yggdrasil_error<B>(
    resp: actix_web::dev::ServiceResponse<B>,
    status: u16,
    error_name: &str,
) where
    B: actix_web::body::MessageBody + 'static,
{
    assert_eq!(resp.status(), status);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], error_name);
    assert!(body["errorMessage"].is_string());
    assert!(body.get("code").is_none());
    assert!(body.get("msg").is_none());
    assert!(body.get("data").is_none());
}

async fn assert_yggdrasil_rate_limited<B>(resp: actix_web::dev::ServiceResponse<B>)
where
    B: actix_web::body::MessageBody + 'static,
{
    assert_eq!(resp.status(), 429);
    let retry_after = resp
        .headers()
        .get("Retry-After")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .expect("Retry-After should be a numeric number of seconds");
    assert!(retry_after > 0);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "TooManyRequestsException");
    assert!(
        body["errorMessage"]
            .as_str()
            .unwrap()
            .contains("Too many requests")
    );
    assert!(body.get("code").is_none());
    assert!(body.get("msg").is_none());
    assert!(body.get("data").is_none());
}

#[actix_web::test]
async fn yggdrasil_profile_lookup_batch_and_session_edges_follow_protocol_statuses() {
    let state = common::setup_with_memory_cache().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile = create_profile!(app, &access, "BatchUser");
    let login = ygg_login!(app, "admin@example.com", "batch-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/api/profiles/minecraft")
        .set_json(serde_json::json!(["BatchUser", "MissingUser"]))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["id"], profile);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/api/profiles/minecraft")
        .set_json(serde_json::json!(["BatchUser", "bad-name"]))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(resp, 400, "IllegalArgumentException", "Invalid request").await;

    let too_many = (0..101).map(|idx| format!("User{idx}")).collect::<Vec<_>>();
    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/api/profiles/minecraft")
        .set_json(&too_many)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "Too many profile names requested",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/profile/00000000000000000000000000000000")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/profile/not-a-uuid")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "selectedProfile": "not-a-uuid",
            "serverId": "bad-selected-profile"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "uuid must be a 32-character unsigned hexadecimal UUID",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=bad-name&serverId=server")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ygg_error(
        resp,
        400,
        "IllegalArgumentException",
        "profile name must be 3-16 ASCII letters",
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .peer_addr("127.0.0.1:22222".parse().unwrap())
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "selectedProfile": profile,
            "serverId": "ip-sensitive"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=BatchUser&serverId=ip-sensitive&ip=203.0.113.9")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/authserver/invalidate")
        .set_json(serde_json::json!({
            "accessToken": "unknown-but-non-blank",
            "clientToken": "anything"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);
}

#[actix_web::test]
async fn yggdrasil_join_works_when_cache_disabled_because_memory_fallback_is_used() {
    let state = setup_yggdrasil().await;
    assert_eq!(state.cache.backend_name(), "memory");
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile = create_profile!(app, &access, "CacheUser");
    let login = ygg_login!(app, "admin@example.com", "cache-client");

    let req = test::TestRequest::post()
        .uri("/api/yggdrasil/sessionserver/session/minecraft/join")
        .set_json(serde_json::json!({
            "accessToken": login.access_token,
            "selectedProfile": profile,
            "serverId": "cache-fallback"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(
            "/api/yggdrasil/sessionserver/session/minecraft/hasJoined?username=CacheUser&serverId=cache-fallback",
        )
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn yggdrasil_tokens_are_hashed_and_pruned_by_runtime_limit() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let _profile = create_profile!(app, &access, "TokenUser");

    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        aster_yggdrasil::config::yggdrasil::YGGDRASIL_MAX_ACTIVE_TOKENS_KEY,
        "2",
        None,
        None,
    )
    .await
    .unwrap();
    state.runtime_config().apply(saved);

    let first = ygg_login!(app, "admin@example.com", "token-one");
    let _second = ygg_login!(app, "admin@example.com", "token-two");
    let third = ygg_login!(app, "admin@example.com", "token-three");

    let active_count = yggdrasil_token::Entity::find()
        .filter(yggdrasil_token::Column::RevokedAt.is_null())
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(active_count, 2);
    assert!(
        yggdrasil_token::Entity::find()
            .filter(yggdrasil_token::Column::AccessTokenHash.eq(&first.access_token))
            .one(state.writer_db())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        yggdrasil_token::Entity::find()
            .filter(
                yggdrasil_token::Column::AccessTokenHash
                    .eq(sha256_hex(third.access_token.as_bytes()))
            )
            .one(state.writer_db())
            .await
            .unwrap()
            .is_some()
    );
}

async fn audit_entry(
    state: &aster_yggdrasil::runtime::AppState,
    action: audit_service::AuditAction,
) -> audit_log::Model {
    audit_log::Entity::find()
        .filter(audit_log::Column::Action.eq(action))
        .one(state.writer_db())
        .await
        .expect("audit query should succeed")
        .expect("audit entry should exist")
}

struct YggLogin {
    access_token: String,
}

async fn assert_ygg_error<B>(
    resp: actix_web::dev::ServiceResponse<B>,
    status: u16,
    error: &str,
    message_contains: &str,
) where
    B: actix_web::body::MessageBody + 'static,
{
    assert_eq!(resp.status().as_u16(), status);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], error);
    assert!(
        body["errorMessage"]
            .as_str()
            .unwrap()
            .contains(message_contains),
        "expected errorMessage to contain {message_contains:?}, got {:?}",
        body["errorMessage"]
    );
}

fn assert_unsigned_uuid(value: &str) {
    assert_eq!(value.len(), 32);
    assert!(
        value.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "expected unsigned UUID hex string, got {value:?}"
    );
}

fn png_texture(width: u32, height: u32) -> Vec<u8> {
    png_texture_with_color(width, height, image::Rgba([128, 64, 32, 255]))
}

fn png_texture_with_color(width: u32, height: u32, color: image::Rgba<u8>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let image = image::RgbaImage::from_pixel(width, height, color);
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("test png should encode");
    bytes
}

#[actix_web::test]
async fn wardrobe_upload_can_be_bound_and_unbound_from_profile_later() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "WardrobeUser");

    let (content_type, body) = texture_multipart_body(Some("slim"), &png_texture(64, 64));
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let upload_body: Value = test::read_body_json(resp).await;
    let wardrobe_texture_id = upload_body["data"]["id"].as_i64().unwrap();
    let wardrobe_hash = upload_body["data"]["hash"].as_str().unwrap().to_string();
    assert_eq!(upload_body["data"]["texture_type"], "skin");
    assert_eq!(upload_body["data"]["texture_model"], "slim");
    assert_eq!(upload_body["data"]["visibility"], "private");

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let list_body: Value = test::read_body_json(resp).await;
    assert_eq!(list_body["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(list_body["data"]["items"][0]["id"], wardrobe_texture_id);
    assert_eq!(list_body["data"]["items"][0]["visibility"], "private");

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{wardrobe_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let textures = decode_textures_property(&profile_body);
    assert_default_skin_textures(&textures, &profile_id, "WardrobeUser");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{profile_id}/textures/skin"
        ))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "texture_id": wardrobe_texture_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let bind_body: Value = test::read_body_json(resp).await;
    assert_eq!(bind_body["data"]["hash"], wardrobe_hash);
    assert_eq!(bind_body["data"]["profile_uuid"], profile_id);

    let textures = profile_textures!(app, &profile_id);
    assert_eq!(texture_hash_from_property(&textures, "SKIN"), wardrobe_hash);
    assert_eq!(textures["textures"]["SKIN"]["metadata"]["model"], "slim");

    audit_service::flush_global_audit_log_manager().await;
    let bind_entry = audit_entry(&state, audit_service::AuditAction::MinecraftTextureBind).await;
    assert_eq!(bind_entry.entity_type, "minecraft_texture");
    assert_eq!(bind_entry.entity_name.as_deref(), Some("WardrobeUser"));
    let bind_details: Value = serde_json::from_str(bind_entry.details.as_ref().unwrap())
        .expect("texture bind details should be json");
    assert_eq!(bind_details["profile_uuid"], profile_id);
    assert_eq!(bind_details["profile_name"], "WardrobeUser");
    assert_eq!(bind_details["texture_type"], "skin");
    assert_eq!(bind_details["texture_model"], "slim");
    assert_eq!(bind_details["texture_hash"], wardrobe_hash);
    assert_eq!(bind_details["width"], 64);
    assert_eq!(bind_details["height"], 64);
    assert!(bind_details["file_size"].as_i64().unwrap() > 0);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/audit-logs")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let audit_body: Value = test::read_body_json(resp).await;
    let bind_presentation = audit_body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "minecraft_texture_bind")
        .and_then(|item| item.get("presentation"))
        .expect("bind audit presentation should exist");
    assert_eq!(
        bind_presentation["summary"]["code"],
        "minecraft_texture_bind"
    );
    assert_eq!(
        bind_presentation["detail"]["code"],
        "minecraft_texture_bound"
    );

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/wardrobe/textures/{wardrobe_texture_id}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let list_body: Value = test::read_body_json(resp).await;
    assert_eq!(list_body["data"]["items"].as_array().unwrap().len(), 0);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/yggdrasil/sessionserver/session/minecraft/profile/{profile_id}"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let profile_body: Value = test::read_body_json(resp).await;
    let textures = decode_textures_property(&profile_body);
    assert_default_skin_textures(&textures, &profile_id, "WardrobeUser");

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{wardrobe_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let replacement_skin = png_texture_with_color(64, 64, image::Rgba([22, 33, 44, 255]));
    let resp =
        upload_wardrobe_texture_req!(app, &access, "skin", Some("default"), &replacement_skin);
    assert_eq!(resp.status(), 200);
    let replacement_body: Value = test::read_body_json(resp).await;
    let replacement_id = replacement_body["data"]["id"].as_i64().unwrap();
    let replacement_hash = replacement_body["data"]["hash"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{profile_id}/textures/skin"
        ))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "texture_id": replacement_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{profile_id}/textures/skin"
        ))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{replacement_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn wardrobe_upload_accepts_public_visibility_and_rejects_unknown_values() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);

    let public_skin = png_texture_with_color(64, 64, image::Rgba([66, 77, 88, 255]));
    let (content_type, body) =
        texture_multipart_body_with_visibility(Some("default"), Some("public"), &public_skin);
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let upload_body: Value = test::read_body_json(resp).await;
    let public_texture_id = upload_body["data"]["id"].as_i64().unwrap();
    assert_eq!(upload_body["data"]["visibility"], "public");

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let list_body: Value = test::read_body_json(resp).await;
    let listed = list_body["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == public_texture_id)
        .expect("public wardrobe texture should be listed");
    assert_eq!(listed["visibility"], "public");

    let (content_type, body) = texture_multipart_body_with_visibility(
        Some("default"),
        Some("friends-only"),
        &png_texture(64, 64),
    );
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_texture.invalid_dimensions");
}

#[actix_web::test]
async fn wardrobe_upload_rejects_streams_over_runtime_size_limit() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state.clone());
    let access = setup_admin!(app);
    let png = png_texture(64, 64);
    let max_upload_bytes =
        aster_yggdrasil::utils::numbers::usize_to_u64(png.len(), "test png size").unwrap() - 1;
    let saved = system_config_repo::upsert_with_options(
        state.writer_db(),
        YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES_KEY,
        &max_upload_bytes.to_string(),
        None,
        None,
    )
    .await
    .expect("texture upload size config should update");
    state.runtime_config().apply(saved);

    let (content_type, body) = texture_multipart_body(None, &png);
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "minecraft_texture.invalid_dimensions");
    assert!(
        body["msg"]
            .as_str()
            .unwrap()
            .contains("Texture upload exceeds")
    );
}

#[actix_web::test]
async fn wardrobe_texture_api_rejects_invalid_upload_and_auth_edges() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let access = setup_admin!(app);
    let profile_id = create_profile!(&app, &access, "WardrobeEdges");

    let req = test::TestRequest::get()
        .uri("/api/v1/wardrobe/textures")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let req = test::TestRequest::delete()
        .uri("/api/v1/wardrobe/textures/1")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let (content_type, body) = texture_multipart_body(None, &png_texture(64, 64));
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let (content_type, body) = texture_multipart_body(None, &png_texture(64, 64));
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/elytra")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let (content_type, body) = texture_multipart_body(None, &png_texture(63, 64));
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let (content_type, body) = texture_multipart_body_without_file();
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let (content_type, body) =
        texture_multipart_body_with_file_content_type(None, &png_texture(64, 64), "text/plain");
    let req = test::TestRequest::post()
        .uri("/api/v1/wardrobe/textures/skin")
        .insert_header(common::bearer_header(&access))
        .insert_header(("Content-Type", content_type))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::delete()
        .uri("/api/v1/wardrobe/textures/-1")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::delete()
        .uri("/api/v1/wardrobe/textures/999999")
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{profile_id}/textures/skin"
        ))
        .insert_header(common::bearer_header(&access))
        .set_json(serde_json::json!({ "texture_id": 0 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let user_access = register_user!(
        app,
        "wardrobe-delete-user",
        "wardrobe-delete-user@example.com",
        "password1234"
    );
    let user_skin = png_texture_with_color(64, 64, image::Rgba([9, 10, 11, 255]));
    let resp = upload_wardrobe_texture_req!(app, &user_access, "skin", None, &user_skin);
    assert_eq!(resp.status(), 200);
    let user_upload_body: Value = test::read_body_json(resp).await;
    let user_texture_id = user_upload_body["data"]["id"].as_i64().unwrap();
    let user_texture_hash = user_upload_body["data"]["hash"]
        .as_str()
        .unwrap()
        .to_string();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/wardrobe/textures/{user_texture_id}"))
        .insert_header(common::bearer_header(&access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{user_texture_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/wardrobe/textures/{user_texture_id}"))
        .insert_header(common::bearer_header(&user_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);

    let req = test::TestRequest::get()
        .uri(&format!("/api/yggdrasil/textures/{user_texture_hash}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/wardrobe/textures/{user_texture_id}"))
        .insert_header(common::bearer_header(&user_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn wardrobe_texture_binding_enforces_owner_profile_and_type_boundaries() {
    let state = setup_yggdrasil().await;
    let app = create_test_app!(state);
    let admin_access = setup_admin!(app);
    let admin_profile = create_profile!(&app, &admin_access, "OwnerSkin");
    let user_access = register_user!(
        app,
        "wardrobe-user",
        "wardrobe-user@example.com",
        "password1234"
    );
    let user_profile = create_profile!(&app, &user_access, "OtherOwner");

    let resp = upload_wardrobe_texture_req!(
        app,
        &user_access,
        "skin",
        Some("slim"),
        &png_texture(64, 64)
    );
    assert_eq!(resp.status(), 200);
    let user_upload_body: Value = test::read_body_json(resp).await;
    let other_user_texture_id = user_upload_body["data"]["id"].as_i64().unwrap();

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{admin_profile}/textures/skin"
        ))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(serde_json::json!({ "texture_id": other_user_texture_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let resp = upload_wardrobe_texture_req!(
        app,
        &admin_access,
        "cape",
        Some("slim"),
        &png_texture(22, 17)
    );
    assert_eq!(resp.status(), 200);
    let admin_cape_body: Value = test::read_body_json(resp).await;
    let admin_cape_id = admin_cape_body["data"]["id"].as_i64().unwrap();
    assert_eq!(admin_cape_body["data"]["texture_model"], "default");

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{admin_profile}/textures/skin"
        ))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(serde_json::json!({ "texture_id": admin_cape_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{user_profile}/textures/cape"
        ))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(serde_json::json!({ "texture_id": admin_cape_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/v1/profiles/minecraft/{admin_profile}/textures/elytra"
        ))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(serde_json::json!({ "texture_id": admin_cape_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

fn texture_multipart_body(model: Option<&str>, png: &[u8]) -> (String, Vec<u8>) {
    texture_multipart_body_with_options(model, None, png, "image/png")
}

fn texture_multipart_body_with_visibility(
    model: Option<&str>,
    visibility: Option<&str>,
    png: &[u8],
) -> (String, Vec<u8>) {
    texture_multipart_body_with_options(model, visibility, png, "image/png")
}

fn texture_multipart_body_with_file_content_type(
    model: Option<&str>,
    png: &[u8],
    file_content_type: &str,
) -> (String, Vec<u8>) {
    texture_multipart_body_with_options(model, None, png, file_content_type)
}

fn texture_multipart_body_with_options(
    model: Option<&str>,
    visibility: Option<&str>,
    png: &[u8],
    file_content_type: &str,
) -> (String, Vec<u8>) {
    let boundary = format!("boundary-{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::new();
    if let Some(model) = model {
        extend_ascii(&mut body, &format!("--{boundary}\r\n"));
        extend_ascii(
            &mut body,
            "Content-Disposition: form-data; name=\"model\"\r\n\r\n",
        );
        extend_ascii(&mut body, model);
        extend_ascii(&mut body, "\r\n");
    }
    if let Some(visibility) = visibility {
        extend_ascii(&mut body, &format!("--{boundary}\r\n"));
        extend_ascii(
            &mut body,
            "Content-Disposition: form-data; name=\"visibility\"\r\n\r\n",
        );
        extend_ascii(&mut body, visibility);
        extend_ascii(&mut body, "\r\n");
    }
    extend_ascii(&mut body, &format!("--{boundary}\r\n"));
    extend_ascii(
        &mut body,
        "Content-Disposition: form-data; name=\"file\"; filename=\"texture.png\"\r\n",
    );
    extend_ascii(
        &mut body,
        &format!("Content-Type: {file_content_type}\r\n\r\n"),
    );
    body.extend_from_slice(png);
    extend_ascii(&mut body, "\r\n");
    extend_ascii(&mut body, &format!("--{boundary}--\r\n"));
    (format!("multipart/form-data; boundary={boundary}"), body)
}

fn texture_multipart_body_without_file() -> (String, Vec<u8>) {
    let boundary = format!("boundary-{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::new();
    extend_ascii(&mut body, &format!("--{boundary}\r\n"));
    extend_ascii(
        &mut body,
        "Content-Disposition: form-data; name=\"model\"\r\n\r\n",
    );
    extend_ascii(&mut body, "slim\r\n");
    extend_ascii(&mut body, &format!("--{boundary}--\r\n"));
    (format!("multipart/form-data; boundary={boundary}"), body)
}

fn decode_textures_property(profile_body: &Value) -> Value {
    let textures_property = profile_body["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["name"] == "textures")
        .expect("textures property should exist");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(textures_property["value"].as_str().unwrap())
        .expect("textures property should be valid base64");
    serde_json::from_slice(&decoded).expect("textures property should be valid json")
}

fn assert_default_skin_textures(
    textures: &Value,
    profile_id: &str,
    profile_name: &str,
) -> &'static str {
    assert_eq!(textures["profileId"], profile_id);
    assert_eq!(textures["profileName"], profile_name);
    let skin = &textures["textures"]["SKIN"];
    let texture_url = skin["url"].as_str().expect("default skin should have url");
    assert!(texture_url.starts_with("http://localhost/api/yggdrasil/textures/"));
    let hash = texture_url
        .rsplit('/')
        .next()
        .expect("default skin texture url should end with hash");
    let expected_hash = expected_default_skin_hash(profile_id);
    assert_eq!(hash, expected_hash);
    if expected_hash == DEFAULT_ALEX_SKIN_HASH {
        assert_eq!(skin["metadata"]["model"], "slim");
    } else {
        assert!(skin.get("metadata").is_none() || skin["metadata"].is_null());
    }
    expected_hash
}

fn assert_default_skin_metadata(
    texture: &Value,
    profile_id: &str,
    profile_name: &str,
    expected_hash: &str,
) {
    assert_eq!(texture["id"], 0);
    assert_eq!(texture["profile_uuid"], profile_id);
    assert_eq!(texture["profile_name"], profile_name);
    assert_eq!(texture["hash"], expected_hash);
    assert_eq!(texture["texture_type"], "skin");
    assert_eq!(texture["visibility"], "public");
    assert_eq!(texture["mime_type"], "image/png");
    assert_eq!(texture["source"], "default");
    assert!(texture["file_size"].as_i64().unwrap() > 0);
    assert!(texture["width"].as_i64().unwrap() > 0);
    assert!(texture["height"].as_i64().unwrap() > 0);
    assert_eq!(
        texture["url"],
        format!("http://localhost/api/yggdrasil/textures/{expected_hash}")
    );
    if expected_hash == DEFAULT_ALEX_SKIN_HASH {
        assert_eq!(texture["texture_model"], "slim");
    } else {
        assert_eq!(texture["texture_model"], "default");
    }
}

fn expected_default_skin_hash(profile_id: &str) -> &'static str {
    let uuid = uuid::Uuid::parse_str(profile_id).expect("profile id should parse as UUID");
    if uuid.as_u128() & 1 == 1 {
        DEFAULT_ALEX_SKIN_HASH
    } else {
        DEFAULT_STEVE_SKIN_HASH
    }
}

fn texture_hash_from_property<'a>(textures: &'a Value, key: &str) -> &'a str {
    textures["textures"][key]["url"]
        .as_str()
        .unwrap()
        .rsplit('/')
        .next()
        .expect("texture url should end with hash")
}

fn verify_property_signature(public_key_pem: &str, value: &str, signature: &str) {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;

    let public_key = rsa::RsaPublicKey::from_public_key_pem(public_key_pem)
        .expect("metadata public key should parse");
    let verifying_key = rsa::pkcs1v15::VerifyingKey::<sha1::Sha1>::new(public_key);
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature)
        .expect("signature should be base64");
    let signature = rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
        .expect("signature bytes should parse");
    verifying_key
        .verify(value.as_bytes(), &signature)
        .expect("profile property signature should verify");
}

fn extend_ascii(target: &mut Vec<u8>, value: &str) {
    target.extend_from_slice(value.as_bytes());
}
