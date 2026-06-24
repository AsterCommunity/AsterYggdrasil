//! Current-user Minecraft profile routes.

use crate::api::dto::validation::validate_unsigned_uuid;
#[cfg(all(debug_assertions, feature = "openapi"))]
use crate::api::dto::yggdrasil::YggdrasilProfile;
use crate::api::dto::{
    BindMinecraftTextureReq, CreateMinecraftProfileReq, CurrentMinecraftProfileListQuery,
    RenameMinecraftProfileReq, validate_request,
};
use crate::api::error_code::AsterErrorCode;
use crate::api::pagination::{CursorPage, IdCursor, LimitQuery, parse_id_cursor};
use crate::api::response::ApiResponse;
use crate::db::repository::minecraft_profile_repo;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::{audit_service, auth_service, texture_service, yggdrasil_service};
use crate::types::MinecraftTextureType;
use actix_web::{HttpRequest, HttpResponse, web};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/profiles")
            .route("/minecraft", web::get().to(list_minecraft_profiles))
            .route("/minecraft", web::post().to(create_minecraft_profile))
            .route(
                "/minecraft/{uuid}/name",
                web::put().to(rename_minecraft_profile),
            )
            .route(
                "/minecraft/{uuid}/textures",
                web::get().to(list_minecraft_profile_textures),
            )
            .route(
                "/minecraft/{uuid}/textures/{texture_type}",
                web::put().to(bind_minecraft_profile_texture),
            )
            .route(
                "/minecraft/{uuid}/textures/{texture_type}",
                web::delete().to(unbind_minecraft_profile_texture),
            )
            .route(
                "/minecraft/{uuid}",
                web::delete().to(delete_minecraft_profile),
            ),
    );
}

#[aster_forge_api_docs_macros::path(
    put,
    path = "/api/v1/profiles/minecraft/{uuid}/textures/{texture_type}",
    tag = "profiles",
    operation_id = "bind_current_user_minecraft_profile_texture",
    request_body = BindMinecraftTextureReq,
    params(
        ("uuid" = String, Path, description = "Unsigned Minecraft profile UUID"),
        ("texture_type" = String, Path, description = "Texture type: skin or cape"),
    ),
    responses(
        (status = 200, description = "Minecraft profile texture bound", body = inline(ApiResponse<texture_service::MinecraftTextureMetadata>)),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Texture slot is not uploadable"),
        (status = 404, description = "Profile or wardrobe texture not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn bind_minecraft_profile_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<BindMinecraftTextureReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        texture_id = body.texture_id,
        "received current user minecraft profile texture bind request"
    );
    validate_request(&*body)?;
    let (uuid, texture_type_raw) = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        texture_type_raw = %texture_type_raw,
        texture_id = body.texture_id,
        "binding current user minecraft profile texture"
    );
    if let Err(error) = validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "minecraft profile texture bind rejected invalid profile uuid"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::MinecraftProfileUuidInvalid,
            error.message.unwrap_or_default(),
        ));
    }
    let texture_type = parse_profile_texture_type(&texture_type_raw)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let Some(profile) =
        minecraft_profile_repo::find_by_uuid_for_user(state.get_ref().reader_db(), &uuid, user.id)
            .await?
    else {
        tracing::debug!(
            user_id = user.id,
            profile_uuid = %uuid,
            "minecraft profile texture bind rejected missing profile"
        );
        return Err(AsterError::record_not_found_code(
            AsterErrorCode::MinecraftProfileNotFound,
            format!("minecraft profile '{uuid}'"),
        ));
    };
    let stored = texture_service::bind_wardrobe_texture_to_profile(
        state.get_ref(),
        user.id,
        &profile,
        body.texture_id,
        texture_type,
    )
    .await
    .map_err(texture_error_to_api_error)?;

    let ctx = audit_service::AuditContext::from_request(&req, user.id);
    log_profile_texture_audit(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftTextureBind,
        &stored,
    )
    .await;

    tracing::debug!(
        user_id = user.id,
        profile_id = stored.profile.id,
        profile_texture_id = stored.texture.binding.id,
        texture_id = stored.texture.texture.id,
        texture_type = ?texture_type,
        "current user minecraft profile texture bound"
    );
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(texture_service::texture_metadata(
            state.get_ref(),
            &stored.profile,
            &stored.texture,
        ))),
    )
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/profiles/minecraft/{uuid}/textures/{texture_type}",
    tag = "profiles",
    operation_id = "unbind_current_user_minecraft_profile_texture",
    params(
        ("uuid" = String, Path, description = "Unsigned Minecraft profile UUID"),
        ("texture_type" = String, Path, description = "Texture type: skin or cape"),
    ),
    responses(
        (status = 204, description = "Minecraft profile texture unbound or already absent"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Texture slot is not uploadable"),
        (status = 404, description = "Profile not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn unbind_minecraft_profile_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (uuid, texture_type_raw) = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        texture_type_raw = %texture_type_raw,
        "received current user minecraft profile texture unbind request"
    );
    if let Err(error) = validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "minecraft profile texture unbind rejected invalid profile uuid"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::MinecraftProfileUuidInvalid,
            error.message.unwrap_or_default(),
        ));
    }
    let texture_type = parse_profile_texture_type(&texture_type_raw)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let Some(profile) =
        minecraft_profile_repo::find_by_uuid_for_user(state.get_ref().reader_db(), &uuid, user.id)
            .await?
    else {
        tracing::debug!(
            user_id = user.id,
            profile_uuid = %uuid,
            "minecraft profile texture unbind rejected missing profile"
        );
        return Err(AsterError::record_not_found_code(
            AsterErrorCode::MinecraftProfileNotFound,
            format!("minecraft profile '{uuid}'"),
        ));
    };
    let deleted =
        texture_service::delete_texture_for_profile(state.get_ref(), &profile, texture_type)
            .await
            .map_err(texture_error_to_api_error)?;

    let ctx = audit_service::AuditContext::from_request(&req, user.id);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftTextureDelete,
        audit_service::AuditEntityType::MinecraftTexture,
        deleted.as_ref().map(|item| item.texture.binding.id),
        Some(&profile.name),
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: &profile.uuid,
                profile_name: &profile.name,
                texture_type,
                texture_hash: deleted
                    .as_ref()
                    .map(|item| item.texture.texture.hash.as_str()),
                texture_model: deleted
                    .as_ref()
                    .map(|item| item.texture.texture.texture_model),
                width: deleted.as_ref().map(|item| item.texture.texture.width),
                height: deleted.as_ref().map(|item| item.texture.texture.height),
                file_size: deleted.as_ref().map(|item| item.texture.texture.file_size),
                library_status: None,
                review_note: None,
            })
        },
    )
    .await;

    tracing::debug!(
        user_id = user.id,
        profile_id = profile.id,
        texture_type = ?texture_type,
        deleted = deleted.is_some(),
        "current user minecraft profile texture unbound"
    );
    Ok(HttpResponse::NoContent().finish())
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/profiles/minecraft",
    tag = "profiles",
    operation_id = "list_current_user_minecraft_profiles",
    params(LimitQuery, CurrentMinecraftProfileListQuery),
    responses(
        (status = 200, description = "Current user's Minecraft profiles", body = inline(ApiResponse<CursorPage<YggdrasilProfile, IdCursor>>)),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_minecraft_profiles(
    state: web::Data<AppState>,
    req: HttpRequest,
    page: web::Query<LimitQuery>,
    query: web::Query<CurrentMinecraftProfileListQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let after_id = parse_id_cursor(query.after_id, "minecraft profile")?;
    let limit = page.limit_or(50, 100);
    tracing::debug!(
        user_id = user.id,
        limit,
        has_cursor = after_id.is_some(),
        has_query = query.query.is_some(),
        "listing current user minecraft profiles"
    );
    let slice = minecraft_profile_repo::list_cursor(
        state.get_ref().reader_db(),
        minecraft_profile_repo::MinecraftProfileFilters {
            user_id: Some(user.id),
            query: query.query.clone(),
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
        user_id = user.id,
        returned = profiles.len(),
        total = slice.total,
        has_next_cursor = next_cursor.is_some(),
        "listed current user minecraft profiles"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(CursorPage::new(
        profiles,
        slice.total,
        limit,
        next_cursor,
    ))))
}

#[aster_forge_api_docs_macros::path(
    post,
    path = "/api/v1/profiles/minecraft",
    tag = "profiles",
    operation_id = "create_current_user_minecraft_profile",
    request_body = CreateMinecraftProfileReq,
    responses(
        (status = 200, description = "Created Minecraft profile", body = inline(ApiResponse<YggdrasilProfile>)),
        (status = 400, description = "Invalid profile name or duplicate profile"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
pub async fn create_minecraft_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateMinecraftProfileReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        profile_name_len = body.name.len(),
        "received current user minecraft profile create request"
    );
    validate_request(&*body)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let profile = yggdrasil_service::create_profile(state.get_ref(), user.id, &body.name).await?;
    let ctx = audit_service::AuditContext::from_request(&req, user.id);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftProfileCreate,
        audit_service::AuditEntityType::MinecraftProfile,
        Some(profile.id),
        Some(&profile.name),
        || {
            audit_service::details(audit_service::MinecraftProfileAuditDetails {
                profile_uuid: &profile.uuid,
                profile_name: &profile.name,
                deleted_texture_count: None,
                revoked_token_count: None,
            })
        },
    )
    .await;
    tracing::debug!(
        user_id = user.id,
        profile_id = profile.id,
        profile_uuid = %profile.uuid,
        "current user minecraft profile created"
    );
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(yggdrasil_service::profile_summary(
            &profile,
        ))),
    )
}

#[aster_forge_api_docs_macros::path(
    put,
    path = "/api/v1/profiles/minecraft/{uuid}/name",
    tag = "profiles",
    operation_id = "rename_current_user_minecraft_profile",
    request_body = RenameMinecraftProfileReq,
    params(("uuid" = String, Path, description = "Unsigned Minecraft profile UUID")),
    responses(
        (status = 200, description = "Renamed Minecraft profile", body = inline(ApiResponse<YggdrasilProfile>)),
        (status = 400, description = "Invalid profile UUID/name or duplicate profile"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Profile not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn rename_minecraft_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<RenameMinecraftProfileReq>,
) -> Result<HttpResponse> {
    tracing::debug!(
        new_profile_name_len = body.name.len(),
        "received current user minecraft profile rename request"
    );
    validate_request(&*body)?;
    let uuid = path.into_inner();
    if let Err(error) = validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "minecraft profile rename rejected invalid profile uuid"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::MinecraftProfileUuidInvalid,
            error.message.unwrap_or_default(),
        ));
    }
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let Some(renamed) =
        yggdrasil_service::rename_profile_for_user(state.get_ref(), user.id, &uuid, &body.name)
            .await?
    else {
        tracing::debug!(
            user_id = user.id,
            profile_uuid = %uuid,
            "minecraft profile rename rejected missing profile"
        );
        return Err(AsterError::record_not_found_code(
            AsterErrorCode::MinecraftProfileNotFound,
            format!("minecraft profile '{uuid}'"),
        ));
    };

    let ctx = audit_service::AuditContext::from_request(&req, user.id);
    log_profile_rename_audit(state.get_ref(), &ctx, &renamed).await;

    tracing::debug!(
        user_id = user.id,
        profile_id = renamed.profile.id,
        profile_uuid = %renamed.profile.uuid,
        temporarily_invalidated_token_count = renamed.temporarily_invalidated_token_count,
        "current user minecraft profile renamed"
    );
    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(yggdrasil_service::profile_summary(
            &renamed.profile,
        ))),
    )
}

#[aster_forge_api_docs_macros::path(
    get,
    path = "/api/v1/profiles/minecraft/{uuid}/textures",
    tag = "profiles",
    operation_id = "list_current_user_minecraft_profile_textures",
    params(("uuid" = String, Path, description = "Unsigned Minecraft profile UUID")),
    responses(
        (status = 200, description = "Current user's Minecraft profile textures", body = inline(ApiResponse<Vec<texture_service::MinecraftTextureMetadata>>)),
        (status = 400, description = "Invalid profile UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Profile not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_minecraft_profile_textures(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let uuid = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        "received current user minecraft profile textures list request"
    );
    if let Err(error) = validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "minecraft profile texture list rejected invalid profile uuid"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::MinecraftProfileUuidInvalid,
            error.message.unwrap_or_default(),
        ));
    }
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let Some(profile) =
        minecraft_profile_repo::find_by_uuid_for_user(state.get_ref().reader_db(), &uuid, user.id)
            .await?
    else {
        tracing::debug!(
            user_id = user.id,
            profile_uuid = %uuid,
            "minecraft profile texture list rejected missing profile"
        );
        return Err(AsterError::record_not_found_code(
            AsterErrorCode::MinecraftProfileNotFound,
            format!("minecraft profile '{uuid}'"),
        ));
    };
    let textures = texture_service::texture_metadata_for_profile(state.get_ref(), &profile).await?;
    tracing::debug!(
        user_id = user.id,
        profile_id = profile.id,
        count = textures.len(),
        "listed current user minecraft profile textures"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(textures)))
}

#[aster_forge_api_docs_macros::path(
    delete,
    path = "/api/v1/profiles/minecraft/{uuid}",
    tag = "profiles",
    operation_id = "delete_current_user_minecraft_profile",
    params(("uuid" = String, Path, description = "Unsigned Minecraft profile UUID")),
    responses(
        (status = 204, description = "Minecraft profile deleted"),
        (status = 400, description = "Invalid profile UUID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Profile not found"),
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
        "received current user minecraft profile delete request"
    );
    if let Err(error) = validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "minecraft profile delete rejected invalid profile uuid"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::MinecraftProfileUuidInvalid,
            error.message.unwrap_or_default(),
        ));
    }
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let Some(deleted) =
        yggdrasil_service::delete_profile_for_user(state.get_ref(), user.id, &uuid).await?
    else {
        tracing::debug!(
            user_id = user.id,
            profile_uuid = %uuid,
            "minecraft profile delete rejected missing profile"
        );
        return Err(AsterError::record_not_found_code(
            AsterErrorCode::MinecraftProfileNotFound,
            format!("minecraft profile '{uuid}'"),
        ));
    };

    let ctx = audit_service::AuditContext::from_request(&req, user.id);
    log_profile_delete_audit(state.get_ref(), &ctx, &deleted).await;

    tracing::debug!(
        user_id = user.id,
        profile_id = deleted.profile.id,
        profile_uuid = %deleted.profile.uuid,
        deleted_texture_count = deleted.deleted_texture_count,
        revoked_token_count = deleted.revoked_token_count,
        "current user minecraft profile deleted"
    );
    Ok(HttpResponse::NoContent().finish())
}

pub(crate) async fn log_profile_delete_audit(
    state: &AppState,
    ctx: &audit_service::AuditContext,
    deleted: &yggdrasil_service::DeleteMinecraftProfileResult,
) {
    audit_service::log_with_details(
        state,
        ctx,
        audit_service::AuditAction::MinecraftProfileDelete,
        audit_service::AuditEntityType::MinecraftProfile,
        Some(deleted.profile.id),
        Some(&deleted.profile.name),
        || {
            audit_service::details(audit_service::MinecraftProfileAuditDetails {
                profile_uuid: &deleted.profile.uuid,
                profile_name: &deleted.profile.name,
                deleted_texture_count: Some(deleted.deleted_texture_count),
                revoked_token_count: Some(deleted.revoked_token_count),
            })
        },
    )
    .await;
}

pub(crate) async fn log_profile_rename_audit(
    state: &AppState,
    ctx: &audit_service::AuditContext,
    renamed: &yggdrasil_service::RenameMinecraftProfileResult,
) {
    audit_service::log_with_details(
        state,
        ctx,
        audit_service::AuditAction::MinecraftProfileRename,
        audit_service::AuditEntityType::MinecraftProfile,
        Some(renamed.profile.id),
        Some(&renamed.profile.name),
        || {
            audit_service::details(audit_service::MinecraftProfileRenameAuditDetails {
                profile_uuid: &renamed.profile.uuid,
                old_profile_name: &renamed.old_name,
                new_profile_name: &renamed.profile.name,
                temporarily_invalidated_token_count: renamed.temporarily_invalidated_token_count,
            })
        },
    )
    .await;
}

pub(crate) async fn log_profile_texture_audit(
    state: &AppState,
    ctx: &audit_service::AuditContext,
    action: audit_service::AuditAction,
    stored: &texture_service::StoredTexture,
) {
    audit_service::log_with_details(
        state,
        ctx,
        action,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(stored.texture.binding.id),
        Some(&stored.profile.name),
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: &stored.profile.uuid,
                profile_name: &stored.profile.name,
                texture_type: stored.texture.binding.texture_type,
                texture_hash: Some(&stored.texture.texture.hash),
                texture_model: Some(stored.texture.texture.texture_model),
                width: Some(stored.texture.texture.width),
                height: Some(stored.texture.texture.height),
                file_size: Some(stored.texture.texture.file_size),
                library_status: None,
                review_note: None,
            })
        },
    )
    .await;
}

fn parse_profile_texture_type(value: &str) -> Result<MinecraftTextureType> {
    texture_service::parse_texture_type(value).map_err(|_| {
        tracing::debug!(
            texture_type_raw = value,
            "minecraft profile texture request rejected invalid texture type"
        );
        AsterError::validation_error_code(
            AsterErrorCode::MinecraftTextureInvalidType,
            "invalid texture type",
        )
    })
}

fn texture_error_to_api_error(error: texture_service::TextureError) -> AsterError {
    match error.kind() {
        texture_service::TextureErrorKind::InvalidToken => AsterError::auth_unauthorized_code(
            AsterErrorCode::AuthTokenInvalid,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::ForbiddenProfile => {
            AsterError::auth_forbidden(error.protocol_message())
        }
        texture_service::TextureErrorKind::UploadDisabled => AsterError::auth_forbidden_code(
            AsterErrorCode::MinecraftTextureUploadDisabled,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::UserBanForbidden => AsterError::auth_forbidden_code(
            AsterErrorCode::UserBanForbidden,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::NotFound => AsterError::record_not_found_code(
            AsterErrorCode::MinecraftTextureNotFound,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::Storage => AsterError::internal_error_code(
            AsterErrorCode::MinecraftObjectStorageFailed,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::InvalidTextureType => AsterError::validation_error_code(
            AsterErrorCode::MinecraftTextureInvalidType,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::InvalidContentType => AsterError::validation_error_code(
            AsterErrorCode::MinecraftTextureUnsupportedMime,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::MissingFile => AsterError::validation_error_code(
            AsterErrorCode::MinecraftTextureInvalidPng,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::InvalidPng => AsterError::validation_error_code(
            AsterErrorCode::MinecraftTextureInvalidPng,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::InvalidDimensions => AsterError::validation_error_code(
            AsterErrorCode::MinecraftTextureInvalidDimensions,
            error.protocol_message(),
        ),
    }
}
