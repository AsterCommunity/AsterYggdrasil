//! Integration tests for local auth routes.

#[macro_use]
mod common;

use actix_web::{body::MessageBody, cookie::SameSite, http::header, test};
use aster_yggdrasil::api::error_code::AsterErrorCode;
use aster_yggdrasil::config::branding::DEFAULT_BRANDING_TITLE;
use aster_yggdrasil::db::repository::{
    auth_session_repo, contact_verification_token_repo, passkey_repo, system_config_repo, user_repo,
};
use aster_yggdrasil::entities::{audit_log, auth_session, contact_verification_token, passkey};
use aster_yggdrasil::services::auth_service::AccessClaims;
use aster_yggdrasil::types::{
    StoredPasskeyCredential, TokenType, UserRole, UserStatus, VerificationChannel,
    VerificationPurpose,
};
use base64::Engine as _;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, Validation};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
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
        let csrf = common::extract_cookie(&resp, "aster_yggdrasil_csrf").expect("csrf cookie missing");
        (access, refresh, csrf)
    }};
}

macro_rules! login_session_with_body {
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
        let status = resp.status();
        let access = common::extract_cookie(&resp, "aster_access");
        let refresh = common::extract_cookie(&resp, "aster_refresh");
        let csrf = common::extract_cookie(&resp, "aster_yggdrasil_csrf");
        let body: Value = test::read_body_json(resp).await;
        (status, body, access, refresh, csrf)
    }};
}

macro_rules! admin_create_user_with_body {
    ($app:expr, $admin_token:expr, $payload:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v1/admin/users")
            .insert_header(("Cookie", common::access_cookie_header(&$admin_token)))
            .insert_header(common::csrf_header_for(&$admin_token))
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json($payload)
            .to_request();
        let resp = test::call_service(&$app, req).await;
        let status = resp.status();
        let body: Value = test::read_body_json(resp).await;
        (status, body)
    }};
}

async fn admin_create_user_with_payload<S, B, E>(
    app: &S,
    admin_token: &str,
    payload: serde_json::Value,
) -> Value
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
        .uri("/api/v1/admin/users")
        .insert_header(("Cookie", common::access_cookie_header(admin_token)))
        .insert_header(common::csrf_header_for(admin_token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(payload)
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 201, "admin create user should return 201");
    test::read_body_json(resp).await
}

async fn current_user_id<S, B, E>(app: &S, access_token: &str) -> i64
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = E,
        >,
    B: MessageBody,
    E: std::fmt::Debug,
{
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(common::bearer_header(access_token))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    body["data"]["id"]
        .as_i64()
        .expect("current user id should exist")
}

fn access_and_csrf_cookie_header(access_token: &str, csrf_token: &str) -> String {
    format!("aster_access={access_token}; aster_yggdrasil_csrf={csrf_token}")
}

fn extract_password_reset_token(
    message: &aster_yggdrasil::services::mail_service::MailMessage,
) -> String {
    common::extract_token_from_mail_message(message, "/reset-password?token=")
        .expect("password reset link missing from mail body")
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

fn assert_generated_password_policy(password: &str) {
    assert!(
        password.len() >= 24,
        "generated password should be at least 24 bytes"
    );
    assert!(
        password.chars().any(|c| c.is_ascii_uppercase()),
        "generated password should include uppercase ASCII"
    );
    assert!(
        password.chars().any(|c| c.is_ascii_lowercase()),
        "generated password should include lowercase ASCII"
    );
    assert!(
        password.chars().any(|c| c.is_ascii_digit()),
        "generated password should include an ASCII digit"
    );
    assert!(
        password
            .chars()
            .any(|c| "!@#$%^&*-_+=".chars().any(|symbol| symbol == c)),
        "generated password should include an allowed symbol"
    );
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
async fn register_with_activation_requires_email_confirmation_before_login() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
        "true",
    ));
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
        r#"["http://localhost:8080"]"#,
    ));
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "pending-user",
            "email": "pending@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert!(
        common::extract_cookie(&resp, "aster_access").is_none(),
        "pending activation registration must not issue access cookie"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["requires_activation"], true);

    let user = user_repo::find_by_email(state.writer_db(), "pending@example.com")
        .await
        .unwrap()
        .unwrap();
    assert!(user.email_verified_at.is_none());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "identifier": "pending@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AuthPendingActivation.as_str());

    common::flush_mail_outbox(&state).await;
    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    let message = sender
        .last_message()
        .expect("activation email should be delivered");
    let token = common::extract_token_from_mail_message(&message, "token=")
        .expect("activation mail should contain token");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/contact-verification/confirm?token={}",
            urlencoding::encode(&token)
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 302);

    let verified = user_repo::find_by_email(state.writer_db(), "pending@example.com")
        .await
        .unwrap()
        .unwrap();
    assert!(verified.email_verified_at.is_some());

    let access = login_user!(app, "pending@example.com", "password1234");
    assert!(!access.is_empty());
}

#[actix_web::test]
async fn register_skips_activation_when_activation_is_disabled() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
        "false",
    ));
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "direct-user",
            "email": "Direct@Example.COM",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert!(
        common::extract_cookie(&resp, "aster_access").is_some(),
        "non-activation registration should issue session cookies"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["requires_activation"], false);
    let user = user_repo::find_by_email(state.writer_db(), "direct@example.com")
        .await
        .unwrap()
        .expect("registered user should exist");
    assert_eq!(user.email, "direct@example.com");
    assert!(user.email_verified_at.is_some());

    common::flush_mail_outbox(&state).await;
    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    assert!(sender.messages().is_empty());
}

#[actix_web::test]
async fn auth_captcha_policy_and_challenge_are_public() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_CAPTCHA_ENABLED_KEY,
        "true",
    ));
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/captcha/policy")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["enabled"], true);
    assert_eq!(body["data"]["login_required"], true);
    assert_eq!(body["data"]["register_required"], true);
    assert_eq!(body["data"]["invitation_accept_required"], true);
    assert_eq!(body["data"]["register_activation_resend_required"], true);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/captcha")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["data"]["challenge_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert_eq!(body["data"]["mime"], "image/jpeg");
    assert_eq!(body["data"]["expires_in"], 120);
    assert!(
        body["data"]["image_base64"]
            .as_str()
            .is_some_and(|value| value.starts_with("data:image/jpeg;base64,"))
    );
}

#[actix_web::test]
async fn auth_captcha_blocks_password_flows_when_required() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_CAPTCHA_ENABLED_KEY,
        "true",
    ));
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
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AuthCaptchaRequired.as_str());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/captcha")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let challenge_id = body["data"]["challenge_id"]
        .as_str()
        .expect("captcha challenge id should be present");

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "identifier": "admin",
            "password": "password1234",
            "captcha_challenge_id": challenge_id,
            "captcha_answer": "definitely-wrong"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AuthCaptchaInvalid.as_str());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "captcha-user",
            "email": "captcha@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AuthCaptchaRequired.as_str());
}

#[actix_web::test]
async fn auth_captcha_disabled_keeps_password_flows_compatible() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
        "false",
    ));
    let app = create_test_app!(state);
    let _ = setup_admin!(app);

    let login_token = login_user!(app, "admin", "password1234");
    assert!(!login_token.is_empty());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "no-captcha-user",
            "email": "no-captcha@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["requires_activation"], false);
}

#[actix_web::test]
async fn auth_captcha_respects_password_flow_switches() {
    let state = common::setup().await;
    for (key, value) in [
        (
            aster_yggdrasil::config::auth_runtime::AUTH_CAPTCHA_ENABLED_KEY,
            "true",
        ),
        (
            aster_yggdrasil::config::auth_runtime::AUTH_CAPTCHA_LOGIN_REQUIRED_KEY,
            "false",
        ),
        (
            aster_yggdrasil::config::auth_runtime::AUTH_CAPTCHA_REGISTER_REQUIRED_KEY,
            "false",
        ),
        (
            aster_yggdrasil::config::auth_runtime::AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY,
            "false",
        ),
        (
            aster_yggdrasil::config::auth_runtime::AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
            "true",
        ),
    ] {
        state
            .runtime_config
            .apply(common::system_config_model(key, value));
    }
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);

    let login_token = login_user!(app, "admin", "password1234");
    assert!(!login_token.is_empty());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "switch-user",
            "email": "switch@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["requires_activation"], true);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register/resend")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "identifier": "switch@example.com"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn register_activation_resend_is_generic_and_respects_cooldown() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
        "true",
    ));
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY,
        "3600",
    ));
    let app = create_test_app!(state.clone());
    let admin_token = setup_admin!(app);
    admin_create_user!(
        app,
        admin_token,
        "active-user",
        "active@example.com",
        "password1234"
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "cooldown-user",
            "email": "cooldown@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;

    let resend = |identifier: &str| {
        test::TestRequest::post()
            .uri("/api/v1/auth/register/resend")
            .peer_addr("127.0.0.1:12345".parse().unwrap())
            .set_json(serde_json::json!({ "identifier": identifier }))
            .to_request()
    };

    for identifier in [
        "missing@example.com",
        "active@example.com",
        "cooldown@example.com",
    ] {
        let resp = test::call_service(&app, resend(identifier)).await;
        assert_eq!(resp.status(), 200);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(
            body["data"]["message"],
            "If the account can be reactivated, an activation email will be sent"
        );
    }

    common::flush_mail_outbox(&state).await;
    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    assert_eq!(
        sender.messages().len(),
        1,
        "cooldown resend should not queue another activation email"
    );
}

#[actix_web::test]
async fn register_activation_resend_hides_email_policy_rejection() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
        "true",
    ));
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_CONTACT_VERIFICATION_RESEND_COOLDOWN_SECS_KEY,
        "1",
    ));
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "policy-pending",
            "email": "policy-pending@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;

    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::local_email_policy::AUTH_LOCAL_EMAIL_ALLOWLIST_KEY,
        r#"["allowed.test"]"#,
    ));
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::local_email_policy::AUTH_LOCAL_EMAIL_BLOCKLIST_KEY,
        r#"["policy-pending@example.com"]"#,
    ));

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register/resend")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "identifier": "policy-pending@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["data"]["message"],
        "If the account can be reactivated, an activation email will be sent"
    );

    common::flush_mail_outbox(&state).await;
    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    assert_eq!(
        sender.messages().len(),
        1,
        "policy rejection should not leak by sending another email"
    );
}

#[actix_web::test]
async fn contact_verification_confirm_rejects_invalid_expired_and_replayed_tokens() {
    use sea_orm::{ColumnTrait, QueryFilter};

    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::auth_runtime::AUTH_REGISTER_ACTIVATION_ENABLED_KEY,
        "true",
    ));
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
        r#"["http://localhost:8080"]"#,
    ));
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/contact-verification/confirm?token=not-a-token")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 302);
    assert_eq!(
        resp.headers()
            .get("Location")
            .and_then(|value| value.to_str().ok()),
        Some("/login?contact_verification=invalid")
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "expiredactive",
            "email": "expired-activation@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;
    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    let token = common::extract_token_from_mail_message(
        &sender
            .last_message()
            .expect("activation email should be delivered"),
        "token=",
    )
    .expect("activation mail should contain token");

    let token_hash = aster_forge_crypto::sha256_hex(token.as_bytes());
    let record = contact_verification_token::Entity::find()
        .filter(contact_verification_token::Column::TokenHash.eq(token_hash))
        .one(state.writer_db())
        .await
        .unwrap()
        .expect("activation token record should exist");
    let mut active: contact_verification_token::ActiveModel = record.into();
    active.expires_at = Set(Utc::now() - Duration::seconds(1));
    active.update(state.writer_db()).await.unwrap();

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/contact-verification/confirm?token={}",
            urlencoding::encode(&token)
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 302);
    assert_eq!(
        resp.headers()
            .get("Location")
            .and_then(|value| value.to_str().ok()),
        Some("/login?contact_verification=expired")
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "replayactive",
            "email": "replay-activation@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;
    let token = common::extract_token_from_mail_message(
        &sender
            .last_message()
            .expect("activation email should be delivered"),
        "token=",
    )
    .expect("activation mail should contain token");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/contact-verification/confirm?token={}",
            urlencoding::encode(&token)
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 302);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/contact-verification/confirm?token={}",
            urlencoding::encode(&token)
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 302);
    assert_eq!(
        resp.headers()
            .get("Location")
            .and_then(|value| value.to_str().ok()),
        Some("/login?contact_verification=invalid")
    );
}

#[actix_web::test]
async fn contact_verification_tokens_allow_only_one_unconsumed_token_per_purpose() {
    let state = common::setup().await;
    let db = state.writer_db().clone();
    let app = create_test_app!(state);
    let _ = setup_admin!(app);
    let user = user_repo::find_by_email(&db, "admin@example.com")
        .await
        .unwrap()
        .expect("admin user should exist");
    let now = Utc::now();

    contact_verification_token_repo::create(
        &db,
        contact_verification_token::ActiveModel {
            user_id: Set(user.id),
            channel: Set(VerificationChannel::Email),
            purpose: Set(VerificationPurpose::RegisterActivation),
            target: Set(user.email.clone()),
            token_hash: Set("token-hash-1".to_string()),
            expires_at: Set(now + Duration::minutes(10)),
            consumed_at: Set(None),
            created_at: Set(now),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let duplicate = contact_verification_token_repo::create(
        &db,
        contact_verification_token::ActiveModel {
            user_id: Set(user.id),
            channel: Set(VerificationChannel::Email),
            purpose: Set(VerificationPurpose::RegisterActivation),
            target: Set(user.email.clone()),
            token_hash: Set("token-hash-2".to_string()),
            expires_at: Set(now + Duration::minutes(20)),
            consumed_at: Set(None),
            created_at: Set(now + Duration::seconds(1)),
            ..Default::default()
        },
    )
    .await;
    assert!(duplicate.is_err());

    let first = contact_verification_token_repo::find_latest_active_for_user(
        &db,
        user.id,
        VerificationChannel::Email,
        VerificationPurpose::RegisterActivation,
    )
    .await
    .unwrap()
    .expect("first token should still be active");
    assert!(
        contact_verification_token_repo::mark_consumed_if_unused(&db, first.id)
            .await
            .unwrap()
    );

    contact_verification_token_repo::create(
        &db,
        contact_verification_token::ActiveModel {
            user_id: Set(user.id),
            channel: Set(VerificationChannel::Email),
            purpose: Set(VerificationPurpose::RegisterActivation),
            target: Set(user.email),
            token_hash: Set("token-hash-3".to_string()),
            expires_at: Set(now + Duration::minutes(30)),
            consumed_at: Set(None),
            created_at: Set(now + Duration::seconds(2)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
}

#[actix_web::test]
async fn admin_invitation_can_be_verified_and_accepted_once() {
    let state = common::setup().await;
    state.runtime_config.apply(common::system_config_model(
        aster_yggdrasil::config::site_url::PUBLIC_SITE_URL_KEY,
        r#"["http://localhost:8080"]"#,
    ));
    let app = create_test_app!(state.clone());
    let admin_token = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/users/invitations")
        .insert_header(common::bearer_header(&admin_token))
        .set_json(serde_json::json!({ "email": "invitee@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["email"], "invitee@example.com");
    assert_eq!(body["data"]["status"], "pending");
    assert_eq!(body["data"]["mail_queued"], true);

    common::flush_mail_outbox(&state).await;
    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    let message = sender
        .last_message()
        .expect("invitation email should be delivered");
    let token = common::extract_token_from_mail_message(&message, "/invite/")
        .expect("invitation mail should contain token");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/auth/invitations/{token}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["email"], "invitee@example.com");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/auth/invitations/{token}/accept"))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "invited-user",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    assert!(
        common::extract_cookie(&resp, "aster_access").is_none(),
        "accepting an invitation should not issue auth cookies"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["username"], "invited-user");
    assert_eq!(body["data"]["email"], "invitee@example.com");

    let user = user_repo::find_by_email(state.writer_db(), "invitee@example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.username, "invited-user");
    assert!(user.email_verified_at.is_some());

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/auth/invitations/{token}/accept"))
        .set_json(serde_json::json!({
            "username": "second-user",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["code"],
        AsterErrorCode::AuthInvitationAccepted.as_str()
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
    let csrf = common::extract_cookie(&resp, "aster_yggdrasil_csrf").expect("csrf cookie missing");
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
        .find(|cookie| cookie.name() == "aster_yggdrasil_csrf")
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
    configure_passkey_public_site_url(&state);
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
    let csrf = common::extract_cookie(&resp, "aster_yggdrasil_csrf").expect("csrf cookie missing");

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
        common::extract_cookie(&resp, "aster_yggdrasil_csrf").expect("rotated csrf cookie missing");
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
        common::extract_cookie(&resp, "aster_yggdrasil_csrf").as_deref(),
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
    assert!(body["data"].get("offset").is_none());
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
    configure_passkey_public_site_url(&state);
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
    assert!(body["data"].get("offset").is_none());
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
async fn auth_sessions_list_clamps_limit_and_uses_cursor() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (_first_access, _first_refresh, _first_csrf) = login_session!(app, "admin", "password1234");
    let (current_access, current_refresh, current_csrf) =
        login_session!(app, "admin", "password1234");
    let _ = login_session!(app, "admin", "password1234");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/sessions?limit=9999")
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
    assert!(body["data"].get("offset").is_none());
    assert!(body["data"]["total"].as_u64().unwrap() >= 3);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/sessions?limit=1")
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
    let first_page_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(first_page_items.len(), 1);
    let next_cursor = &body["data"]["next_cursor"];
    let after_last_seen_at = next_cursor["value"]
        .as_str()
        .expect("next cursor should include last_seen_at value");
    let after_id = next_cursor["id"]
        .as_str()
        .expect("next cursor should include session id");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/sessions?limit=1&after_last_seen_at={}&after_id={}",
            urlencoding::encode(after_last_seen_at),
            urlencoding::encode(after_id),
        ))
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
    let second_page_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(second_page_items.len(), 1);
    assert_ne!(second_page_items[0]["id"], first_page_items[0]["id"]);
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
async fn passkeys_list_clamps_limit_and_uses_cursor() {
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
        .uri("/api/v1/auth/passkeys?limit=9999")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100);
    assert!(body["data"].get("offset").is_none());
    assert_eq!(body["data"]["total"], 3);
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["name"], "Device 2");
    assert_eq!(items[1]["name"], "Device 1");
    assert_eq!(items[2]["name"], "Device 0");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/passkeys?limit=1")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let first_page_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(first_page_items.len(), 1);
    assert_eq!(first_page_items[0]["name"], "Device 2");
    let next_cursor = &body["data"]["next_cursor"];
    let after_created_at = next_cursor["value"]
        .as_str()
        .expect("next cursor should include created_at value");
    let after_id = next_cursor["id"]
        .as_i64()
        .expect("next cursor should include passkey id");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/passkeys?limit=1&after_created_at={}&after_id={after_id}",
            urlencoding::encode(after_created_at),
        ))
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    let second_page_items = body["data"]["items"].as_array().unwrap();
    assert_eq!(second_page_items.len(), 1);
    assert_eq!(second_page_items[0]["name"], "Device 1");
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
        common::extract_cookie(&resp, "aster_yggdrasil_csrf").as_deref(),
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
            .contains("username must be 4-16 characters")
    );
}

#[actix_web::test]
async fn auth_register_enforces_username_policy_boundaries() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);

    for (index, (username, expected_message)) in [
        ("abc", "username must be 4-16 characters"),
        ("a2345678901234567", "username must be 4-16 characters"),
        (
            "bad.name",
            "username may only contain letters, numbers, underscores and hyphens",
        ),
        (
            "bad name",
            "username may only contain letters, numbers, underscores and hyphens",
        ),
        (
            "用户名",
            "username may only contain letters, numbers, underscores and hyphens",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(serde_json::json!({
                "username": username,
                "email": format!("invalid-username-{index}@example.com"),
                "password": "password1234"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400, "{username} should be rejected");
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], AsterErrorCode::BadRequest.as_str());
        assert!(
            body["msg"]
                .as_str()
                .unwrap_or_default()
                .contains(expected_message),
            "unexpected validation message: {}",
            body["msg"]
        );
    }

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "username": "user-name_123456",
            "email": "user-name-123456@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
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
            .contains("password must be 8-128 characters")
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
async fn auth_register_enforces_password_policy_boundaries() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);

    for (username, password) in [
        ("pwshort", "1234567".to_string()),
        ("pwlong", "a".repeat(129)),
    ] {
        let req = test::TestRequest::post()
            .uri("/api/v1/auth/register")
            .set_json(serde_json::json!({
                "username": username,
                "email": format!("{username}@example.com"),
                "password": password
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400, "{username} should be rejected");
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], AsterErrorCode::BadRequest.as_str());
        assert!(
            body["msg"]
                .as_str()
                .unwrap_or_default()
                .contains("password must be 8-128 characters")
        );
    }

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/register")
        .set_json(serde_json::json!({
            "username": "pwboundary",
            "email": "pwboundary@example.com",
            "password": "a".repeat(128)
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn admin_create_user_uses_auth_username_and_password_policy() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let token = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/users")
        .insert_header(("Cookie", common::access_cookie_header(&token)))
        .insert_header(common::csrf_header_for(&token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "admin.created",
            "email": "admin-created@example.com",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or_default()
            .contains("username may only contain letters, numbers, underscores and hyphens")
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/users")
        .insert_header(("Cookie", common::access_cookie_header(&token)))
        .insert_header(common::csrf_header_for(&token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "username": "admin-created",
            "email": "admin-created@example.com",
            "password": "a".repeat(129)
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or_default()
            .contains("password must be 8-128 characters")
    );
}

#[actix_web::test]
async fn forced_password_change_login_restricts_session_until_password_is_changed() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let admin_token = setup_admin!(app);

    let (status, body) = admin_create_user_with_body!(
        app,
        admin_token,
        &serde_json::json!({
            "username": "forced-user",
            "email": "forced-user@example.com",
            "password": "password1234",
            "must_change_password": true
        })
    );
    assert_eq!(status, 201);
    assert_eq!(body["data"]["user"]["must_change_password"], true);
    assert!(body["data"].get("generated_password").is_none());

    let (status, body, access, refresh, csrf) =
        login_session_with_body!(app, "forced-user", "password1234");
    assert_eq!(status, 200);
    assert_eq!(body["data"]["status"], "password_change_required");
    let access = access.expect("forced login should issue access cookie");
    let refresh = refresh.expect("forced login should issue refresh cookie");
    let csrf = csrf.expect("forced login should issue csrf cookie");
    let claims = decode_test_claims(&access);
    assert!(claims.password_change);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["must_change_password"], true);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/sessions")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["code"],
        AsterErrorCode::AuthPasswordChangeRequired.as_str()
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Cookie", common::refresh_cookie_header(&refresh, &csrf)))
        .insert_header(common::csrf_header_for(&refresh))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body["code"],
        AsterErrorCode::AuthPasswordChangeRequired.as_str()
    );

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/password")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .insert_header(common::csrf_header_for(&access))
        .set_json(serde_json::json!({
            "current_password": "wrong-password",
            "new_password": "newpassword1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::AuthCredentialsFailed.as_str());

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/password")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .insert_header(common::csrf_header_for(&access))
        .set_json(serde_json::json!({
            "current_password": "password1234",
            "new_password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["must_change_password"], true);

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/password")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .insert_header(common::csrf_header_for(&access))
        .set_json(serde_json::json!({
            "current_password": "password1234",
            "new_password": "newpassword1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let new_access =
        common::extract_cookie(&resp, "aster_access").expect("new access cookie missing");
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "authenticated");

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Cookie", common::access_cookie_header(&new_access)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["must_change_password"], false);
}

#[actix_web::test]
async fn forced_password_change_token_can_logout() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let admin_token = setup_admin!(app);

    let (status, _) = admin_create_user_with_body!(
        app,
        admin_token,
        &serde_json::json!({
            "username": "forced-logout",
            "email": "forced-logout@example.com",
            "password": "password1234",
            "must_change_password": true
        })
    );
    assert_eq!(status, 201);

    let (status, _, access, refresh, csrf) =
        login_session_with_body!(app, "forced-logout", "password1234");
    assert_eq!(status, 200);
    let access = access.expect("forced login should issue access cookie");
    let refresh = refresh.expect("forced login should issue refresh cookie");
    let csrf = csrf.expect("forced login should issue csrf cookie");

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .insert_header((
            "Cookie",
            common::access_and_refresh_cookie_header(&access, &refresh, &csrf),
        ))
        .insert_header(common::csrf_header_for(&access))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Cookie", common::refresh_cookie_header(&refresh, &csrf)))
        .insert_header(common::csrf_header_for(&refresh))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);
}

#[actix_web::test]
async fn admin_create_user_can_generate_temporary_password_without_leaking_audit_details() {
    let state = common::setup().await;
    let db = state.writer_db().clone();
    let app = create_test_app!(state.clone());
    let admin_token = setup_admin!(app);

    let (status, body) = admin_create_user_with_body!(
        app,
        admin_token,
        &serde_json::json!({
            "username": "generated-user",
            "email": "generated-user@example.com"
        })
    );
    assert_eq!(status, 201);
    let generated_password = body["data"]["generated_password"]
        .as_str()
        .expect("create response should include generated password")
        .to_string();
    assert_generated_password_policy(&generated_password);
    assert_eq!(body["data"]["user"]["must_change_password"], true);

    let user = user_repo::find_by_username(&db, "generated-user")
        .await
        .expect("user lookup should succeed")
        .expect("generated user should exist");
    assert!(user.must_change_password);
    assert!(!user.password_hash.contains(&generated_password));

    let audit = audit_log::Entity::find()
        .all(&db)
        .await
        .expect("audit lookup should succeed")
        .into_iter()
        .find(|entry| {
            entry.action.to_string() == "admin_create_user"
                && entry.entity_name.as_deref() == Some("generated-user")
        })
        .expect("admin create user audit should exist");
    let details = audit.details.unwrap_or_default();
    assert!(details.contains("\"temporary_password_generated\":true"));
    assert!(details.contains("\"must_change_password\":true"));
    assert!(!details.contains(&generated_password));
    assert!(!details.contains("\"generated_password\""));

    let (status, body, _access, _refresh, _csrf) =
        login_session_with_body!(app, "generated-user", &generated_password);
    assert_eq!(status, 200);
    assert_eq!(body["data"]["status"], "password_change_required");
}

#[actix_web::test]
async fn admin_can_toggle_forced_password_change_for_existing_users() {
    let state = common::setup().await;
    let db = state.writer_db().clone();
    let app = create_test_app!(state);
    let admin_token = setup_admin!(app);

    let (status, body) = admin_create_user_with_body!(
        app,
        admin_token,
        &serde_json::json!({
            "username": "toggle-user",
            "email": "toggle-user@example.com",
            "password": "password1234",
            "must_change_password": true
        })
    );
    assert_eq!(status, 201);
    assert!(body["data"].get("generated_password").is_none());
    let user_id = body["data"]["user"]["id"]
        .as_i64()
        .expect("created user should include id");

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/users/{user_id}"))
        .insert_header(("Cookie", common::access_cookie_header(&admin_token)))
        .insert_header(common::csrf_header_for(&admin_token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "must_change_password": false }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["must_change_password"], false);

    let (status, body, _access, _refresh, _csrf) =
        login_session_with_body!(app, "toggle-user", "password1234");
    assert_eq!(status, 200);
    assert_eq!(body["data"]["status"], "authenticated");

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/users/{user_id}"))
        .insert_header(("Cookie", common::access_cookie_header(&admin_token)))
        .insert_header(common::csrf_header_for(&admin_token))
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "must_change_password": true }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["must_change_password"], true);

    let (status, body, _access, _refresh, _csrf) =
        login_session_with_body!(app, "toggle-user", "password1234");
    assert_eq!(status, 200);
    assert_eq!(body["data"]["status"], "password_change_required");

    let update_details: Vec<String> = audit_log::Entity::find()
        .all(&db)
        .await
        .expect("audit lookup should succeed")
        .into_iter()
        .filter(|entry| entry.action.to_string() == "admin_update_user")
        .filter_map(|entry| entry.details)
        .collect();
    assert!(
        update_details
            .iter()
            .any(|details| details.contains("\"must_change_password\":false"))
    );
    assert!(
        update_details
            .iter()
            .any(|details| details.contains("\"must_change_password\":true"))
    );
}

#[actix_web::test]
async fn auth_change_password_enforces_new_password_boundaries() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let _ = setup_admin!(app);
    let (mut access, _refresh, _) = login_session!(app, "admin", "password1234");

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/password")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .insert_header(common::csrf_header_for(&access))
        .set_json(serde_json::json!({
            "current_password": "password1234",
            "new_password": "1234567"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/password")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .insert_header(common::csrf_header_for(&access))
        .set_json(serde_json::json!({
            "current_password": "password1234",
            "new_password": "12345678"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    access = common::extract_cookie(&resp, "aster_access").expect("new access cookie missing");
    let csrf =
        common::extract_cookie(&resp, "aster_yggdrasil_csrf").expect("new csrf cookie missing");

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/password")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .insert_header(common::csrf_header(&csrf))
        .set_json(serde_json::json!({
            "current_password": "12345678",
            "new_password": "a".repeat(129)
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let req = test::TestRequest::put()
        .uri("/api/v1/auth/password")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .insert_header(common::csrf_header(&csrf))
        .set_json(serde_json::json!({
            "current_password": "12345678",
            "new_password": "b".repeat(128)
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
    let state_for_assert = state.clone();

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

    let stored_512 = "avatar/user/1/v1/512.webp";
    let stored_1024 = "avatar/user/1/v1/1024.webp";
    assert!(
        state_for_assert
            .object_storage()
            .exists(stored_512)
            .await
            .unwrap()
    );
    assert!(
        state_for_assert
            .object_storage()
            .exists(stored_1024)
            .await
            .unwrap()
    );

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

    let admin_avatar_url = format!(
        "/api/v1/admin/avatars/users/{}/1024",
        current_user_id(&app, &token).await
    );
    let req = test::TestRequest::get()
        .uri(&admin_avatar_url)
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
        .uri(&admin_avatar_url)
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
    assert!(
        !state_for_assert
            .object_storage()
            .exists(stored_512)
            .await
            .unwrap()
    );
    assert!(
        !state_for_assert
            .object_storage()
            .exists(stored_1024)
            .await
            .unwrap()
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/profile/avatar/512")
        .insert_header(common::bearer_header(&token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn admin_user_avatar_media_requires_users_scope() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let admin_token = setup_admin!(app);
    admin_create_user_with_payload(
        &app,
        &admin_token,
        serde_json::json!({
            "username": "avatar-target",
            "email": "avatar-target@example.com",
            "password": "password123"
        }),
    )
    .await;
    let (target_token, _, _) = login_session!(app, "avatar-target", "password123");

    let boundary = "avatar-scope-boundary";
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/profile/avatar/upload")
        .insert_header(common::bearer_header(&target_token))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        ))
        .set_payload(multipart_file_body(
            boundary,
            "avatar.png",
            "image/png",
            &png(320, 320),
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let target_user_id = current_user_id(&app, &target_token).await;
    let avatar_url = format!("/api/v1/admin/avatars/users/{target_user_id}/512");

    admin_create_user_with_payload(
        &app,
        &admin_token,
        serde_json::json!({
            "username": "users-operator",
            "email": "users-operator@example.com",
            "password": "password123",
            "role": "operator",
            "operator_scopes": ["users"]
        }),
    )
    .await;
    let (users_operator_token, _, _) = login_session!(app, "users-operator", "password123");

    admin_create_user_with_payload(
        &app,
        &admin_token,
        serde_json::json!({
            "username": "texture-operator",
            "email": "texture-operator@example.com",
            "password": "password123",
            "role": "operator",
            "operator_scopes": ["texture_library"]
        }),
    )
    .await;
    let (texture_operator_token, _, _) = login_session!(app, "texture-operator", "password123");

    admin_create_user_with_payload(
        &app,
        &admin_token,
        serde_json::json!({
            "username": "plain-user",
            "email": "plain-user@example.com",
            "password": "password123"
        }),
    )
    .await;
    let (plain_user_token, _, _) = login_session!(app, "plain-user", "password123");

    let req = test::TestRequest::get()
        .uri(&avatar_url)
        .insert_header(common::bearer_header(&users_operator_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::get()
        .uri(&avatar_url)
        .insert_header(common::bearer_header(&texture_operator_token))
        .to_request();
    let error = test::try_call_service(&app, req)
        .await
        .expect_err("operator without users scope should be rejected");
    assert!(error.to_string().contains("admin permission required"));

    let req = test::TestRequest::get()
        .uri(&avatar_url)
        .insert_header(common::bearer_header(&plain_user_token))
        .to_request();
    let error = test::try_call_service(&app, req)
        .await
        .expect_err("plain user should be rejected");
    assert!(error.to_string().contains("admin permission required"));

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/users")
        .insert_header(common::bearer_header(&texture_operator_token))
        .to_request();
    let error = test::try_call_service(&app, req)
        .await
        .expect_err("operator without users scope should not list users");
    assert!(error.to_string().contains("admin permission required"));
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

#[actix_web::test]
async fn password_reset_request_is_generic_for_unknown_email() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/request")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "email": "missing@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "success");

    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    assert!(sender.messages().is_empty());
}

#[actix_web::test]
async fn password_reset_rotates_session_sends_notice_and_records_audit() {
    let state = common::setup().await;
    let db = state.writer_db().clone();
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (access, refresh, csrf) = login_session!(app, "admin", "password1234");

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/request")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "email": "admin@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;

    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    let token = extract_password_reset_token(
        &sender
            .last_message()
            .expect("password reset email should be sent"),
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/confirm")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "token": token,
            "new_password": "newsecret456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;

    let notice = sender
        .last_message()
        .expect("password reset notice should be sent");
    assert_eq!(notice.to.address, "admin@example.com");
    assert_eq!(
        notice.subject,
        format!("Your {DEFAULT_BRANDING_TITLE} Password Was Reset")
    );
    assert!(notice.text_body.contains("Your password was reset"));

    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .insert_header(("Cookie", common::access_cookie_header(&access)))
        .insert_header(common::csrf_header_for(&access))
        .to_request();
    assert_service_status!(app, req, 401);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .insert_header(("Cookie", common::refresh_cookie_header(&refresh, &csrf)))
        .insert_header(common::csrf_header_for(&refresh))
        .to_request();
    assert_service_status!(app, req, 401);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "identifier": "admin",
            "password": "password1234"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401);

    let _ = login_user!(app, "admin", "newsecret456");

    let actions: Vec<String> = audit_log::Entity::find()
        .all(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.action.to_string())
        .collect();
    assert!(actions.contains(&"user_request_password_reset".to_string()));
    assert!(actions.contains(&"user_confirm_password_reset".to_string()));
}

#[actix_web::test]
async fn password_reset_rejects_reused_expired_and_wrong_endpoint_tokens() {
    use sea_orm::{ColumnTrait, IntoActiveModel, QueryFilter};

    let state = common::setup().await;
    let db = state.writer_db().clone();
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/request")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "email": "admin@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;
    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    let token = extract_password_reset_token(
        &sender
            .last_message()
            .expect("password reset email should be sent"),
    );

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/contact-verification/confirm?token={}",
            urlencoding::encode(&token)
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 302);
    assert_eq!(
        resp.headers()
            .get("Location")
            .and_then(|value| value.to_str().ok()),
        Some("/login?contact_verification=invalid")
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/confirm")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "token": token,
            "new_password": "newsecret456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/confirm")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "token": token,
            "new_password": "anothersecret789"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "auth.contact_verification_invalid");

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/request")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({ "email": "admin@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;
    let expired_token = extract_password_reset_token(
        &sender
            .last_message()
            .expect("password reset email should be sent"),
    );
    let token_hash = aster_forge_crypto::sha256_hex(expired_token.as_bytes());
    let record = contact_verification_token::Entity::find()
        .filter(contact_verification_token::Column::TokenHash.eq(token_hash))
        .one(&db)
        .await
        .unwrap()
        .expect("password reset token record should exist");
    let mut active = record.into_active_model();
    active.expires_at = Set(Utc::now() - Duration::seconds(1));
    active.update(&db).await.unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/password/reset/confirm")
        .peer_addr("127.0.0.1:12345".parse().unwrap())
        .set_json(serde_json::json!({
            "token": expired_token,
            "new_password": "newsecret456"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 410);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "auth.contact_verification_expired");
}

#[actix_web::test]
async fn email_change_confirms_resends_and_sends_notice() {
    let state = common::setup().await;
    let db = state.writer_db().clone();
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (access, _refresh, csrf) = login_session!(app, "admin", "password1234");

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/email/change")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .insert_header(common::csrf_header(&csrf))
        .set_json(serde_json::json!({ "new_email": "new-admin@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["pending_email"], "new-admin@example.com");
    common::flush_mail_outbox(&state).await;

    let user = user_repo::find_by_email(&db, "admin@example.com")
        .await
        .unwrap()
        .expect("admin user should still use old email");
    assert_eq!(user.pending_email.as_deref(), Some("new-admin@example.com"));

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/email/change/resend")
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .insert_header(common::csrf_header(&csrf))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    common::flush_mail_outbox(&state).await;

    let sender = aster_yggdrasil::services::mail_service::memory_sender_ref(&state.mail_sender)
        .expect("test state should use memory mail sender");
    let token = common::extract_token_from_mail_message(
        &sender
            .last_message()
            .expect("contact change confirmation email should be sent"),
        "token=",
    )
    .expect("contact change email should include token");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/auth/contact-verification/confirm?token={}",
            urlencoding::encode(&token)
        ))
        .insert_header(("Cookie", access_and_csrf_cookie_header(&access, &csrf)))
        .insert_header(common::csrf_header(&csrf))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 302);
    let location = resp
        .headers()
        .get("Location")
        .and_then(|value| value.to_str().ok())
        .expect("contact verification redirect location missing");
    assert_eq!(
        location,
        "/settings/security?contact_verification=email-changed&email=new-admin%40example.com"
    );
    common::flush_mail_outbox(&state).await;

    let updated = user_repo::find_by_email(&db, "new-admin@example.com")
        .await
        .unwrap()
        .expect("admin email should be changed");
    assert_eq!(updated.pending_email, None);
    let notice = sender
        .last_message()
        .expect("contact change notice should be sent");
    assert_eq!(notice.to.address, "admin@example.com");
    assert!(notice.text_body.contains("new-admin@example.com"));
}

#[actix_web::test]
async fn email_change_rejects_conflicts_and_pending_activation() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let _ = setup_admin!(app);
    let (admin_access, _refresh, admin_csrf) = login_session!(app, "admin", "password1234");
    let admin_token = login_user!(app, "admin", "password1234");
    let _user_id = admin_create_user!(
        app,
        admin_token,
        "otheruser",
        "other@example.com",
        "password1234"
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/email/change")
        .insert_header((
            "Cookie",
            access_and_csrf_cookie_header(&admin_access, &admin_csrf),
        ))
        .insert_header(common::csrf_header(&admin_csrf))
        .set_json(serde_json::json!({ "new_email": "other@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "auth.email_exists");

    let mut admin = user_repo::find_by_email(state.writer_db(), "admin@example.com")
        .await
        .unwrap()
        .expect("admin user should exist")
        .into_active_model();
    admin.email_verified_at = Set(None);
    admin.update(state.writer_db()).await.unwrap();

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/email/change")
        .insert_header((
            "Cookie",
            access_and_csrf_cookie_header(&admin_access, &admin_csrf),
        ))
        .insert_header(common::csrf_header(&admin_csrf))
        .set_json(serde_json::json!({ "new_email": "pending-new@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 403);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "auth.pending_activation");
}
