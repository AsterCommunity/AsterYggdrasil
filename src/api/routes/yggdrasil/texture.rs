use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, ResponseError, web};
use futures::StreamExt;

use crate::api::cache::{not_modified_response, request_etag_matches, weak_etag_for_sha256_hash};
use crate::runtime::AppState;
use crate::services::yggdrasil_service::{YggdrasilError, YggdrasilErrorKind};
use crate::services::{audit_service, texture_service, yggdrasil_service};
use crate::types::MinecraftTextureType;

use super::yggdrasil_error_response;

#[api_docs_macros::path(
    put,
    path = "/api/yggdrasil/api/user/profile/{uuid}/{texture_type}",
    tag = "yggdrasil",
    operation_id = "yggdrasil_upload_texture",
    request_body(
        content = String,
        content_type = "multipart/form-data",
        description = "Multipart form with PNG file field and optional model field"
    ),
    params(
        ("uuid" = String, Path, description = "Unsigned Minecraft profile UUID"),
        ("texture_type" = String, Path, description = "Texture type: skin or cape"),
    ),
    responses(
        (status = 204, description = "Texture uploaded"),
        (status = 400, description = "Invalid upload or texture", body = crate::api::dto::yggdrasil::YggdrasilErrorBody),
        (status = 401, description = "Invalid token", body = crate::api::dto::yggdrasil::YggdrasilErrorBody),
        (status = 403, description = "Forbidden profile", body = crate::api::dto::yggdrasil::YggdrasilErrorBody),
    ),
    security(("bearer" = [])),
)]
pub async fn upload_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    payload: Multipart,
) -> HttpResponse {
    let (uuid, texture_type_raw) = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        texture_type_raw = %texture_type_raw,
        "received yggdrasil texture upload request"
    );
    if let Err(error) = crate::api::dto::validation::validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "yggdrasil texture upload rejected invalid profile uuid"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message.unwrap_or_default(),
        ));
    }
    let texture_type = match texture_service::parse_texture_type(&texture_type_raw) {
        Ok(texture_type) => texture_type,
        Err(error) => return texture_error_response(error),
    };
    // authlib-injector explicitly requires 401 for missing Authorization on
    // texture upload/delete, even though other Yggdrasil invalid-token cases
    // in this codebase still use 403.
    let Some(access_token) = crate::api::request_auth::bearer_token(&req) else {
        tracing::debug!(
            profile_uuid = %uuid,
            texture_type = ?texture_type,
            "yggdrasil texture upload rejected missing bearer token"
        );
        return texture_error_response(texture_service::TextureError::new(
            texture_service::TextureErrorKind::InvalidToken,
        ));
    };
    let (token, profile) =
        match texture_service::authenticate_texture_access(state.get_ref(), &access_token, &uuid)
            .await
        {
            Ok(value) => value,
            Err(error) => return texture_error_response(error),
        };
    tracing::debug!(
        user_id = token.user_id,
        token_id = token.id,
        profile_id = profile.id,
        texture_type = ?texture_type,
        "yggdrasil texture upload authenticated"
    );

    let upload = match receive_texture_upload(&state, payload, texture_type).await {
        Ok(upload) => upload,
        Err(error) => return texture_error_response(error),
    };
    let stored = match texture_service::store_texture(
        state.get_ref(),
        &profile,
        texture_type,
        upload.texture_model,
        upload.file_path.clone(),
    )
    .await
    {
        Ok(stored) => stored,
        Err(error) => {
            cleanup_upload_file(&upload.file_path).await;
            return texture_error_response(error);
        }
    };
    cleanup_upload_file(&upload.file_path).await;

    let ctx = audit_service::AuditContext::from_request(&req, token.user_id);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftTextureUpload,
        audit_service::AuditEntityType::MinecraftTexture,
        Some(stored.texture.binding.id),
        Some(&stored.profile.name),
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: &stored.profile.uuid,
                profile_name: &stored.profile.name,
                texture_type,
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

    tracing::debug!(
        user_id = token.user_id,
        profile_id = stored.profile.id,
        profile_texture_id = stored.texture.binding.id,
        texture_id = stored.texture.texture.id,
        texture_type = ?texture_type,
        hash = %stored.texture.texture.hash,
        "yggdrasil texture upload completed"
    );
    HttpResponse::NoContent().finish()
}

#[api_docs_macros::path(
    delete,
    path = "/api/yggdrasil/api/user/profile/{uuid}/{texture_type}",
    tag = "yggdrasil",
    operation_id = "yggdrasil_delete_texture",
    params(
        ("uuid" = String, Path, description = "Unsigned Minecraft profile UUID"),
        ("texture_type" = String, Path, description = "Texture type: skin or cape"),
    ),
    responses(
        (status = 204, description = "Texture deleted or already absent"),
        (status = 400, description = "Invalid request", body = crate::api::dto::yggdrasil::YggdrasilErrorBody),
        (status = 401, description = "Invalid token", body = crate::api::dto::yggdrasil::YggdrasilErrorBody),
        (status = 403, description = "Forbidden profile", body = crate::api::dto::yggdrasil::YggdrasilErrorBody),
    ),
    security(("bearer" = [])),
)]
pub async fn delete_texture(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (uuid, texture_type_raw) = path.into_inner();
    tracing::debug!(
        profile_uuid = %uuid,
        texture_type_raw = %texture_type_raw,
        "received yggdrasil texture delete request"
    );
    if let Err(error) = crate::api::dto::validation::validate_unsigned_uuid(&uuid) {
        tracing::debug!(
            profile_uuid = %uuid,
            "yggdrasil texture delete rejected invalid profile uuid"
        );
        return yggdrasil_error_response(YggdrasilError::with_detail(
            YggdrasilErrorKind::BadRequest,
            error.message.unwrap_or_default(),
        ));
    }
    let texture_type = match texture_service::parse_texture_type(&texture_type_raw) {
        Ok(texture_type) => texture_type,
        Err(error) => return texture_error_response(error),
    };
    // See upload_texture above: authlib-injector expects 401 here for missing
    // Authorization or invalid accessToken.
    let Some(access_token) = crate::api::request_auth::bearer_token(&req) else {
        tracing::debug!(
            profile_uuid = %uuid,
            texture_type = ?texture_type,
            "yggdrasil texture delete rejected missing bearer token"
        );
        return texture_error_response(texture_service::TextureError::new(
            texture_service::TextureErrorKind::InvalidToken,
        ));
    };
    let (token, profile) =
        match texture_service::authenticate_texture_access(state.get_ref(), &access_token, &uuid)
            .await
        {
            Ok(value) => value,
            Err(error) => return texture_error_response(error),
        };
    tracing::debug!(
        user_id = token.user_id,
        token_id = token.id,
        profile_id = profile.id,
        texture_type = ?texture_type,
        "yggdrasil texture delete authenticated"
    );
    let deleted =
        match texture_service::delete_texture(state.get_ref(), &profile, texture_type).await {
            Ok(deleted) => deleted,
            Err(error) => return texture_error_response(error),
        };

    let ctx = audit_service::AuditContext::from_request(&req, token.user_id);
    audit_service::log_with_details(
        state.get_ref(),
        &ctx,
        audit_service::AuditAction::MinecraftTextureDelete,
        audit_service::AuditEntityType::MinecraftTexture,
        deleted.as_ref().map(|texture| texture.binding.id),
        Some(&profile.name),
        || {
            audit_service::details(audit_service::MinecraftTextureAuditDetails {
                profile_uuid: &profile.uuid,
                profile_name: &profile.name,
                texture_type,
                texture_hash: deleted
                    .as_ref()
                    .map(|texture| texture.texture.hash.as_str()),
                texture_model: deleted
                    .as_ref()
                    .map(|texture| texture.texture.texture_model),
                width: deleted.as_ref().map(|texture| texture.texture.width),
                height: deleted.as_ref().map(|texture| texture.texture.height),
                file_size: deleted.as_ref().map(|texture| texture.texture.file_size),
                library_status: None,
                review_note: None,
            })
        },
    )
    .await;

    tracing::debug!(
        user_id = token.user_id,
        profile_id = profile.id,
        texture_type = ?texture_type,
        deleted = deleted.is_some(),
        profile_texture_id = deleted.as_ref().map(|texture| texture.binding.id),
        "yggdrasil texture delete completed"
    );
    HttpResponse::NoContent().finish()
}

#[api_docs_macros::path(
    get,
    path = "/api/yggdrasil/textures/{hash}",
    tag = "yggdrasil",
    operation_id = "yggdrasil_texture_by_hash",
    params(("hash" = String, Path, description = "Texture SHA-256 hash")),
    responses(
        (status = 200, description = "Texture PNG bytes", content_type = "image/png"),
        (status = 404, description = "Texture not found"),
    ),
)]
pub async fn texture_by_hash(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let hash = path.into_inner();
    tracing::debug!(hash, "received yggdrasil texture by hash request");
    let etag = weak_etag_for_sha256_hash(&hash);
    let Some(texture) = (match texture_service::texture_blob_by_hash(state.get_ref(), &hash).await {
        Ok(texture) => texture,
        Err(error) => return error.error_response(),
    }) else {
        if let Some(default_skin) = texture_service::embedded_default_skin_by_hash(&hash) {
            if request_etag_matches(&req, &etag) {
                return not_modified_response(etag, texture_service::TEXTURE_CACHE_CONTROL);
            }
            tracing::debug!(
                hash,
                size = default_skin.bytes.len(),
                "serving embedded default yggdrasil skin"
            );
            return HttpResponse::Ok()
                .content_type("image/png")
                .insert_header((
                    actix_web::http::header::CACHE_CONTROL,
                    texture_service::TEXTURE_CACHE_CONTROL,
                ))
                .insert_header((actix_web::http::header::ETAG, etag))
                .insert_header((
                    actix_web::http::header::CONTENT_LENGTH,
                    default_skin.bytes.len().to_string(),
                ))
                .body(default_skin.bytes);
        }
        tracing::debug!(hash, "yggdrasil texture by hash not found");
        return HttpResponse::NotFound().finish();
    };
    if request_etag_matches(&req, &etag) {
        return not_modified_response(etag, texture_service::TEXTURE_CACHE_CONTROL);
    }
    let download = match texture_service::download_texture_blob(state.get_ref(), &texture).await {
        Ok(download) => download,
        Err(error) => return error.error_response(),
    };
    tracing::debug!(
        hash,
        content_type = %download.content_type,
        size = download.size,
        "yggdrasil texture by hash opened download"
    );
    HttpResponse::Ok()
        .content_type(download.content_type)
        .insert_header((
            actix_web::http::header::CACHE_CONTROL,
            download.cache_control,
        ))
        .insert_header((actix_web::http::header::ETAG, etag))
        .insert_header((
            actix_web::http::header::CONTENT_LENGTH,
            download.size.to_string(),
        ))
        .streaming(download.stream)
}

#[api_docs_macros::path(
    get,
    path = "/api/yggdrasil/sessionserver/session/minecraft/forwardedTextures/{upstream_id}/{texture_hash}/{ticket}",
    tag = "yggdrasil",
    operation_id = "yggdrasil_forwarded_texture_by_hash",
    params(
        ("upstream_id" = i64, Path, description = "Forwarding server ID"),
        ("texture_hash" = String, Path, description = "Forwarded texture hash"),
        ("ticket" = String, Path, description = "Signed forwarded texture ticket"),
    ),
    responses(
        (status = 200, description = "Forwarded texture PNG bytes", content_type = "image/png"),
        (status = 404, description = "Forwarded texture ticket invalid or upstream texture missing"),
        (status = 502, description = "Forwarded texture upstream failed"),
    ),
)]
pub async fn forwarded_texture_by_hash(
    state: web::Data<AppState>,
    path: web::Path<(i64, String, String)>,
) -> HttpResponse {
    let (upstream_id, texture_hash, ticket) = path.into_inner();
    tracing::debug!(
        upstream_id,
        texture_hash = %texture_hash,
        "received forwarded yggdrasil texture request by hash"
    );
    let Some(url) = (match yggdrasil_service::forwarded_texture_url(
        state.get_ref(),
        upstream_id,
        &texture_hash,
        &ticket,
    )
    .await
    {
        Ok(url) => url,
        Err(error) => return super::yggdrasil_error_response(error),
    }) else {
        tracing::debug!(
            upstream_id,
            texture_hash = %texture_hash,
            "forwarded yggdrasil texture hash mapping was not found or expired"
        );
        return HttpResponse::NotFound().finish();
    };
    tracing::debug!(
        upstream_id,
        texture_hash = %texture_hash,
        upstream_origin = reqwest::Url::parse(&url)
            .ok()
            .map(|url| url.origin().ascii_serialization())
            .as_deref()
            .unwrap_or("invalid"),
        "forwarded yggdrasil texture hash mapping resolved"
    );

    proxy_forwarded_texture_url(state.get_ref(), &url, &texture_hash).await
}

async fn proxy_forwarded_texture_url(
    state: &AppState,
    url: &str,
    texture_ref: &str,
) -> HttpResponse {
    let response = match state
        .yggdrasil_session_forward_http_client()
        .get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(error = %error, texture_ref, "forwarded yggdrasil texture request failed");
            return HttpResponse::BadGateway().finish();
        }
    };
    if response.status().as_u16() == 404 {
        return HttpResponse::NotFound().finish();
    }
    if !response.status().is_success() {
        tracing::warn!(
            status = %response.status(),
            texture_ref,
            "forwarded yggdrasil texture upstream returned failure"
        );
        return HttpResponse::BadGateway().finish();
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !content_type.starts_with("image/png") {
        tracing::warn!(
            content_type = %content_type,
            texture_ref,
            "forwarded yggdrasil texture upstream returned non-PNG content"
        );
        return HttpResponse::BadGateway().finish();
    }
    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(error = %error, texture_ref, "failed to read forwarded yggdrasil texture body");
            return HttpResponse::BadGateway().finish();
        }
    };
    HttpResponse::Ok()
        .content_type("image/png")
        .insert_header((
            actix_web::http::header::CACHE_CONTROL,
            texture_service::TEXTURE_CACHE_CONTROL,
        ))
        .insert_header((
            actix_web::http::header::CONTENT_LENGTH,
            bytes.len().to_string(),
        ))
        .body(bytes)
}

struct ReceivedTextureUpload {
    texture_model: crate::types::MinecraftTextureModel,
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
    let mut texture_model = crate::types::MinecraftTextureModel::Default;
    let mut file_path = None;
    let mut field_count = 0_u64;
    tracing::debug!(
        texture_type = ?texture_type,
        max_texture_upload_bytes = policy.max_texture_upload_bytes,
        "receiving yggdrasil texture upload multipart payload"
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
                tracing::debug!(
                    field_count,
                    "reading yggdrasil texture model multipart field"
                );
                let text = read_small_text_field(&mut field).await?;
                texture_model = texture_service::parse_skin_model(Some(&text))?;
                tracing::debug!(
                    field_count,
                    texture_model = ?texture_model,
                    "parsed yggdrasil texture model multipart field"
                );
            }
            Some("file") => {
                if !is_png_field(&field) {
                    tracing::debug!(
                        field_count,
                        "yggdrasil texture upload rejected non-png file field"
                    );
                    return Err(texture_service::TextureError::new(
                        texture_service::TextureErrorKind::InvalidContentType,
                    ));
                }
                let path = texture_upload_temp_path(state);
                tracing::debug!(
                    field_count,
                    "writing yggdrasil texture file multipart field"
                );
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
                    "draining ignored yggdrasil texture multipart field"
                );
                drain_field(&mut field).await?
            }
        }
    }

    if texture_type == MinecraftTextureType::Cape {
        texture_model = crate::types::MinecraftTextureModel::Default;
    }

    let Some(file_path) = file_path else {
        tracing::debug!(
            field_count,
            "yggdrasil texture upload rejected because file field is missing"
        );
        return Err(texture_service::TextureError::new(
            texture_service::TextureErrorKind::MissingFile,
        ));
    };
    tracing::debug!(
        field_count,
        texture_type = ?texture_type,
        texture_model = ?texture_model,
        "received yggdrasil texture upload multipart payload"
    );
    Ok(ReceivedTextureUpload {
        texture_model,
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
        "read yggdrasil texture text multipart field"
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
    tracing::debug!(chunk_count, "drained yggdrasil texture multipart field");
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
            "texture-upload-{}.png",
            uuid::Uuid::new_v4().simple()
        ))
}

fn is_png_field(field: &actix_multipart::Field) -> bool {
    field
        .content_type()
        .map(|mime| mime.type_().as_str() == "image" && mime.subtype().as_str() == "png")
        .unwrap_or(false)
}

fn texture_error_response(error: texture_service::TextureError) -> HttpResponse {
    let status = error.status_code();
    if status.is_server_error() {
        tracing::error!(
            kind = ?error.kind(),
            status = %status,
            "texture request failed"
        );
    } else {
        tracing::warn!(
            kind = ?error.kind(),
            status = %status,
            "texture request failed"
        );
    }
    HttpResponse::build(status).json(crate::api::dto::yggdrasil::YggdrasilErrorBody {
        error: error.protocol_error_name(),
        error_message: error.protocol_message(),
        cause: None,
    })
}
