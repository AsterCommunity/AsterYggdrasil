//! Integration tests for local auth routes.

#[macro_use]
mod common;

use actix_web::{body::MessageBody, cookie::SameSite, http::header, test};
use aster_yggdrasil::api::error_code::AsterErrorCode;
use aster_yggdrasil::config::avatar::AVATAR_DIR_KEY;
use aster_yggdrasil::db::repository::{
    auth_session_repo, passkey_repo, system_config_repo, user_repo,
};
use aster_yggdrasil::entities::{auth_session, passkey};
use aster_yggdrasil::services::auth_service::AccessClaims;
use aster_yggdrasil::types::{StoredPasskeyCredential, TokenType, UserRole, UserStatus};
use base64::Engine as _;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, Validation};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::Value;
use std::io::Cursor;
use webauthn_authenticator_rs::prelude::{Url, WebauthnAuthenticator};
use webauthn_authenticator_rs::softpasskey::SoftPasskey;
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};
use webauthn_rs_proto::{AllowCredentials, Mediation, ResidentKeyRequirement};

const TEST_BROWSER_ORIGIN: &str = "http://localhost:8080";

fn decode_test_claims(token: &str) -> AccessClaims {
    jsonwebtoken::decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(b"test-secret-key-for-integration-tests"),
        &Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .expect("test token should decode")
    .claims
}

macro_rules! login_session {
    ($app:expr, $identifier:expr, $password:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/login")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({
                "identifier": $identifier,
                "password": $password
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200, "login should return 200");
        let access = common::extract_cookie(&resp, "aster_access").expect("access cookie missing");
        let refresh =
            common::extract_cookie(&resp, "aster_refresh").expect("refresh cookie missing");
        let csrf = common::extract_cookie(&resp, "aster_csrf").expect("csrf cookie missing");
        (access, refresh, csrf)
    }};
}

fn access_and_csrf_cookie_header(access_token: &str, csrf_token: &str) -> String {
    format!("aster_access={access_token}; aster_csrf={csrf_token}")
}

fn configure_passkey_public_site_url(state: &aster_yggdrasil::runtime::AppState) {
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
        r#"["http://localhost:8080"]"#,
    ));
}

async fn admin_user_id<C: sea_orm::ConnectionTrait>(db: &C) -> i64 {
    aster_yggdrasil::db::repository::user_repo::find_by_username(db, "admin")
        .await
        .expect("admin lookup should succeed")
        .expect("admin user should exist")
        .id
}

async fn register_test_passkey<S, B, E>(
    app: &S,
    access_token: &str,
    csrf_token: &str,
    name: &str,
) -> (SoftPasskey, Value)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = E,
        >,
    B: MessageBody,
    E: std::fmt::Debug,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/register/start")
        .insert_header((
            "Cookie",
            access_and_csrf_cookie_header(access_token, csrf_token),
        ))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "name": name }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200);
    let start_body: Value = test::read_body_json(resp).await;
    let flow_id = start_body["data"]["flow_id"]
        .as_str()
        .expect("registration flow id should exist")
        .to_string();
    let mut challenge = serde_json::from_value::<CreationChallengeResponse>(
        start_body["data"]["public_key"].clone(),
    )
    .expect("registration challenge should deserialize");
    let selection = challenge
        .public_key
        .authenticator_selection
        .as_ref()
        .expect("registration should include authenticator selection");
    assert_eq!(
        selection.resident_key,
        Some(ResidentKeyRequirement::Required)
    );
    assert!(selection.require_resident_key);

    let selection = challenge
        .public_key
        .authenticator_selection
        .as_mut()
        .expect("registration should include authenticator selection");
    selection.resident_key = Some(ResidentKeyRequirement::Discouraged);
    selection.require_resident_key = false;

    let mut softpasskey = SoftPasskey::new(true);
    let credential = softpasskey
        .do_registration(Url::parse(TEST_BROWSER_ORIGIN).unwrap(), challenge)
        .expect("soft passkey registration should succeed");
    let credential = serde_json::to_value(credential).expect("credential should serialize");

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/register/finish")
        .insert_header(("Origin", TEST_BROWSER_ORIGIN))
        .insert_header(common::csrf_header(csrf_token))
        .insert_header((
            "Cookie",
            access_and_csrf_cookie_header(access_token, csrf_token),
        ))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "flow_id": flow_id,
            "credential": credential,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200);
    let passkey_body: Value = test::read_body_json(resp).await;

    (softpasskey, passkey_body["data"].clone())
}

async fn passkey_login_start<S, B, E>(
    app: &S,
    identifier: Option<&str>,
) -> (String, RequestChallengeResponse)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = E,
        >,
    B: MessageBody,
    E: std::fmt::Debug,
{
    let payload = match identifier {
        Some(identifier) => serde_json::json!({ "identifier": identifier }),
        None => serde_json::json!({}),
    };
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/login/start")
        .insert_header(("Origin", TEST_BROWSER_ORIGIN))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(payload)
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200);
    let start_body: Value = test::read_body_json(resp).await;
    let flow_id = start_body["data"]["flow_id"]
        .as_str()
        .expect("login flow id should exist")
        .to_string();
    let challenge = serde_json::from_value::<RequestChallengeResponse>(
        start_body["data"]["public_key"].clone(),
    )
    .expect("login challenge should deserialize");
    (flow_id, challenge)
}

async fn conditional_passkey_login_start<S, B, E>(app: &S) -> (String, RequestChallengeResponse)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = E,
        >,
    B: MessageBody,
    E: std::fmt::Debug,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/login/start")
        .insert_header(("Origin", TEST_BROWSER_ORIGIN))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "conditional": true }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200);
    let start_body: Value = test::read_body_json(resp).await;
    let flow_id = start_body["data"]["flow_id"]
        .as_str()
        .expect("conditional login flow id should exist")
        .to_string();
    let challenge = serde_json::from_value::<RequestChallengeResponse>(
        start_body["data"]["public_key"].clone(),
    )
    .expect("conditional login challenge should deserialize");
    (flow_id, challenge)
}

async fn passkey_login_finish<S, B, E>(
    app: &S,
    flow_id: &str,
    credential: Value,
) -> actix_web::dev::ServiceResponse<B>
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = E,
        >,
    B: MessageBody,
    E: std::fmt::Debug,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/login/finish")
        .insert_header(("User-Agent", "AsterYggdrasil Passkey Test/1.0"))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "flow_id": flow_id,
            "credential": credential,
        }))
        .to_request();
    test::call_service(app, req).await
}

fn allow_test_passkey_credential(
    mut challenge: RequestChallengeResponse,
    stored_passkey: &passkey::Model,
) -> (RequestChallengeResponse, uuid::Uuid) {
    let credential_id = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(stored_passkey.credential_id.as_bytes())
        .expect("stored credential id should decode");
    let user_handle =
        uuid::Uuid::parse_str(&stored_passkey.user_handle).expect("user handle should parse");
    challenge.public_key.allow_credentials = vec![AllowCredentials {
        type_: "public-key".to_string(),
        id: credential_id,
        transports: None,
    }];
    (challenge, user_handle)
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([24, 96, 160, 255]));
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("test png should encode");
    bytes
}

fn multipart_file_body(
    boundary: &str,
    file_name: &str,
    content_type: &str,
    bytes: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

fn empty_multipart_body(boundary: &str) -> Vec<u8> {
    format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"note\"\r\n\r\nno file here\r\n--{boundary}--\r\n"
    )
    .into_bytes()
}

#[actix_web::test]
async fn auth_setup_login_me_and_logout_flow() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/check")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["initialized"], false);

    let access_token = setup_admin!(app);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/check")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(status, 200, "auth check failed with body: {body}");
    assert_eq!(body["data"]["initialized"], true);

    let login_token = login_user!(app, "admin", "password1234");
    assert!(!login_token.is_empty());

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(common::bearer_header(&access_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["username"], "admin");
    assert_eq!(body["data"]["role"], "admin");
}

#[actix_web::test]
async fn auth_setup_saves_public_site_url_to_runtime_config() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/setup")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "admin",
            "email": "admin@example.com",
            "password": "password1234",
            "public_site_url": "https://Skin.EXAMPLE.test/"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let stored = system_config_repo::find_by_key(
        state.writer_db(),
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
    )
    .await
    .unwrap()
    .expect("public_site_url config should be stored");
    assert_eq!(stored.value, r#"["https://skin.example.test"]"#);
    assert_eq!(stored.updated_by, Some(1));
    assert_eq!(
        state
            .runtime_config
            .get(aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY)
            .as_deref(),
        Some(r#"["https://skin.example.test"]"#)
    );
}

#[actix_web::test]
async fn auth_setup_accepts_localhost_http_public_site_url() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/setup")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "admin",
            "email": "admin@example.com",
            "password": "password1234",
            "public_site_url": "http://localhost:3000"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let stored = system_config_repo::find_by_key(
        state.writer_db(),
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
    )
    .await
    .unwrap()
    .expect("public_site_url config should be stored");
    assert_eq!(stored.value, r#"["http://localhost:3000"]"#);
}

#[actix_web::test]
async fn auth_setup_rejects_invalid_public_site_url_before_creating_user() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/setup")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "admin",
            "email": "admin@example.com",
            "password": "password1234",
            "public_site_url": "https://example.com/app"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::BadRequest.as_str());
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or_default()
            .contains("invalid public_site_url origin")
    );
    assert_eq!(user_repo::count_all(state.writer_db()).await.unwrap(), 0);
}

#[actix_web::test]
async fn auth_login_sets_http_only_session_cookies_without_token_body() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "identifier": "admin",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let access = common::extract_cookie(&resp, "aster_access").expect("access cookie missing");
    let refresh = common::extract_cookie(&resp, "aster_refresh").expect("refresh cookie missing");
    let csrf = common::extract_cookie(&resp, "aster_csrf").expect("csrf cookie missing");
    assert!(!access.is_empty());
    assert!(!refresh.is_empty());
    assert!(!csrf.is_empty());
    let access_claims = decode_test_claims(&access);
    let refresh_claims = decode_test_claims(&refresh);
    assert_eq!(access_claims.sub, access_claims.user_id.to_string());
    assert_eq!(access_claims.token_type, TokenType::Access);
    assert!(access_claims.jti.is_none());
    assert_eq!(refresh_claims.sub, refresh_claims.user_id.to_string());
    assert_eq!(refresh_claims.token_type, TokenType::Refresh);
    assert!(refresh_claims.jti.is_some());

    let access_cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == "aster_access")
        .expect("access cookie missing");
    assert_eq!(access_cookie.path(), Some("/"));
    assert_eq!(access_cookie.same_site(), Some(SameSite::Lax));
    assert_eq!(access_cookie.http_only(), Some(true));

    let refresh_cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == "aster_refresh")
        .expect("refresh cookie missing");
    assert_eq!(refresh_cookie.path(), Some("/api/v1/auth"));
    assert_eq!(refresh_cookie.same_site(), Some(SameSite::Lax));
    assert_eq!(refresh_cookie.http_only(), Some(true));

    let csrf_cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == "aster_csrf")
        .expect("csrf cookie missing");
    assert_eq!(csrf_cookie.path(), Some("/"));
    assert_eq!(csrf_cookie.same_site(), Some(SameSite::Lax));
    assert_ne!(csrf_cookie.http_only(), Some(true));

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Success.as_str());
    assert!(body["data"]["expires_in"].is_number());
    assert!(body["data"]["access_token"].is_null());
    assert!(body["data"]["refresh_token"].is_null());
    assert!(body["data"]["user"].is_null());
}

#[actix_web::test]
async fn auth_cookie_session_can_read_me_refresh_and_logout() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "identifier": "admin",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let access = common::extract_cookie(&resp, "aster_access").expect("access cookie missing");
    let refresh = common::extract_cookie(&resp, "aster_refresh").expect("refresh cookie missing");
    let csrf = common::extract_cookie(&resp, "aster_csrf").expect("csrf cookie missing");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["username"], "admin");
    assert_eq!(body["data"]["role"], "admin");

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&csrf))
        .insert_header(("Cookie", common::refresh_cookie_header(&refresh, &csrf)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let rotated_access =
        common::extract_cookie(&resp, "aster_access").expect("rotated access cookie missing");
    let rotated_refresh =
        common::extract_cookie(&resp, "aster_refresh").expect("rotated refresh cookie missing");
    let rotated_csrf =
        common::extract_cookie(&resp, "aster_csrf").expect("rotated csrf cookie missing");
    assert!(!rotated_access.is_empty());
    assert_ne!(rotated_refresh, refresh);
    assert_ne!(rotated_csrf, csrf);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Success.as_str());
    assert!(body["data"]["expires_in"].is_number());
    assert!(body["data"]["access_token"].is_null());
    assert!(body["data"]["refresh_token"].is_null());

    let old_refresh_jti = decode_test_claims(&refresh)
        .jti
        .expect("refresh token should include jti");
    let old_refresh_session =
        auth_session_repo::find_by_refresh_jti(state.reader_db(), &old_refresh_jti)
            .await
            .expect("old refresh session lookup should succeed");
    assert!(old_refresh_session.is_none());
    assert!(
        auth_session_repo::find_by_previous_refresh_jti(state.reader_db(), &old_refresh_jti)
            .await
            .expect("previous refresh session lookup should succeed")
            .is_some()
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&rotated_csrf))
        .insert_header((
            "Cookie",
            common::access_and_refresh_cookie_header(
                &rotated_access,
                &rotated_refresh,
                &rotated_csrf,
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        common::extract_cookie(&resp, "aster_access").as_deref(),
        Some("")
    );
    assert_eq!(
        common::extract_cookie(&resp, "aster_refresh").as_deref(),
        Some("")
    );
    assert_eq!(
        common::extract_cookie(&resp, "aster_csrf").as_deref(),
        Some("")
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["revoked"], true);

    let rotated_refresh_jti = decode_test_claims(&rotated_refresh)
        .jti
        .expect("rotated refresh token should include jti");
    let revoked_refresh_session =
        auth_session_repo::find_by_refresh_jti(state.reader_db(), &rotated_refresh_jti)
            .await
            .expect("revoked refresh session lookup should succeed");
    assert!(
        revoked_refresh_session
            .expect("revoked refresh session should remain queryable")
            .revoked_at
            .is_some()
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&rotated_csrf))
        .insert_header((
            "Cookie",
            common::refresh_cookie_header(&rotated_refresh, &rotated_csrf),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn auth_passkey_login_start_rejects_missing_public_site_url_with_config_error() {
    let state = common::setup_with_memory_cache().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/login/start")
        .insert_header(("Origin", TEST_BROWSER_ORIGIN))
        .set_json(serde_json::json!({ "identifier": "admin" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["code"],
        AsterErrorCode::ConfigPublicSiteUrlRequired.as_str()
    );
    assert_eq!(
        body["msg"],
        "public_site_url must be configured before enabling passkey authentication"
    );
}

#[actix_web::test]
async fn auth_passkey_login_start_rejects_insecure_public_site_url_with_config_error() {
    let state = common::setup_with_memory_cache().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
        r#"["http://example.com"]"#,
    ));
    let app = create_test_app!(state);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/login/start")
        .insert_header(("Origin", TEST_BROWSER_ORIGIN))
        .set_json(serde_json::json!({ "identifier": "admin" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["code"],
        AsterErrorCode::ConfigPublicSiteUrlInvalid.as_str()
    );
    assert_eq!(
        body["msg"],
        "passkey authentication requires HTTPS public_site_url, except localhost"
    );
}

#[actix_web::test]
async fn auth_passkey_register_login_and_replay_protection() {
    let state = common::setup_with_memory_cache().await;
    configure_passkey_public_site_url(&state);
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (access, _refresh, csrf) = login_session!(app, "admin", "password1234");

    let (mut softpasskey, passkey) = register_test_passkey(&app, &access, &csrf, "Laptop").await;
    assert_eq!(passkey["name"], "Laptop");
    assert_eq!(passkey["sign_count"], 0);
    let user_id = admin_user_id(state.writer_db()).await;
    let passkey_id = passkey["id"].as_i64().expect("passkey id should exist");
    let stored_passkey = passkey_repo::find_by_id_for_user(state.writer_db(), passkey_id, user_id)
        .await
        .expect("stored passkey lookup should succeed")
        .expect("stored passkey should exist");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/passkeys")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["limit"], 20);
    assert_eq!(body["data"]["offset"], 0);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"]["items"][0]["name"], "Laptop");

    let (flow_id, challenge) = passkey_login_start(&app, Some("admin")).await;
    assert!(challenge.public_key.allow_credentials.is_empty());
    let (challenge, user_handle) = allow_test_passkey_credential(challenge, &stored_passkey);
    let mut credential = softpasskey
        .do_authentication(Url::parse(TEST_BROWSER_ORIGIN).unwrap(), challenge)
        .expect("soft passkey authentication should succeed");
    credential.response.user_handle = Some(user_handle.as_bytes().to_vec());
    let credential = serde_json::to_value(credential).expect("credential should serialize");
    let replay_credential = credential.clone();
    let active_sessions_before_login = auth_session_repo::list_by_user(state.writer_db(), user_id)
        .await
        .expect("session listing should succeed")
        .into_iter()
        .filter(|session| session.revoked_at.is_none())
        .count();

    let resp = passkey_login_finish(&app, &flow_id, credential).await;
    assert_eq!(resp.status(), 200);
    let login_access = common::extract_cookie(&resp, "aster_access").unwrap();
    let login_refresh = common::extract_cookie(&resp, "aster_refresh").unwrap();
    assert!(!login_access.is_empty());
    assert!(!login_refresh.is_empty());
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Success.as_str());

    let sessions = auth_session_repo::list_by_user(state.writer_db(), user_id)
        .await
        .expect("session listing should succeed");
    let active_sessions = sessions
        .iter()
        .filter(|session| session.revoked_at.is_none())
        .count();
    assert_eq!(active_sessions, active_sessions_before_login + 1);
    assert!(sessions.iter().any(|session| {
        session.user_agent.as_deref() == Some("AsterYggdrasil Passkey Test/1.0")
    }));

    let resp = passkey_login_finish(&app, &flow_id, replay_credential).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn auth_passkey_conditional_login_preserves_mediation() {
    let state = common::setup_with_memory_cache().await;
    configure_passkey_public_site_url(&state);
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (access, _refresh, csrf) = login_session!(app, "admin", "password1234");

    let (mut softpasskey, passkey) = register_test_passkey(&app, &access, &csrf, "Laptop").await;
    let user_id = admin_user_id(state.writer_db()).await;
    let passkey_id = passkey["id"].as_i64().expect("passkey id should exist");
    let stored_passkey = passkey_repo::find_by_id_for_user(state.writer_db(), passkey_id, user_id)
        .await
        .expect("stored passkey lookup should succeed")
        .expect("stored passkey should exist");

    let (flow_id, challenge) = conditional_passkey_login_start(&app).await;
    assert!(challenge.public_key.allow_credentials.is_empty());
    assert!(matches!(challenge.mediation, Some(Mediation::Conditional)));

    let (challenge, user_handle) = allow_test_passkey_credential(challenge, &stored_passkey);
    let mut credential = softpasskey
        .do_authentication(Url::parse(TEST_BROWSER_ORIGIN).unwrap(), challenge)
        .expect("soft passkey authentication should succeed");
    credential.response.user_handle = Some(user_handle.as_bytes().to_vec());
    let credential = serde_json::to_value(credential).expect("credential should serialize");

    let resp = passkey_login_finish(&app, &flow_id, credential).await;
    assert_eq!(resp.status(), 200);
    assert!(common::extract_cookie(&resp, "aster_access").is_some());
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Success.as_str());
}

#[actix_web::test]
async fn auth_passkey_login_policy_disables_start_and_finish_without_deleting_credentials() {
    let state = common::setup_with_memory_cache().await;
    configure_passkey_public_site_url(&state);
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (access, _refresh, csrf) = login_session!(app, "admin", "password1234");

    let (mut softpasskey, passkey) = register_test_passkey(&app, &access, &csrf, "Laptop").await;
    let user_id = admin_user_id(state.writer_db()).await;
    let passkey_id = passkey["id"].as_i64().expect("passkey id should exist");
    let stored_passkey = passkey_repo::find_by_id_for_user(state.writer_db(), passkey_id, user_id)
        .await
        .expect("stored passkey lookup should succeed")
        .expect("registered passkey should remain stored");

    let (flow_id, challenge) = passkey_login_start(&app, Some("admin")).await;
    let (challenge, user_handle) = allow_test_passkey_credential(challenge, &stored_passkey);
    let mut credential = softpasskey
        .do_authentication(Url::parse(TEST_BROWSER_ORIGIN).unwrap(), challenge)
        .expect("soft passkey authentication should succeed");
    credential.response.user_handle = Some(user_handle.as_bytes().to_vec());
    let credential = serde_json::to_value(credential).expect("credential should serialize");

    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_PASSKEY_LOGIN_ENABLED_KEY,
        "false",
    ));

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/passkeys/login/start")
        .insert_header(("Origin", TEST_BROWSER_ORIGIN))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "identifier": "admin" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["code"],
        AsterErrorCode::AuthPasskeyLoginDisabled.as_str()
    );
    assert_eq!(
        body["msg"],
        "passkey login is disabled by administrator policy"
    );

    let resp = passkey_login_finish(&app, &flow_id, credential).await;
    assert_eq!(resp.status(), 403);
    assert!(common::extract_cookie(&resp, "aster_access").is_none());
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["code"],
        AsterErrorCode::AuthPasskeyLoginDisabled.as_str()
    );

    let stored_after_disable =
        passkey_repo::find_by_id_for_user(state.writer_db(), passkey_id, user_id)
            .await
            .expect("stored passkey lookup should succeed");
    assert!(
        stored_after_disable.is_some(),
        "disabling passkey login must not delete registered credentials"
    );
}

#[actix_web::test]
async fn auth_sessions_mark_current_and_revoke_selected_session() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (_other_access, other_refresh, other_csrf) = login_session!(app, "admin", "password1234");
    let (current_access, current_refresh, current_csrf) =
        login_session!(app, "admin", "password1234");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/sessions")
        .insert_header((
            "Cookie",
            common::access_and_refresh_cookie_header(
                &current_access,
                &current_refresh,
                &current_csrf,
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 50);
    assert_eq!(body["data"]["offset"], 0);
    let sessions = body["data"]["items"]
        .as_array()
        .expect("sessions response should be an array");
    assert_eq!(
        sessions
            .iter()
            .filter(|session| session["is_current"].as_bool() == Some(true))
            .count(),
        1
    );

    let other_refresh_jti = decode_test_claims(&other_refresh)
        .jti
        .expect("other refresh token should include jti");
    let other_session =
        auth_session_repo::find_by_refresh_jti(state.reader_db(), &other_refresh_jti)
            .await
            .expect("other session lookup should succeed")
            .expect("other session should exist");
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/auth/sessions/{}", other_session.id))
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&current_csrf))
        .insert_header((
            "Cookie",
            common::access_and_refresh_cookie_header(
                &current_access,
                &current_refresh,
                &current_csrf,
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&other_csrf))
        .insert_header((
            "Cookie",
            common::refresh_cookie_header(&other_refresh, &other_csrf),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn auth_sessions_list_clamps_limit_and_applies_offset() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (_first_access, _first_refresh, _first_csrf) = login_session!(app, "admin", "password1234");
    let (current_access, current_refresh, current_csrf) =
        login_session!(app, "admin", "password1234");
    let _ = login_session!(app, "admin", "password1234");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/sessions?limit=9999&offset=1")
        .insert_header((
            "Cookie",
            common::access_and_refresh_cookie_header(
                &current_access,
                &current_refresh,
                &current_csrf,
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100);
    assert_eq!(body["data"]["offset"], 1);
    assert!(body["data"]["total"].as_u64().unwrap() >= 3);
    assert!(body["data"]["items"].as_array().unwrap().len() >= 2);
}

#[actix_web::test]
async fn auth_sessions_revoke_others_keeps_current_session() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);
    let (_other_access, other_refresh, other_csrf) = login_session!(app, "admin", "password1234");
    let (current_access, current_refresh, current_csrf) =
        login_session!(app, "admin", "password1234");

    let req = test::TestRequest::delete()
        .uri("/api/v1/auth/sessions/others")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&current_csrf))
        .insert_header((
            "Cookie",
            common::access_and_refresh_cookie_header(
                &current_access,
                &current_refresh,
                &current_csrf,
            ),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["removed"], 2);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&other_csrf))
        .insert_header((
            "Cookie",
            common::refresh_cookie_header(&other_refresh, &other_csrf),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&current_csrf))
        .insert_header((
            "Cookie",
            common::refresh_cookie_header(&current_refresh, &current_csrf),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn passkeys_list_clamps_limit_and_applies_offset() {
    let state = common::setup().await;
    configure_passkey_public_site_url(&state);
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (access, _refresh, csrf) = login_session!(app, "admin", "password1234");
    let user_id = admin_user_id(state.writer_db()).await;
    let now = Utc::now();
    for index in 0..3 {
        passkey::ActiveModel {
            user_id: Set(user_id),
            credential_id: Set(format!("credential-{index}")),
            user_handle: Set(uuid::Uuid::new_v4().to_string()),
            credential: Set(StoredPasskeyCredential("{}".to_string())),
            name: Set(format!("Device {index}")),
            transports: Set(None),
            backup_eligible: Set(false),
            backed_up: Set(false),
            sign_count: Set(0),
            created_at: Set(now + Duration::seconds(index)),
            updated_at: Set(now + Duration::seconds(index)),
            last_used_at: Set(None),
            ..Default::default()
        }
        .insert(state.writer_db())
        .await
        .expect("passkey should insert");
    }

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/passkeys?limit=9999&offset=1")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100);
    assert_eq!(body["data"]["offset"], 1);
    assert_eq!(body["data"]["total"], 3);
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["name"], "Device 1");
    assert_eq!(items[1]["name"], "Device 0");
}

#[actix_web::test]
async fn auth_sessions_revoke_current_clears_cookies_and_blocks_refresh() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (access, refresh, csrf) = login_session!(app, "admin", "password1234");
    let refresh_jti = decode_test_claims(&refresh)
        .jti
        .expect("refresh token should include jti");
    let session = auth_session_repo::find_by_refresh_jti(state.reader_db(), &refresh_jti)
        .await
        .expect("current session lookup should succeed")
        .expect("current session should exist");

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/auth/sessions/{}", session.id))
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&csrf))
        .insert_header((
            "Cookie",
            common::access_and_refresh_cookie_header(&access, &refresh, &csrf),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        common::extract_cookie(&resp, "aster_access").as_deref(),
        Some("")
    );
    assert_eq!(
        common::extract_cookie(&resp, "aster_refresh").as_deref(),
        Some("")
    );
    assert_eq!(
        common::extract_cookie(&resp, "aster_csrf").as_deref(),
        Some("")
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(common::csrf_header(&csrf))
        .insert_header(("Cookie", common::refresh_cookie_header(&refresh, &csrf)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn auth_errors_use_stable_public_codes_without_internal_code() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(serde_json::json!({
            "identifier": "admin",
            "password": "wrong-password"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AuthCredentialsFailed.as_str());
    assert_eq!(
        body["error"]["code"],
        AsterErrorCode::AuthCredentialsFailed.as_str()
    );
    assert!(body["internal_code"].is_null());
    assert!(body["error"]["internal_code"].is_null());
}

#[actix_web::test]
async fn auth_register_requires_username_at_least_four_characters() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "username": "abc",
            "email": "abc@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::BadRequest.as_str());
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or_default()
            .contains("username must contain at least 4 characters")
    );
}

#[actix_web::test]
async fn auth_register_requires_password_at_least_eight_characters() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "username": "shortpass",
            "email": "shortpass@example.com",
            "password": "1234567"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::BadRequest.as_str());
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or_default()
            .contains("password must contain at least 8 characters")
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "username": "eightpass",
            "email": "eightpass@example.com",
            "password": "12345678"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn auth_profile_defaults_and_display_name_boundaries() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["profile"]["display_name"], Value::Null);
    assert_eq!(body["data"]["profile"]["avatar"]["source"], "none");
    assert_eq!(body["data"]["profile"]["avatar"]["version"], 0);
    assert_eq!(body["data"]["profile"]["avatar"]["url_512"], Value::Null);
    assert_eq!(body["data"]["profile"]["avatar"]["url_1024"], Value::Null);

    let req = test::TestRequest::patch()
        .uri("/api/v1/auth/profile")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "display_name": "  管理员  " }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["display_name"], "管理员");

    let max_name = "界".repeat(64);
    let req = test::TestRequest::patch()
        .uri("/api/v1/auth/profile")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "display_name": max_name }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["display_name"], "界".repeat(64));

    let too_long_name = "界".repeat(65);
    let req = test::TestRequest::patch()
        .uri("/api/v1/auth/profile")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "display_name": too_long_name }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::BadRequest.as_str());
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or_default()
            .contains("64 characters or fewer")
    );

    let req = test::TestRequest::patch()
        .uri("/api/v1/auth/profile")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "display_name": "   " }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["display_name"], Value::Null);
}

#[actix_web::test]
async fn admin_cannot_change_super_admin_role_or_status() {
    let state = common::setup().await;
    let state_for_lookup = state.clone();
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::patch()
        .uri("/api/v1/admin/users/1")
        .insert_header(("Cookie", common::access_cookie_header(&token)))
        .insert_header(common::csrf_header_for(&token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "role": "user" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Forbidden.as_str());

    let req = test::TestRequest::patch()
        .uri("/api/v1/admin/users/1")
        .insert_header(("Cookie", common::access_cookie_header(&token)))
        .insert_header(common::csrf_header_for(&token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "status": "disabled" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Forbidden.as_str());

    let user = user_repo::find_by_id(state_for_lookup.reader_db(), 1)
        .await
        .expect("super admin lookup should succeed");
    assert_eq!(user.role, UserRole::Admin);
    assert_eq!(user.status, UserStatus::Active);
}

#[actix_web::test]
async fn admin_can_update_super_admin_profile_fields() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::patch()
        .uri("/api/v1/admin/users/1")
        .insert_header(("Cookie", common::access_cookie_header(&token)))
        .insert_header(common::csrf_header_for(&token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "email": "root@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["id"], 1);
    assert_eq!(body["data"]["email"], "root@example.com");
    assert_eq!(body["data"]["role"], "admin");
    assert_eq!(body["data"]["status"], "active");
}

#[actix_web::test]
async fn admin_can_change_regular_user_role_and_status() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);
    let user_id = admin_create_user!(app, token, "managed", "managed@example.com", "password1234");

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/users/{user_id}"))
        .insert_header(("Cookie", common::access_cookie_header(&token)))
        .insert_header(common::csrf_header_for(&token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "role": "admin",
            "status": "disabled"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["id"], user_id);
    assert_eq!(body["data"]["role"], "admin");
    assert_eq!(body["data"]["status"], "disabled");
}

#[actix_web::test]
async fn auth_avatar_source_gravatar_and_invalid_upload_source() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/profile/avatar/source")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "source": "gravatar" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["avatar"]["source"], "gravatar");
    assert_eq!(body["data"]["avatar"]["version"], 0);
    assert_eq!(
        body["data"]["avatar"]["url_512"],
        "https://www.gravatar.com/avatar/e64c7d89f26bd1972efa854d13d7dd61?d=identicon&s=512&r=g"
    );
    assert_eq!(
        body["data"]["avatar"]["url_1024"],
        "https://www.gravatar.com/avatar/e64c7d89f26bd1972efa854d13d7dd61?d=identicon&s=1024&r=g"
    );

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/profile/avatar/source")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "source": "upload" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AvatarSourceInvalid.as_str());

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/profile/avatar/source")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "source": "none" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["avatar"]["source"], "none");
    assert_eq!(body["data"]["avatar"]["url_512"], Value::Null);
}

#[actix_web::test]
async fn auth_avatar_read_rejects_missing_upload_and_invalid_size() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/profile/avatar/512")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AvatarNotFound.as_str());

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/profile/avatar/128")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AvatarSizeInvalid.as_str());
}

#[actix_web::test]
async fn auth_avatar_upload_validates_empty_multipart_and_serves_webp_variants() {
    let state = common::setup().await;
    let avatar_root = std::env::temp_dir().join(format!(
        "asteryggdrasil-avatar-test-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&avatar_root).expect("avatar root should create");
    system_config_repo::upsert_with_actor(
        state.writer_db(),
        AVATAR_DIR_KEY,
        avatar_root.to_str().expect("avatar root should be utf8"),
        None,
    )
    .await
    .expect("avatar_dir should save");
    state
        .runtime_config
        .reload(state.writer_db())
        .await
        .expect("runtime config should reload");

    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let empty_boundary = "empty-avatar-boundary";
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/profile/avatar/upload")
        .insert_header(common::bearer_header(&token))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={empty_boundary}"),
        ))
        .set_payload(empty_multipart_body(empty_boundary))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AvatarFileRequired.as_str());

    let boundary = "avatar-boundary";
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/profile/avatar/upload")
        .insert_header(common::bearer_header(&token))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        ))
        .set_payload(multipart_file_body(
            boundary,
            "avatar.png",
            "image/png",
            &png(320, 640),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["avatar"]["source"], "upload");
    assert_eq!(body["data"]["avatar"]["version"], 1);
    assert_eq!(
        body["data"]["avatar"]["url_512"],
        "/auth/profile/avatar/512?v=1"
    );
    assert_eq!(
        body["data"]["avatar"]["url_1024"],
        "/auth/profile/avatar/1024?v=1"
    );

    let stored_512 = avatar_root.join("user/1/v1/512.webp");
    let stored_1024 = avatar_root.join("user/1/v1/1024.webp");
    assert!(stored_512.exists());
    assert!(stored_1024.exists());

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/profile/avatar/512")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("Content-Type")
            .and_then(|value| value.to_str().ok()),
        Some("image/webp")
    );
    assert!(
        resp.headers()
            .get("Cache-Control")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .contains("immutable")
    );
    let avatar_etag = resp
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("avatar response should include etag")
        .to_owned();
    let bytes = test::read_body(resp).await;
    let decoded = image::load_from_memory(&bytes)
        .expect("served avatar should decode")
        .to_rgba8();
    assert_eq!(decoded.dimensions(), (512, 512));

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/profile/avatar/512")
        .insert_header(common::bearer_header(&token))
        .insert_header((header::IF_NONE_MATCH, avatar_etag))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 304);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/users/1/avatar/1024")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let admin_avatar_etag = resp
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("admin avatar response should include etag")
        .to_owned();
    let bytes = test::read_body(resp).await;
    let decoded = image::load_from_memory(&bytes)
        .expect("admin avatar should decode")
        .to_rgba8();
    assert_eq!(decoded.dimensions(), (1024, 1024));

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/users/1/avatar/1024")
        .insert_header(common::bearer_header(&token))
        .insert_header((header::IF_NONE_MATCH, admin_avatar_etag))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 304);

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/profile/avatar/source")
        .insert_header(common::bearer_header(&token))
        .set_json(serde_json::json!({ "source": "none" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert!(!stored_512.exists());
    assert!(!stored_1024.exists());

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/profile/avatar/512")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn cleanup_expired_auth_sessions_removes_only_expired_sessions() {
    let state = common::setup().await;
    let state_for_insert = state.clone();
    let app = create_test_app!(state);
    let _ = setup_admin!(app);
    let now = Utc::now();

    let expired = auth_session::ActiveModel {
        id: Set("expired-session".to_string()),
        user_id: Set(1),
        current_refresh_jti: Set("expired-refresh-jti".to_string()),
        previous_refresh_jti: Set(None),
        refresh_expires_at: Set(now - Duration::minutes(1)),
        user_agent: Set(None),
        ip_address: Set(None),
        created_at: Set(now - Duration::hours(1)),
        last_seen_at: Set(now - Duration::hours(1)),
        revoked_at: Set(None),
    }
    .insert(state_for_insert.writer_db())
    .await
    .expect("expired auth session should insert");
    let active = auth_session::ActiveModel {
        id: Set("active-session".to_string()),
        user_id: Set(1),
        current_refresh_jti: Set("active-refresh-jti".to_string()),
        previous_refresh_jti: Set(None),
        refresh_expires_at: Set(now + Duration::hours(1)),
        user_agent: Set(None),
        ip_address: Set(None),
        created_at: Set(now),
        last_seen_at: Set(now),
        revoked_at: Set(None),
    }
    .insert(state_for_insert.writer_db())
    .await
    .expect("active auth session should insert");

    let removed =
        aster_yggdrasil::services::auth_service::cleanup_expired_auth_sessions(&state_for_insert)
            .await
            .expect("auth session cleanup should succeed");

    assert_eq!(removed, 1);
    let expired_after = auth_session::Entity::find_by_id(expired.id)
        .one(state_for_insert.reader_db())
        .await
        .expect("expired session query should succeed");
    let active_after = auth_session::Entity::find_by_id(active.id)
        .one(state_for_insert.reader_db())
        .await
        .expect("active session query should succeed");
    assert!(expired_after.is_none());
    assert!(active_after.is_some());
}
