//! Integration tests for health routes.

#[macro_use]
mod common;

use actix_web::test;
use actix_web::web;
use aster_yggdrasil::api::error_code::AsterErrorCode;
use aster_yggdrasil::runtime::role::{RoleHealth, ServeRole};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Mounts only the health scope with an explicit role health handle, mirroring
/// what a role-gated process exposes.
async fn probe_app_with_role(
    state: aster_yggdrasil::runtime::AppState,
    role_health: RoleHealth,
) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let observability = state.config().observability.clone();
    test::init_service(
        actix_web::App::new()
            .app_data(web::Data::new(state))
            .app_data(web::Data::new(role_health))
            .service(aster_yggdrasil::api::routes::health::routes(&observability)),
    )
    .await
}

fn role_health(role: ServeRole, initialized: bool) -> RoleHealth {
    let consumers = if role.consumes_tasks() {
        vec![aster_forge_tasks::TaskRuntimeStatus::default()]
    } else {
        Vec::new()
    };
    RoleHealth {
        role,
        consumers,
        shutdown: Default::default(),
        initialized: Arc::new(AtomicBool::new(initialized)),
    }
}

#[actix_web::test]
async fn uninitialized_role_probe_reports_not_live_and_not_ready() {
    let state = common::setup().await;
    let app = probe_app_with_role(state, role_health(ServeRole::Worker, false)).await;

    let resp = test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;
    assert_eq!(resp.status(), 503);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "initializing_or_worker_stopped");

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/health/ready").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 503);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::RuntimeUnavailable.as_str());
    assert_eq!(body["msg"], "Process role is initializing or draining");
    assert_eq!(body["error"]["retryable"], true);
}

#[actix_web::test]
async fn api_role_probe_is_ready_after_initialization() {
    let state = common::setup().await;
    let app = probe_app_with_role(state, role_health(ServeRole::Api, true)).await;

    let resp = test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;
    assert_eq!(resp.status(), 200);

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/health/ready").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["status"], "ready");
}

#[actix_web::test]
async fn worker_role_with_unstarted_consumer_stays_unready_but_shows_role_gate() {
    let state = common::setup().await;
    let app = probe_app_with_role(state, role_health(ServeRole::Worker, true)).await;

    // The consumer status in this fixture never started, so the role gate
    // alone must reject readiness even though the database is healthy.
    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/health/ready").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 503);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["msg"], "Process role is initializing or draining");

    // Liveness is also gated on a running consumer while not draining.
    let resp = test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;
    assert_eq!(resp.status(), 503);
}

#[actix_web::test]
async fn draining_role_stays_live_but_leaves_readiness() {
    let state = common::setup().await;
    let role_health = {
        let role = role_health(ServeRole::Api, true);
        role.shutdown.cancel();
        role
    };
    let app = probe_app_with_role(state, role_health).await;

    let resp = test::call_service(&app, test::TestRequest::get().uri("/health").to_request()).await;
    assert_eq!(resp.status(), 200, "a draining process must stay live");

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/health/ready").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 503, "a draining process must leave rotation");
}

#[actix_web::test]
async fn health_returns_ok() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(
        body.get("version").is_none(),
        "public health leaked version"
    );
    assert!(
        body.get("build_time").is_none(),
        "public health leaked build time"
    );
    assert!(
        body.get("uptime_seconds").is_none(),
        "public health leaked uptime"
    );
    assert!(
        body.get("data").is_none(),
        "public health should use a minimal probe response"
    );
    assert!(
        body.get("code").is_none(),
        "public health should not use the internal API envelope"
    );
    assert!(
        body.get("msg").is_none(),
        "public health should not use the internal API envelope"
    );
}

#[actix_web::test]
async fn health_head_returns_minimal_probe_status() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::default()
        .method(actix_web::http::Method::HEAD)
        .uri("/health")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn ready_checks_database() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::get().uri("/health/ready").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Success.as_str());
    assert_eq!(body["data"]["status"], "ready");
    assert!(
        body["data"].get("version").is_none(),
        "readiness leaked version"
    );
    assert!(
        body["data"].get("build_time").is_none(),
        "readiness leaked build time"
    );
    assert!(
        body["data"].get("uptime_seconds").is_none(),
        "readiness leaked uptime"
    );
    assert!(
        body.get("version").is_none(),
        "readiness leaked version outside data"
    );
    assert!(
        body.get("build_time").is_none(),
        "readiness leaked build time outside data"
    );
    assert!(
        body.get("uptime_seconds").is_none(),
        "readiness leaked uptime outside data"
    );
}

#[actix_web::test]
async fn ready_head_checks_database() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::default()
        .method(actix_web::http::Method::HEAD)
        .uri("/health/ready")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn ready_redacts_database_error() {
    let state = common::setup().await;
    let db = state.writer_db().clone();
    let app = create_test_app!(state);

    db.close_by_ref().await.unwrap();

    let req = test::TestRequest::get().uri("/health/ready").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 503);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::DatabaseError.as_str());
    assert_eq!(body["msg"], "Database unavailable");
    assert_eq!(body["error"]["retryable"], true);
    assert!(body["internal_code"].is_null());
    assert!(
        !body["msg"]
            .as_str()
            .unwrap_or_default()
            .contains("connection"),
        "readiness error leaked database driver detail"
    );
    assert!(
        body.get("data").is_none() || body["data"].is_null(),
        "failed readiness should not include health data"
    );
}

#[actix_web::test]
async fn admin_system_info_exposes_build_and_runtime_metadata_after_auth() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let (admin_token, _) = register_and_login!(app);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/system-info")
        .insert_header(("Cookie", common::access_cookie_header(&admin_token)))
        .insert_header(common::csrf_header_for(&admin_token))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], AsterErrorCode::Success.as_str());
    assert_eq!(body["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(body["data"]["build_time"].as_str().is_some());
    assert!(body["data"]["uptime_seconds"].as_u64().is_some());
    assert_eq!(body["data"]["status"], Value::Null);
    assert_eq!(body["data"].get("status"), None);
}

#[actix_web::test]
async fn admin_system_info_requires_auth() {
    let state = common::setup().await;
    let app = create_test_app!(state);

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/system-info")
        .to_request();
    assert_service_status!(app, req, 401);
}

#[actix_web::test]
async fn admin_system_info_rejects_non_admin_user() {
    let state = common::setup().await;
    let app = create_test_app!(state);
    let admin_token = setup_admin!(app);
    let _user_id = admin_create_user!(
        app,
        admin_token,
        "health-user",
        "health-user@example.com",
        "password1234"
    );
    let user_token = login_user!(app, "health-user", "password1234");

    let req = test::TestRequest::get()
        .uri("/api/v1/admin/system-info")
        .insert_header(("Cookie", common::access_cookie_header(&user_token)))
        .insert_header(common::csrf_header_for(&user_token))
        .to_request();
    assert_service_status!(app, req, 403);
}
