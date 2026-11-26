//! Integration tests for user capability bans.

#[macro_use]
mod common;

use actix_web::{http::StatusCode, test};
use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Value, json};

use aster_yggdrasil::entities::{audit_log, user_ban, user_ban_event};
use aster_yggdrasil::services::ops::audit;

async fn create_operator_user<S, B>(
    app: &S,
    admin_access: &str,
    username: &str,
    email: &str,
    scopes: &[&str],
) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/admin/users")
        .insert_header(common::bearer_header(admin_access))
        .set_json(json!({
            "username": username,
            "email": email,
            "password": "password1234",
            "role": "operator",
            "operator_scopes": scopes
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    login_user!(app, username, "password1234")
}

async fn create_user_ban<S, B>(
    app: &S,
    access: &str,
    user_id: i64,
    scope: &str,
    reason: &str,
    starts_at: Option<String>,
    expires_at: Option<String>,
) -> Value
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{user_id}/bans"))
        .insert_header(common::bearer_header(access))
        .set_json(json!({
            "scopes": [scope],
            "reason": reason,
            "public_reason": "visible policy reason",
            "admin_note": "internal note",
            "starts_at": starts_at,
            "expires_at": expires_at,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    test::read_body_json(resp).await
}

async fn create_user_ban_with_scopes<S, B>(
    app: &S,
    access: &str,
    user_id: i64,
    scopes: &[&str],
    reason: &str,
) -> Value
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{user_id}/bans"))
        .insert_header(common::bearer_header(access))
        .set_json(json!({
            "scopes": scopes,
            "reason": reason,
            "public_reason": "visible policy reason",
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    test::read_body_json(resp).await
}

#[actix_web::test]
async fn admin_user_ban_flow_lists_updates_revokes_events_and_audit() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let admin_access = setup_admin!(app);
    let users_operator_access = create_operator_user(
        &app,
        &admin_access,
        "banop",
        "banop@example.com",
        &["com.astercosm.yggdrasil.permission.admin.users"],
    )
    .await;
    let texture_operator_access = create_operator_user(
        &app,
        &admin_access,
        "bantextureop",
        "bantextureop@example.com",
        &["com.astercosm.yggdrasil.permission.admin.texture_library"],
    )
    .await;
    let target_user_id = admin_create_user!(
        app,
        admin_access,
        "banneduser",
        "banned@example.com",
        "password1234"
    );

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
        .to_request();
    let error = test::try_call_service(&app, req)
        .await
        .expect_err("missing token should be rejected");
    assert_eq!(
        error.as_response_error().status_code(),
        StatusCode::UNAUTHORIZED
    );

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
        .insert_header(common::bearer_header(&texture_operator_access))
        .set_json(json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"],
            "reason": "wrong operator scope"
        }))
        .to_request();
    let error = test::try_call_service(&app, req)
        .await
        .expect_err("operator without users scope should be rejected");
    assert_eq!(
        error.as_response_error().status_code(),
        StatusCode::FORBIDDEN
    );

    let create_body = create_user_ban(
        &app,
        &users_operator_access,
        target_user_id,
        "com.astercosm.yggdrasil.capability.texture.upload",
        "  repeated invalid uploads  ",
        None,
        None,
    )
    .await;
    let ban_id = create_body["data"]["id"].as_i64().unwrap();
    assert_eq!(create_body["data"]["user_id"], target_user_id);
    assert_eq!(
        create_body["data"]["scopes"],
        json!(["com.astercosm.yggdrasil.capability.texture.upload"])
    );
    assert_eq!(create_body["data"]["reason"], "repeated invalid uploads");
    assert_eq!(create_body["data"]["effective"], true);
    assert_eq!(create_body["data"]["effective_status"], "active");
    assert_eq!(
        create_body["data"]["public_reason"],
        "visible policy reason"
    );
    assert_eq!(create_body["data"]["admin_note"], "internal note");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
        .insert_header(common::bearer_header(&users_operator_access))
        .set_json(json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"],
            "reason": "duplicate active scope"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let duplicate_body: Value = test::read_body_json(resp).await;
    assert_eq!(duplicate_body["code"], "user_ban.already_active");

    let future_starts = (Utc::now() + Duration::hours(2)).to_rfc3339();
    let future_expires = (Utc::now() + Duration::hours(3)).to_rfc3339();
    let future_body = create_user_ban(
        &app,
        &users_operator_access,
        target_user_id,
        "com.astercosm.yggdrasil.capability.yggdrasil.join",
        "future moderation window",
        Some(future_starts),
        Some(future_expires),
    )
    .await;
    let future_ban_id = future_body["data"]["id"].as_i64().unwrap();
    assert_eq!(future_body["data"]["status"], "active");
    assert_eq!(future_body["data"]["effective"], false);
    assert_eq!(future_body["data"]["effective_status"], "expired");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/user-bans?user_id={target_user_id}&limit=1"
        ))
        .insert_header(common::bearer_header(&users_operator_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let first_page: Value = test::read_body_json(resp).await;
    assert_eq!(first_page["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(first_page["data"]["total"], 2);
    let next_cursor = first_page["data"]["next_cursor"]
        .as_object()
        .expect("first page should have next cursor");

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/user-bans?user_id={target_user_id}&limit=10&after_created_at={}&after_id={}",
            urlencoding::encode(next_cursor["value"].as_str().unwrap()),
            next_cursor["id"].as_i64().unwrap()
        ))
        .insert_header(common::bearer_header(&users_operator_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let second_page: Value = test::read_body_json(resp).await;
    assert_eq!(second_page["data"]["items"].as_array().unwrap().len(), 1);

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/user-bans?user_id={target_user_id}&effective_only=true"
        ))
        .insert_header(common::bearer_header(&users_operator_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let effective_page: Value = test::read_body_json(resp).await;
    assert_eq!(effective_page["data"]["total"], 1);
    assert_eq!(effective_page["data"]["items"][0]["id"], ban_id);

    let update_expires = (Utc::now() + Duration::days(7)).to_rfc3339();
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/user-bans/{ban_id}"))
        .insert_header(common::bearer_header(&users_operator_access))
        .set_json(json!({
            "scopes": ["com.astercosm.yggdrasil.capability.profile.manage"],
            "reason": "profile abuse",
            "public_reason": null,
            "admin_note": "updated note",
            "expires_at": update_expires
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let update_body: Value = test::read_body_json(resp).await;
    assert_eq!(
        update_body["data"]["scopes"],
        json!(["com.astercosm.yggdrasil.capability.profile.manage"])
    );
    assert_eq!(update_body["data"]["reason"], "profile abuse");
    assert_eq!(update_body["data"]["public_reason"], Value::Null);
    assert_eq!(update_body["data"]["admin_note"], "updated note");
    assert!(update_body["data"]["expires_at"].is_string());

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/user-bans/{future_ban_id}"))
        .insert_header(common::bearer_header(&users_operator_access))
        .set_json(json!({
            "reason": "cannot update future inactive ban"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let inactive_update_body: Value = test::read_body_json(resp).await;
    assert_eq!(inactive_update_body["code"], "user_ban.not_active");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/user-bans/{ban_id}"))
        .insert_header(common::bearer_header(&users_operator_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail_body: Value = test::read_body_json(resp).await;
    assert_eq!(detail_body["data"]["id"], ban_id);

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/user-bans/{ban_id}/revoke"))
        .insert_header(common::bearer_header(&users_operator_access))
        .set_json(json!({ "revoke_note": "appeal accepted" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let revoked_body: Value = test::read_body_json(resp).await;
    assert_eq!(revoked_body["data"]["status"], "revoked");
    assert_eq!(revoked_body["data"]["effective"], false);
    assert_eq!(revoked_body["data"]["revoke_note"], "appeal accepted");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/user-bans/{ban_id}/revoke"))
        .insert_header(common::bearer_header(&users_operator_access))
        .set_json(json!({ "revoke_note": "second revoke" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let revoke_again_body: Value = test::read_body_json(resp).await;
    assert_eq!(revoke_again_body["code"], "user_ban.not_active");

    let replacement_body = create_user_ban(
        &app,
        &users_operator_access,
        target_user_id,
        "com.astercosm.yggdrasil.capability.profile.manage",
        "replacement after revoke",
        None,
        None,
    )
    .await;
    assert_eq!(replacement_body["data"]["effective"], true);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/admin/user-bans/{ban_id}/events"))
        .insert_header(common::bearer_header(&users_operator_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let events_body: Value = test::read_body_json(resp).await;
    let events = events_body["data"].as_array().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["event_type"], "revoked");
    assert_eq!(events[0]["note"], "appeal accepted");
    assert_eq!(events[1]["event_type"], "updated");
    assert_eq!(
        events[1]["previous_scopes"],
        json!(["com.astercosm.yggdrasil.capability.texture.upload"])
    );
    assert_eq!(
        events[1]["next_scopes"],
        json!(["com.astercosm.yggdrasil.capability.profile.manage"])
    );
    assert_eq!(events[2]["event_type"], "created");

    {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        state
            .audit_writer()
            .flush(deadline)
            .await
            .expect("audit writer flush should succeed");
    }
    for (action, entity_id) in [
        (audit::AuditAction::AdminCreateUserBan, ban_id),
        (audit::AuditAction::AdminUpdateUserBan, ban_id),
        (audit::AuditAction::AdminRevokeUserBan, ban_id),
    ] {
        let count = audit_log::Entity::find()
            .filter(audit_log::Column::Action.eq(action))
            .filter(audit_log::Column::EntityId.eq(entity_id))
            .count(state.writer_db())
            .await
            .unwrap();
        assert_eq!(count, 1, "{action:?} audit should be written once");
    }
}

#[actix_web::test]
async fn user_ban_record_supports_multiple_scopes_and_rejects_overlap() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let admin_access = setup_admin!(app);
    let target_user_id = admin_create_user!(
        app,
        admin_access,
        "multiscopeban",
        "multiscopeban@example.com",
        "password1234"
    );

    let create_body = create_user_ban_with_scopes(
        &app,
        &admin_access,
        target_user_id,
        &[
            "com.astercosm.yggdrasil.capability.texture.upload",
            "com.astercosm.yggdrasil.capability.yggdrasil.access",
            "com.astercosm.yggdrasil.capability.texture.upload",
            "com.astercosm.yggdrasil.capability.profile.manage",
        ],
        "multi capability restriction",
    )
    .await;
    let ban_id = create_body["data"]["id"].as_i64().unwrap();
    assert_eq!(
        create_body["data"]["scopes"],
        json!([
            "com.astercosm.yggdrasil.capability.profile.manage",
            "com.astercosm.yggdrasil.capability.texture.upload",
            "com.astercosm.yggdrasil.capability.yggdrasil.access"
        ])
    );

    for scope in [
        "com.astercosm.yggdrasil.capability.texture.upload",
        "com.astercosm.yggdrasil.capability.yggdrasil.access",
        "com.astercosm.yggdrasil.capability.profile.manage",
    ] {
        let req = test::TestRequest::get()
            .uri(&format!(
                "/api/v1/admin/user-bans?user_id={target_user_id}&scope={scope}&effective_only=true"
            ))
            .insert_header(common::bearer_header(&admin_access))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["data"]["total"], 1, "{scope} should match the ban");
        assert_eq!(body["data"]["items"][0]["id"], ban_id);
    }

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/admin/user-bans?user_id={target_user_id}&scope=com.astercosm.yggdrasil.capability.texture_library.interact&effective_only=true"
        ))
        .insert_header(common::bearer_header(&admin_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let unmatched_body: Value = test::read_body_json(resp).await;
    assert_eq!(unmatched_body["data"]["total"], 0);

    for scopes in [
        json!(["com.astercosm.yggdrasil.capability.texture.upload"]),
        json!([
            "com.astercosm.yggdrasil.capability.texture_library.interact",
            "com.astercosm.yggdrasil.capability.profile.manage"
        ]),
    ] {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
            .insert_header(common::bearer_header(&admin_access))
            .set_json(json!({
                "scopes": scopes,
                "reason": "overlapping restriction",
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], "user_ban.already_active");
    }

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture_library.interact"],
            "reason": "non-overlapping restriction",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let non_overlapping_body: Value = test::read_body_json(resp).await;
    assert_eq!(
        non_overlapping_body["data"]["scopes"],
        json!(["com.astercosm.yggdrasil.capability.texture_library.interact"])
    );

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/user-bans/{ban_id}"))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(json!({
            "scopes": [],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let empty_update_body: Value = test::read_body_json(resp).await;
    assert_eq!(empty_update_body["code"], "user_ban.reason_invalid");

    let events = user_ban_event::Entity::find()
        .filter(user_ban_event::Column::BanId.eq(ban_id))
        .all(state.writer_db())
        .await
        .unwrap();
    assert_eq!(
        events.len(),
        1,
        "failed overlap writes must not create events"
    );
    assert_eq!(
        events[0].next_scopes.as_ref().unwrap().as_vec().unwrap(),
        vec![
            aster_yggdrasil::types::permission::capability::profile::MANAGE,
            aster_yggdrasil::types::permission::capability::texture::UPLOAD,
            aster_yggdrasil::types::permission::capability::yggdrasil::ACCESS,
        ]
    );
}

#[actix_web::test]
async fn user_ban_admin_api_rejects_invalid_edges_without_writing_events() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let admin_access = setup_admin!(app);
    let target_user_id = admin_create_user!(
        app,
        admin_access,
        "banedge",
        "banedge@example.com",
        "password1234"
    );

    let req = test::TestRequest::post()
        .uri("/api/v1/admin/users/999999/bans")
        .insert_header(common::bearer_header(&admin_access))
        .set_json(json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"],
            "reason": "missing user"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    for payload in [
        json!({ "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"], "reason": "" }),
        json!({ "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"], "reason": "   " }),
        json!({ "scopes": [], "reason": "valid" }),
        json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"],
            "reason": "x".repeat(129)
        }),
        json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"],
            "reason": "valid",
            "public_reason": "x".repeat(1001)
        }),
        json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"],
            "reason": "valid",
            "admin_note": "x".repeat(1001)
        }),
    ] {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
            .insert_header(common::bearer_header(&admin_access))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: Value = test::read_body_json(resp).await;
        assert!(
            body["code"] == "bad_request"
                || body["code"] == "validation.failed"
                || body["code"] == "user_ban.reason_invalid",
            "unexpected error body: {body}"
        );
    }

    let starts_at = (Utc::now() + Duration::hours(1)).to_rfc3339();
    let expires_at = (Utc::now() + Duration::minutes(30)).to_rfc3339();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(json!({
            "scopes": ["com.astercosm.yggdrasil.capability.texture.upload"],
            "reason": "bad range",
            "starts_at": starts_at,
            "expires_at": expires_at
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "user_ban.duration_invalid");

    let expired_start = (Utc::now() - Duration::hours(2)).to_rfc3339();
    let expired_end = (Utc::now() - Duration::hours(1)).to_rfc3339();
    let expired_body = create_user_ban(
        &app,
        &admin_access,
        target_user_id,
        "com.astercosm.yggdrasil.capability.texture.upload",
        "already expired",
        Some(expired_start),
        Some(expired_end),
    )
    .await;
    let expired_ban_id = expired_body["data"]["id"].as_i64().unwrap();
    assert_eq!(expired_body["data"]["status"], "active");
    assert_eq!(expired_body["data"]["effective"], false);
    assert_eq!(expired_body["data"]["effective_status"], "expired");

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/admin/user-bans/{expired_ban_id}"))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(json!({ "reason": "late update" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "user_ban.not_active");

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/admin/user-bans/{expired_ban_id}/revoke"))
        .insert_header(common::bearer_header(&admin_access))
        .set_json(json!({ "revoke_note": "late revoke" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "user_ban.not_active");

    for uri in [
        "/api/v1/admin/user-bans/0",
        "/api/v1/admin/user-bans/999999",
        "/api/v1/admin/user-bans/999999/events",
    ] {
        let req = test::TestRequest::get()
            .uri(uri)
            .insert_header(common::bearer_header(&admin_access))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["code"], "user_ban.not_found");
    }

    let event_count = user_ban_event::Entity::find()
        .count(state.writer_db())
        .await
        .unwrap();
    assert_eq!(
        event_count, 1,
        "only successful expired create should write an event"
    );
    let stored_expired = user_ban::Entity::find_by_id(expired_ban_id)
        .one(state.writer_db())
        .await
        .unwrap()
        .expect("expired ban should remain stored");
    assert_eq!(stored_expired.status.as_str(), "active");
}

#[actix_web::test]
async fn account_user_bans_list_only_current_user_and_hide_admin_fields() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let admin_access = setup_admin!(app);
    let target_user_id = admin_create_user!(
        app,
        admin_access,
        "selfbanuser",
        "selfban@example.com",
        "password1234"
    );
    let other_user_id = admin_create_user!(
        app,
        admin_access,
        "otherbanuser",
        "otherban@example.com",
        "password1234"
    );
    let target_access = login_user!(app, "selfbanuser", "password1234");
    let other_access = login_user!(app, "otherbanuser", "password1234");

    let req = test::TestRequest::get()
        .uri("/api/v1/account/bans")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let active_body = create_user_ban(
        &app,
        &admin_access,
        target_user_id,
        "com.astercosm.yggdrasil.capability.texture.upload",
        "private moderation reason",
        None,
        None,
    )
    .await;
    let active_ban_id = active_body["data"]["id"].as_i64().unwrap();

    let expired_start = (Utc::now() - Duration::hours(3)).to_rfc3339();
    let expired_end = (Utc::now() - Duration::hours(2)).to_rfc3339();
    let expired_body = create_user_ban(
        &app,
        &admin_access,
        target_user_id,
        "com.astercosm.yggdrasil.capability.profile.manage",
        "expired restriction",
        Some(expired_start),
        Some(expired_end),
    )
    .await;
    let expired_ban_id = expired_body["data"]["id"].as_i64().unwrap();

    let _other_body = create_user_ban(
        &app,
        &admin_access,
        other_user_id,
        "com.astercosm.yggdrasil.capability.yggdrasil.join",
        "other user restriction",
        None,
        None,
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/v1/account/bans?limit=1")
        .insert_header(common::bearer_header(&target_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let first_page: Value = test::read_body_json(resp).await;
    assert_eq!(first_page["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(first_page["data"]["total"], 2);
    assert!(first_page["data"]["next_cursor"].is_object());

    let first_item = first_page["data"]["items"][0].as_object().unwrap();
    assert_eq!(first_item.get("user_id"), None);
    assert_eq!(first_item.get("admin_note"), None);
    assert_eq!(first_item.get("created_by_user_id"), None);
    assert_eq!(first_item.get("revoked_by_user_id"), None);
    assert_eq!(first_item.get("revoke_note"), None);
    assert!(first_item.get("reason").and_then(Value::as_str).is_some());
    assert_eq!(
        first_item.get("public_reason").unwrap(),
        "visible policy reason"
    );

    let next_cursor = first_page["data"]["next_cursor"].as_object().unwrap();
    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/v1/account/bans?limit=10&after_created_at={}&after_id={}",
            urlencoding::encode(next_cursor["value"].as_str().unwrap()),
            next_cursor["id"].as_i64().unwrap()
        ))
        .insert_header(common::bearer_header(&target_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let second_page: Value = test::read_body_json(resp).await;
    assert_eq!(second_page["data"]["items"].as_array().unwrap().len(), 1);

    let req = test::TestRequest::get()
        .uri("/api/v1/account/bans?effective_only=true")
        .insert_header(common::bearer_header(&target_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let effective_page: Value = test::read_body_json(resp).await;
    assert_eq!(effective_page["data"]["total"], 1);
    assert_eq!(effective_page["data"]["items"][0]["id"], active_ban_id);
    assert_eq!(effective_page["data"]["items"][0]["effective"], true);
    assert_eq!(
        effective_page["data"]["items"][0]["effective_status"],
        "active"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/account/bans?status=active&scope=com.astercosm.yggdrasil.capability.profile.manage")
        .insert_header(common::bearer_header(&target_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let scoped_page: Value = test::read_body_json(resp).await;
    assert_eq!(scoped_page["data"]["total"], 1);
    assert_eq!(scoped_page["data"]["items"][0]["id"], expired_ban_id);
    assert_eq!(
        scoped_page["data"]["items"][0]["effective_status"],
        "expired"
    );

    let req = test::TestRequest::get()
        .uri("/api/v1/account/bans")
        .insert_header(common::bearer_header(&other_access))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let other_page: Value = test::read_body_json(resp).await;
    assert_eq!(other_page["data"]["total"], 1);
    assert_ne!(other_page["data"]["items"][0]["id"], active_ban_id);
    assert_ne!(other_page["data"]["items"][0]["id"], expired_ban_id);
}

#[actix_web::test]
async fn user_ban_admin_api_rejects_unregistered_or_non_capability_scopes() {
    let state = common::setup().await;
    let app = create_test_app!(state.clone());
    let admin_access = setup_admin!(app);
    let target_user_id = admin_create_user!(
        app,
        admin_access,
        "banregscope",
        "banregscope@example.com",
        "password1234"
    );

    for payload in [
        // Valid syntax, but no plugin registry exists to register it.
        json!({
            "scopes": ["com.example.plugin.capability.rendering.export"],
            "reason": "unregistered capability"
        }),
        // Admin-panel areas are grant-scoped and can never be banned.
        json!({
            "scopes": ["com.astercosm.yggdrasil.permission.admin.users"],
            "reason": "admin area as ban scope"
        }),
    ] {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/admin/users/{target_user_id}/bans"))
            .insert_header(common::bearer_header(&admin_access))
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(
            body["code"], "permission.not_registered",
            "unexpected error body: {body}"
        );
    }
}
