//! Administrator Minecraft profile routes.

use crate::api::dto::RenameMinecraftProfileReq;
use crate::api::dto::{AdminMinecraftProfileListQuery, validation::validate_request};
use crate::api::error_code::AsterErrorCode;
use crate::api::response::ApiResponse;
use crate::db::repository::minecraft_profile_repo;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::auth_service::AuthUserInfo;
use crate::services::{audit_service, texture_service, yggdrasil_service};
use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use aster_forge_api::{CursorPage, IdCursor, LimitQuery, parse_id_cursor};

#[cfg(all(debug_assertions, feature = "openapi"))]
use crate::api::dto::yggdrasil::YggdrasilProfile;

fn current_admin_user_id(req: &HttpRequest) -> Result<i64> {
    req.extensions()
        .get::<AuthUserInfo>()
        .map(|user| user.id)
        .ok_or_else(|| AsterError::internal_error("missing authenticated user in request context"))
}

fn validate_profile_uuid(uuid: &str) -> Result<()> {
    crate::api::dto::validation::validate_unsigned_uuid(uuid).map_err(|error| {
        AsterError::validation_error_code(
            AsterErrorCode::MinecraftProfileUuidInvalid,
            error.message.unwrap_or_default(),
        )
    })
}

async fn find_profile_by_uuid(
    state: &AppState,
    uuid: &str,
) -> Result<crate::entities::minecraft_profile::Model> {
    minecraft_profile_repo::find_by_uuid(state.reader_db(), uuid)
        .await?
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                AsterErrorCode::MinecraftProfileNotFound,
                format!("minecraft profile '{uuid}'"),
            )
        })
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/minecraft-profiles",
    tag = "admin",
    operation_id = "admin_list_minecraft_profiles",
    params(LimitQuery, AdminMinecraftProfileListQuery),
    responses(
        (status = 200, description = "Minecraft profiles", body = inline(ApiResponse<CursorPage<yggdrasil_service::MinecraftProfileInfo, IdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_minecraft_profiles(
    state: web::Data<AppState>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminMinecraftProfileListQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let after_id = parse_id_cursor(query.after_id, "minecraft profile")?;
    let limit = page.limit_or(50, 100);
    tracing::debug!(
        limit,
        has_cursor = after_id.is_some(),
        user_id = query.user_id,
        has_name = query.name.is_some(),
        has_uuid = query.uuid.is_some(),
        has_query = query.query.is_some(),
        "admin listing minecraft profiles"
    );
    let filters = minecraft_profile_repo::MinecraftProfileFilters {
        user_id: query.user_id,
        name: query.name.clone(),
        uuid: query.uuid.clone(),
        query: query.query.clone(),
    };
    let slice =
        minecraft_profile_repo::list_cursor(state.get_ref().reader_db(), filters, limit, after_id)
            .await?;
    let next_cursor = if slice.has_more {
        slice
            .items
            .last()
            .map(|profile| IdCursor { id: profile.id })
    } else {
        None
    };
    let items = slice
        .items
        .iter()
        .map(yggdrasil_service::profile_info)
        .collect::<Vec<_>>();
    tracing::debug!(
        returned = items.len(),
        total = slice.total,
        has_next_cursor = next_cursor.is_some(),
        "admin listed minecraft profiles cursor page"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(CursorPage::new(
        items,
        slice.total,
        limit,
        next_cursor,
    ))))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/users/{user_id}/minecraft-profiles",
    tag = "admin",
    operation_id = "admin_list_user_minecraft_profiles",
    params(("user_id" = i64, Path, description = "User ID"), LimitQuery, AdminMinecraftProfileListQuery),
    responses(
        (status = 200, description = "Minecraft profiles owned by the user", body = inline(ApiResponse<CursorPage<YggdrasilProfile, IdCursor>>)),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "User not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_user_minecraft_profiles(
    state: web::Data<AppState>,
    path: web::Path<i64>,
    page: web::Query<LimitQuery>,
    query: web::Query<AdminMinecraftProfileListQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let user_id = path.into_inner();
    let after_id = parse_id_cursor(query.after_id, "minecraft profile")?;
    let limit = page.limit_or(50, 100);
    tracing::debug!(
        user_id,
        limit,
        has_cursor = after_id.is_some(),
        "admin listing user minecraft profiles"
    );
    crate::db::repository::user_repo::find_by_id(state.get_ref().reader_db(), user_id).await?;
    let slice = minecraft_profile_repo::list_cursor(
        state.get_ref().reader_db(),
        minecraft_profile_repo::MinecraftProfileFilters {
            user_id: Some(user_id),
            ..Default::default()
        },
        limit,
        after_id,
    )
    .await?;
    let next_cursor = if slice.has_more {
        slice
            .items
            .last()
            .map(|profile| IdCursor { id: profile.id })
    } else {
        None
    };
    let profiles = slice
        .items
        .iter()
        .map(yggdrasil_service::profile_summary)
        .collect::<Vec<_>>();
    tracing::debug!(
        user_id,
        returned = profiles.len(),
        total = slice.total,
        has_next_cursor = next_cursor.is_some(),
        "admin listed user minecraft profiles"
    );

    Ok(HttpResponse::Ok().json(ApiResponse::ok(CursorPage::new(
        profiles,
        slice.total,
        limit,
        next_cursor,
    ))))
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/minecraft-profiles/{uuid}",
    tag = "admin",
    operation_id = "admin_get_minecraft_profile",
    params(("uuid" = String, Path, description = "Unsigned Minecraft profile UUID")),
    responses(
        (status = 200, description = "Minecraft profile", body = inline(ApiResponse<yggdrasil_service::MinecraftProfileInfo>)),
        (status = 400, description = "Invalid profile UUID"),
        (status = 404, description = "Profile not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn get_minecraft_profile(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let uuid = path.into_inner();
    tracing::debug!(profile_uuid = %uuid, "admin loading minecraft profile");
    validate_profile_uuid(&uuid)?;
    let profile = find_profile_by_uuid(state.get_ref(), &uuid).await?;
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        user_id = profile.user_id,
        "admin loaded minecraft profile"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(yggdrasil_service::profile_info(&profile))))
}

#[aster_forge_api_docs_macros::path(
    put,
    path = "/api/v1/admin/minecraft-profiles/{uuid}/name",
    tag = "admin",
    operation_id = "admin_rename_minecraft_profile",
    request_body = RenameMinecraftProfileReq,
    params(("uuid" = String, Path, description = "Unsigned Minecraft profile UUID")),
    responses(
        (status = 200, description = "Renamed Minecraft profile", body = inline(ApiResponse<yggdrasil_service::MinecraftProfileInfo>)),
        (status = 400, description = "Invalid profile UUID/name or duplicate profile"),
        (status = 404, description = "Profile not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn rename_minecraft_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<RenameMinecraftProfileReq>,
) -> Result<HttpResponse> {
    let uuid = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        new_profile_name_len = body.name.len(),
        "admin renaming minecraft profile"
    );
    validate_request(&*body)?;
    validate_profile_uuid(&uuid)?;
    let profile = find_profile_by_uuid(state.get_ref(), &uuid).await?;
    let renamed = yggdrasil_service::rename_profile(state.get_ref(), profile, &body.name).await?;

    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    crate::api::routes::profiles::log_profile_rename_audit(state.get_ref(), &ctx, &renamed).await;

    tracing::debug!(
        profile_id = renamed.profile.id,
        profile_uuid = %renamed.profile.uuid,
        user_id = renamed.profile.user_id,
        temporarily_invalidated_token_count = renamed.temporarily_invalidated_token_count,
        "admin renamed minecraft profile"
    );
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(yggdrasil_service::profile_info(
            &renamed.profile,
        ))),
    )
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/admin/minecraft-profiles/{uuid}/textures",
    tag = "admin",
    operation_id = "admin_list_minecraft_profile_textures",
    params(("uuid" = String, Path, description = "Unsigned Minecraft profile UUID")),
    responses(
        (status = 200, description = "Minecraft profile textures", body = inline(ApiResponse<Vec<texture_service::MinecraftTextureMetadata>>)),
        (status = 400, description = "Invalid profile UUID"),
        (status = 404, description = "Profile not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_minecraft_profile_textures(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let uuid = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        "admin listing minecraft profile textures"
    );
    validate_profile_uuid(&uuid)?;
    let profile = find_profile_by_uuid(state.get_ref(), &uuid).await?;
    let textures = texture_service::texture_metadata_for_profile(state.get_ref(), &profile).await?;
    tracing::debug!(
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        count = textures.len(),
        "admin listed minecraft profile textures"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(textures)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/minecraft-profiles/{uuid}",
    tag = "admin",
    operation_id = "admin_delete_minecraft_profile",
    params(("uuid" = String, Path, description = "Unsigned Minecraft profile UUID")),
    responses(
        (status = 204, description = "Minecraft profile deleted"),
        (status = 400, description = "Invalid profile UUID"),
        (status = 404, description = "Profile not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_minecraft_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let uuid = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        "admin deleting minecraft profile"
    );
    validate_profile_uuid(&uuid)?;
    let profile = find_profile_by_uuid(state.get_ref(), &uuid).await?;
    let Some(deleted) =
        yggdrasil_service::delete_profile_for_user(state.get_ref(), profile.user_id, &profile.uuid)
            .await?
    else {
        return Err(AsterError::record_not_found(format!(
            "minecraft profile '{uuid}'"
        )));
    };

    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    crate::api::routes::profiles::log_profile_delete_audit(state.get_ref(), &ctx, &deleted).await;

    tracing::debug!(
        profile_id = deleted.profile.id,
        profile_uuid = %deleted.profile.uuid,
        user_id = deleted.profile.user_id,
        deleted_texture_count = deleted.deleted_texture_count,
        revoked_token_count = deleted.revoked_token_count,
        "admin deleted minecraft profile"
    );
    Ok(HttpResponse::NoContent().finish())
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/minecraft-profiles/{uuid}/textures/{texture_type}",
    tag = "admin",
    operation_id = "admin_delete_minecraft_profile_texture",
    params(
        ("uuid" = String, Path, description = "Unsigned Minecraft profile UUID"),
        ("texture_type" = String, Path, description = "Texture type: skin or cape"),
    ),
    responses(
        (status = 204, description = "Texture deleted or already absent"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Profile not found"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_minecraft_profile_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (uuid, texture_type_raw) = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        texture_type_raw = %texture_type_raw,
        "admin deleting minecraft profile texture"
    );
    validate_profile_uuid(&uuid)?;
    let texture_type = match texture_service::parse_texture_type(&texture_type_raw) {
        Ok(texture_type) => texture_type,
        Err(error) => {
            tracing::debug!(
                profile_uuid = %uuid,
                texture_type_raw = %texture_type_raw,
                "admin minecraft profile texture delete rejected invalid texture type"
            );
            return Err(AsterError::validation_error(error.protocol_message()));
        }
    };
    let profile = find_profile_by_uuid(state.get_ref(), &uuid).await?;
    let Some(deleted) = texture_service::delete_texture_for_profile_unchecked(
        state.get_ref(),
        &profile,
        texture_type,
    )
    .await
    .map_err(|error| AsterError::internal_error(error.protocol_message()))?
    else {
        tracing::debug!(
            profile_id = profile.id,
            texture_type = ?texture_type,
            "admin minecraft profile texture delete found no texture"
        );
        return Ok(HttpResponse::NoContent().finish());
    };

    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftTextureDelete,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(deleted.texture.binding.id),
        Some(&deleted.profile.name),
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: &deleted.profile.uuid,
                profile_name: &deleted.profile.name,
                texture_type,
                texture_hash: Some(&deleted.texture.texture.hash),
                texture_model: Some(deleted.texture.texture.texture_model),
                width: Some(deleted.texture.texture.width),
                height: Some(deleted.texture.texture.height),
                file_size: Some(deleted.texture.texture.file_size),
                library_status: None,
                review_note: None,
            })
        },
    )
    .await;

    tracing::debug!(
        profile_id = deleted.profile.id,
        profile_uuid = %deleted.profile.uuid,
        profile_texture_id = deleted.texture.binding.id,
        texture_type = ?texture_type,
        "admin deleted minecraft profile texture"
    );
    Ok(HttpResponse::NoContent().finish())
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/admin/minecraft-textures/{hash}",
    tag = "admin",
    operation_id = "admin_delete_minecraft_textures_by_hash",
    params(("hash" = String, Path, description = "Texture SHA-256 hash")),
    responses(
        (status = 204, description = "Textures deleted or already absent"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_minecraft_textures_by_hash(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let hash = path.into_inner();
    tracing::debug!(hash, "admin deleting minecraft textures by hash");
    let deleted = texture_service::delete_textures_by_hash(state.get_ref(), &hash)
        .await
        .map_err(|error| AsterError::internal_error(error.protocol_message()))?;
    let ctx = audit_service::AuditContext::from_request(&req, current_admin_user_id(&req)?);
    let deleted_count = deleted.len();
    for item in deleted {
        audit_service::log_with_details(
            state.get_ref(),
            &ctx,
            audit_service::AuditAction::MinecraftTextureDelete,
            audit_service::AuditEntityType::MinecraftTexture,
            Some(item.texture.binding.id),
            Some(&item.profile.name),
            || {
                audit_service::details(audit_service::MinecraftTextureAuditDetails {
                    profile_uuid: &item.profile.uuid,
                    profile_name: &item.profile.name,
                    texture_type: item.texture.binding.texture_type,
                    texture_hash: Some(&item.texture.texture.hash),
                    texture_model: Some(item.texture.texture.texture_model),
                    width: Some(item.texture.texture.width),
                    height: Some(item.texture.texture.height),
                    file_size: Some(item.texture.texture.file_size),
                    library_status: None,
                    review_note: None,
                })
            },
        )
        .await;
    }
    tracing::debug!(
        hash,
        deleted_count,
        "admin deleted minecraft textures by hash"
    );
    Ok(HttpResponse::NoContent().finish())
}
