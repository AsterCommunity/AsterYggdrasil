//! Current-user Minecraft wardrobe texture routes.

use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, web};
use futures::StreamExt;
use serde::Deserialize;
use validator::Validate;

use crate::api::dto::validate_request;
use crate::api::error_code::AsterErrorCode;
use crate::api::pagination::{LimitOffsetQuery, OffsetPage};
use crate::api::response::ApiResponse;
use crate::db::repository::minecraft_texture_repo::WardrobeTextureListFilter;
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::{audit_service, auth_service, texture_service};
use crate::types::{MinecraftTextureModel, MinecraftTextureType, MinecraftTextureVisibility};

#[derive(Debug, Clone, Default, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(utoipa::IntoParams, utoipa::ToSchema)
)]
pub struct WardrobeTextureListQuery {
    #[validate(length(max = 96, message = "keyword must not exceed 96 characters"))]
    pub keyword: Option<String>,
    pub texture_type: Option<MinecraftTextureType>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/wardrobe")
            .route("/textures", web::get().to(list_wardrobe_textures))
            .route(
                "/textures/{texture_type}",
                web::post().to(upload_wardrobe_texture),
            )
            .route(
                "/textures/{texture_id}",
                web::delete().to(delete_wardrobe_texture),
            ),
    );
}

#[api_docs_macros::path(
    get,
    path = "/api/v1/wardrobe/textures",
    tag = "profiles",
    operation_id = "list_current_user_wardrobe_textures",
    params(LimitOffsetQuery, WardrobeTextureListQuery),
    responses(
        (status = 200, description = "Current user's wardrobe textures", body = inline(ApiResponse<OffsetPage<texture_service::MinecraftWardrobeTextureMetadata>>)),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
pub async fn list_wardrobe_textures(
    state: web::Data<AppState>,
    req: HttpRequest,
    page: web::Query<LimitOffsetQuery>,
    query: web::Query<WardrobeTextureListQuery>,
) -> Result<HttpResponse> {
    validate_request(&*query)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    let limit = page.limit_or(50, 100);
    let offset = page.offset();
    let texture_type = query.texture_type;
    let keyword = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    tracing::debug!(
        user_id = user.id,
        limit,
        offset,
        texture_type = ?texture_type,
        has_keyword = keyword.is_some(),
        "listing current user wardrobe textures"
    );
    let page = texture_service::list_wardrobe_textures_paginated(
        state.get_ref(),
        user.id,
        limit,
        offset,
        WardrobeTextureListFilter {
            texture_type,
            keyword,
        },
    )
    .await?;
    let textures = page
        .items
        .iter()
        .map(|texture| texture_service::wardrobe_texture_metadata(state.get_ref(), texture))
        .collect::<Vec<_>>();
    tracing::debug!(
        user_id = user.id,
        returned = textures.len(),
        total = page.total,
        "listed current user wardrobe textures"
    );
    Ok(HttpResponse::Ok().json(ApiResponse::ok(OffsetPage::new(
        textures,
        page.total,
        page.limit,
        page.offset,
    ))))
}

#[api_docs_macros::path(
    post,
    path = "/api/v1/wardrobe/textures/{texture_type}",
    tag = "profiles",
    operation_id = "upload_current_user_wardrobe_texture",
    request_body(
        content = String,
        content_type = "multipart/form-data",
        description = "Multipart form with PNG file field and optional model field"
    ),
    params(("texture_type" = String, Path, description = "Texture type: skin or cape")),
    responses(
        (status = 200, description = "Wardrobe texture uploaded", body = inline(ApiResponse<texture_service::MinecraftWardrobeTextureMetadata>)),
        (status = 400, description = "Invalid upload or texture"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer" = [])),
)]
pub async fn upload_wardrobe_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    payload: Multipart,
) -> Result<HttpResponse> {
    let texture_type_raw = path.into_inner();
    tracing::debug!(
        texture_type_raw = %texture_type_raw,
        "received wardrobe texture upload request"
    );
    let texture_type = parse_texture_type(&texture_type_raw)?;
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(
        user_id = user.id,
        texture_type = ?texture_type,
        "receiving wardrobe texture upload"
    );
    let upload = receive_texture_upload(&state, payload, texture_type)
        .await
        .map_err(texture_error_to_api_error)?;
    let stored = texture_service::store_wardrobe_texture(
        state.get_ref(),
        user.id,
        texture_type,
        upload.texture_model,
        upload.visibility,
        upload.file_path.clone(),
    )
    .await
    .map_err(texture_error_to_api_error);
    cleanup_upload_file(&upload.file_path).await;
    let stored = stored?;
    tracing::debug!(
        user_id = user.id,
        texture_id = stored.texture.id,
        texture_type = ?stored.texture.texture_type,
        hash = %stored.texture.hash,
        "wardrobe texture upload completed"
    );

    let ctx = audit_service::AuditContext::from_request(&req, user.id);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftTextureUpload,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(stored.texture.id),
        None,
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: "",
                profile_name: "",
                texture_type: stored.texture.texture_type,
                texture_hash: Some(&stored.texture.hash),
                texture_model: Some(stored.texture.texture_model),
                width: Some(stored.texture.width),
                height: Some(stored.texture.height),
                file_size: Some(stored.texture.file_size),
            })
        },
    )
    .await;

    Ok(
        HttpResponse::Ok().json(ApiResponse::ok(texture_service::wardrobe_texture_metadata(
            state.get_ref(),
            &stored.texture,
        ))),
    )
}

#[api_docs_macros::path(
    delete,
    path = "/api/v1/wardrobe/textures/{texture_id}",
    tag = "profiles",
    operation_id = "delete_current_user_wardrobe_texture",
    params(("texture_id" = i64, Path, description = "Wardrobe texture ID")),
    responses(
        (status = 204, description = "Wardrobe texture deleted"),
        (status = 400, description = "Invalid wardrobe texture ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Wardrobe texture not found"),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_wardrobe_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse> {
    let texture_id = path.into_inner();
    tracing::debug!(texture_id, "received wardrobe texture delete request");
    if texture_id <= 0 {
        tracing::debug!(
            texture_id,
            "wardrobe texture delete rejected invalid texture id"
        );
        return Err(AsterError::validation_error_code(
            AsterErrorCode::WardrobeTextureNotFound,
            "invalid wardrobe texture id",
        ));
    }
    let user = auth_service::current_user(state.get_ref(), &req).await?;
    tracing::debug!(
        user_id = user.id,
        texture_id,
        "deleting current user wardrobe texture"
    );
    let deleted = texture_service::delete_wardrobe_texture(state.get_ref(), user.id, texture_id)
        .await
        .map_err(texture_error_to_api_error)?;
    tracing::debug!(
        user_id = user.id,
        texture_id = deleted.id,
        hash = %deleted.hash,
        "deleted current user wardrobe texture"
    );

    let ctx = audit_service::AuditContext::from_request(&req, user.id);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftTextureDelete,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(deleted.id),
        None,
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: "",
                profile_name: "",
                texture_type: deleted.texture_type,
                texture_hash: Some(&deleted.hash),
                texture_model: Some(deleted.texture_model),
                width: Some(deleted.width),
                height: Some(deleted.height),
                file_size: Some(deleted.file_size),
            })
        },
    )
    .await;

    Ok(HttpResponse::NoContent().finish())
}

struct ReceivedTextureUpload {
    texture_model: MinecraftTextureModel,
    visibility: MinecraftTextureVisibility,
    file_path: std::path::PathBuf,
}

async fn receive_texture_upload(
    state: &AppState,
    mut payload: Multipart,
    texture_type: MinecraftTextureType,
) -> std::result::Result<ReceivedTextureUpload, texture_service::TextureError> {
    let policy = crate::config::yggdrasil::RuntimeYggdrasilPolicy::from_runtime_config(
        state.runtime_config(),
    );
    let mut texture_model = MinecraftTextureModel::Default;
    let mut visibility = MinecraftTextureVisibility::Private;
    let mut file_path = None;
    let mut field_count = 0_u64;
    tracing::debug!(
        texture_type = ?texture_type,
        max_texture_upload_bytes = policy.max_texture_upload_bytes,
        "receiving wardrobe texture multipart payload"
    );

    while let Some(field) = payload.next().await {
        field_count += 1;
        let mut field = field.map_err(|error| {
            texture_service::TextureError::with_detail(
                texture_service::TextureErrorKind::InvalidPng,
                format!("Invalid multipart payload: {error}"),
            )
        })?;
        match field.name() {
            Some("model") => {
                tracing::debug!(field_count, "reading wardrobe texture model field");
                let text = read_small_text_field(&mut field).await?;
                texture_model = texture_service::parse_skin_model(Some(&text))?;
                tracing::debug!(
                    field_count,
                    texture_model = ?texture_model,
                    "parsed wardrobe texture model field"
                );
            }
            Some("visibility") => {
                tracing::debug!(field_count, "reading wardrobe texture visibility field");
                let text = read_small_text_field(&mut field).await?;
                visibility =
                    MinecraftTextureVisibility::parse_form_value(&text).ok_or_else(|| {
                        texture_service::TextureError::with_detail(
                            texture_service::TextureErrorKind::InvalidDimensions,
                            "Invalid texture visibility.",
                        )
                    })?;
                tracing::debug!(
                    field_count,
                    visibility = ?visibility,
                    "parsed wardrobe texture visibility field"
                );
            }
            Some("file") => {
                if !is_png_field(&field) {
                    tracing::debug!(
                        field_count,
                        "wardrobe texture upload rejected non-png file field"
                    );
                    return Err(texture_service::TextureError::new(
                        texture_service::TextureErrorKind::InvalidContentType,
                    ));
                }
                let path = texture_upload_temp_path(state);
                tracing::debug!(field_count, "writing wardrobe texture file field");
                texture_service::write_multipart_texture_field_to_file(
                    &mut field,
                    &path,
                    policy.max_texture_upload_bytes,
                )
                .await?;
                file_path = Some(path);
            }
            _ => {
                tracing::debug!(
                    field_count,
                    "draining ignored wardrobe texture multipart field"
                );
                drain_field(&mut field).await?
            }
        }
    }

    if texture_type == MinecraftTextureType::Cape {
        texture_model = MinecraftTextureModel::Default;
    }

    let Some(file_path) = file_path else {
        tracing::debug!(
            field_count,
            "wardrobe texture upload rejected because file field is missing"
        );
        return Err(texture_service::TextureError::new(
            texture_service::TextureErrorKind::MissingFile,
        ));
    };
    tracing::debug!(
        field_count,
        texture_type = ?texture_type,
        texture_model = ?texture_model,
        visibility = ?visibility,
        "received wardrobe texture multipart payload"
    );
    Ok(ReceivedTextureUpload {
        texture_model,
        visibility,
        file_path,
    })
}

async fn read_small_text_field(
    field: &mut actix_multipart::Field,
) -> std::result::Result<String, texture_service::TextureError> {
    let bytes = field.bytes(128).await.map_err(|_| {
        texture_service::TextureError::with_detail(
            texture_service::TextureErrorKind::InvalidDimensions,
            "Multipart text field is too large.",
        )
    })?;
    let bytes = bytes.map_err(|error| {
        texture_service::TextureError::with_detail(
            texture_service::TextureErrorKind::InvalidPng,
            format!("Invalid multipart text field: {error}"),
        )
    })?;
    let text = String::from_utf8(bytes.to_vec()).map_err(|error| {
        texture_service::TextureError::with_detail(
            texture_service::TextureErrorKind::InvalidDimensions,
            format!("Multipart text field is not UTF-8: {error}"),
        )
    })?;
    tracing::debug!(
        len = text.len(),
        "read wardrobe texture text multipart field"
    );
    Ok(text)
}

async fn drain_field(
    field: &mut actix_multipart::Field,
) -> std::result::Result<(), texture_service::TextureError> {
    let mut chunk_count = 0_u64;
    while let Some(chunk) = field.next().await {
        chunk.map_err(|error| {
            texture_service::TextureError::with_detail(
                texture_service::TextureErrorKind::InvalidPng,
                format!("Invalid multipart field: {error}"),
            )
        })?;
        chunk_count += 1;
    }
    tracing::debug!(chunk_count, "drained wardrobe texture multipart field");
    Ok(())
}

async fn cleanup_upload_file(path: &std::path::Path) {
    if let Err(error) = tokio::fs::remove_file(path).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(path = %path.display(), error = %error, "failed to remove upload temp texture");
    }
}

fn texture_upload_temp_path(state: &AppState) -> std::path::PathBuf {
    std::path::Path::new(&state.config().server.temp_dir)
        .join("_runtime")
        .join(format!(
            "wardrobe-texture-upload-{}.png",
            uuid::Uuid::new_v4().simple()
        ))
}

fn is_png_field(field: &actix_multipart::Field) -> bool {
    field
        .content_type()
        .map(|mime| mime.type_().as_str() == "image" && mime.subtype().as_str() == "png")
        .unwrap_or(false)
}

fn parse_texture_type(value: &str) -> Result<MinecraftTextureType> {
    texture_service::parse_texture_type(value).map_err(|_| {
        tracing::debug!(
            texture_type_raw = value,
            "wardrobe texture request rejected invalid texture type"
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
        texture_service::TextureErrorKind::NotFound => AsterError::record_not_found_code(
            AsterErrorCode::WardrobeTextureNotFound,
            error.protocol_message(),
        ),
        texture_service::TextureErrorKind::Storage => AsterError::internal_error_code(
            AsterErrorCode::MinecraftTextureStorageFailed,
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
