//! 统一 API 响应封装。
//!
//! Serialization and OpenAPI composition are delegated to the shared Forge
//! envelope; this module only injects the product error code and retryability
//! policy. The wire shape is `{"code","msg","data"?,"error"?}` where `error`
//! carries `{retryable}` (AsterDrive-style; the redundant per-error code that
//! older clients saw was dropped when the envelope moved to Forge).

use crate::api::error_code::AsterErrorCode;
use actix_web::HttpResponse;
use aster_forge_api::response::{
    ApiErrorInfo as ForgeApiErrorInfo, ApiResponse as ForgeApiResponse, ApiResponseCode,
    ResponseEnvelopeError,
};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

impl ApiResponseCode for AsterErrorCode {
    fn is_success(&self) -> bool {
        *self == AsterErrorCode::Success
    }
}

/// Error metadata block of the API envelope; carries the retryability flag
/// and no per-error diagnostic payload.
pub type ApiErrorInfo = ForgeApiErrorInfo<()>;

/// AsterYggdrasil's REST adapter binds the product error code to Forge's
/// validated envelope.
#[derive(Serialize)]
#[serde(transparent)]
pub struct ApiResponse<T: Serialize>(ForgeApiResponse<T, AsterErrorCode>);

#[cfg(all(debug_assertions, feature = "openapi"))]
impl<T: Serialize + ToSchema> ToSchema for ApiResponse<T> {
    fn name() -> std::borrow::Cow<'static, str> {
        // Utoipa appends this product adapter's generic arguments itself.
        std::borrow::Cow::Borrowed("ApiResponse")
    }

    fn schemas(
        schemas: &mut Vec<(
            String,
            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
        )>,
    ) {
        <ForgeApiResponse<T, AsterErrorCode> as ToSchema>::schemas(schemas);
    }
}

#[cfg(all(debug_assertions, feature = "openapi"))]
impl<T: Serialize + ToSchema> utoipa::__dev::ComposeSchema for ApiResponse<T> {
    fn compose(
        generics: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
    ) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        <ForgeApiResponse<T, AsterErrorCode> as utoipa::__dev::ComposeSchema>::compose(generics)
    }
}

#[expect(
    clippy::expect_used,
    reason = "Yggdrasil supplies a fixed success code or an AsterError-mapped failure code; a classification mismatch is a product programming error"
)]
fn adapt_envelope<T: Serialize>(
    response: Result<ForgeApiResponse<T, AsterErrorCode>, ResponseEnvelopeError>,
) -> ApiResponse<T> {
    ApiResponse(response.expect("Yggdrasil response code must match the envelope state"))
}

/// Injects the success code while keeping the wire envelope owned by Forge.
pub fn success<T: Serialize>(data: T) -> ApiResponse<T> {
    adapt_envelope(ForgeApiResponse::ok(AsterErrorCode::Success, data))
}

/// Builds the successful response whose `data` field is omitted.
pub fn empty_success() -> ApiResponse<()> {
    adapt_envelope(ForgeApiResponse::ok_empty(AsterErrorCode::Success))
}

/// Builds a failure with `retryable: false` and no error metadata, matching an
/// ordinary AsterYggdrasil error.
pub fn failure(code: AsterErrorCode, message: impl Into<String>) -> ApiResponse<()> {
    adapt_envelope(ForgeApiResponse::error(code, message))
}

/// Builds a failure with the product-owned retryability policy.
pub fn failure_with_retryability(
    code: AsterErrorCode,
    message: impl Into<String>,
    retryable: Option<bool>,
) -> ApiResponse<()> {
    let error = retryable.map(|retryable| ForgeApiErrorInfo {
        retryable,
        diagnostic: None,
    });
    adapt_envelope(ForgeApiResponse::error_with_details(code, message, error))
}

/// Liveness probe response.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct HealthResponse {
    /// Static liveness marker reported by the process.
    pub status: String,
}

impl<T: Serialize> ApiResponse<T> {
    /// Builds a success envelope wrapping `data`.
    pub fn ok(data: T) -> Self {
        success(data)
    }

    /// Serializes the envelope as an HTTP 200 JSON response.
    pub fn into_response(self) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
}

impl ApiResponse<()> {
    /// Builds a success envelope without a `data` field.
    pub fn ok_empty() -> Self {
        empty_success()
    }

    /// Builds a failure envelope with `retryable: false`.
    pub fn error(code: AsterErrorCode, message: impl Into<String>) -> Self {
        failure(code, message)
    }

    /// Builds a failure envelope with an explicit retryability flag.
    pub fn error_with_details(
        code: AsterErrorCode,
        message: impl Into<String>,
        retryable: Option<bool>,
    ) -> Self {
        failure_with_retryability(code, message, retryable)
    }

    /// Builds a failure envelope directly from raw fields, for callers that
    /// serialize the body themselves instead of returning a handler error.
    pub fn error_body(
        code: AsterErrorCode,
        message: impl Into<String>,
        retryable: Option<bool>,
    ) -> Self {
        failure_with_retryability(code, message, retryable)
    }
}

#[cfg(test)]
mod tests {
    use super::ApiResponse;
    use crate::api::error_code::AsterErrorCode;
    use actix_web::body::to_bytes;

    #[test]
    fn ok_response_serializes_data_without_error() {
        let value = serde_json::to_value(ApiResponse::ok(serde_json::json!({
            "ready": true
        })))
        .unwrap();

        assert_eq!(value["code"], AsterErrorCode::Success.as_str());
        assert_eq!(value["msg"], "");
        assert_eq!(value["data"]["ready"], true);
        assert!(value.get("error").is_none());
    }

    #[test]
    fn ok_empty_omits_data_and_error_fields() {
        let value = serde_json::to_value(ApiResponse::ok_empty()).unwrap();

        assert_eq!(value["code"], AsterErrorCode::Success.as_str());
        assert!(value.get("data").is_none());
        assert!(value.get("error").is_none());
    }

    #[test]
    fn error_helpers_keep_stable_public_code_and_retryability() {
        let plain = serde_json::to_value(ApiResponse::<()>::error(
            AsterErrorCode::BadRequest,
            "bad input",
        ))
        .unwrap();
        assert_eq!(plain["code"], AsterErrorCode::BadRequest.as_str());
        assert_eq!(plain["msg"], "bad input");
        assert!(plain.get("data").is_none());
        assert_eq!(plain["error"]["retryable"], false);

        let detailed = serde_json::to_value(ApiResponse::<()>::error_with_details(
            AsterErrorCode::DatabaseError,
            "database unavailable",
            Some(true),
        ))
        .unwrap();
        assert_eq!(detailed["code"], AsterErrorCode::DatabaseError.as_str());
        assert_eq!(detailed["error"]["retryable"], true);
        assert!(detailed["error"].get("code").is_none());

        let body = serde_json::to_value(ApiResponse::<()>::error_body(
            AsterErrorCode::RateLimited,
            "retry later",
            Some(true),
        ))
        .unwrap();
        assert_eq!(body["code"], AsterErrorCode::RateLimited.as_str());
        assert_eq!(body["error"]["retryable"], true);
    }

    #[actix_web::test]
    async fn into_response_uses_http_ok_with_json_envelope() {
        let response = ApiResponse::ok(serde_json::json!({ "value": 42 })).into_response();

        assert_eq!(response.status(), 200);
        let body = to_bytes(response.into_body()).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], AsterErrorCode::Success.as_str());
        assert_eq!(value["data"]["value"], 42);
    }

    /// Byte-level wire contract after the Forge envelope migration. The plain
    /// error now carries `error:{"retryable":false}` (Forge-validated state);
    /// the redundant per-error `code` field is gone.
    #[test]
    fn wire_bytes_are_stable() {
        assert_eq!(
            serde_json::to_string(&ApiResponse::ok(serde_json::json!({ "ready": true }))).unwrap(),
            r#"{"code":"success","msg":"","data":{"ready":true}}"#
        );
        assert_eq!(
            serde_json::to_string(&ApiResponse::ok_empty()).unwrap(),
            r#"{"code":"success","msg":""}"#
        );
        assert_eq!(
            serde_json::to_string(&ApiResponse::<()>::error(
                AsterErrorCode::BadRequest,
                "bad input"
            ))
            .unwrap(),
            r#"{"code":"bad_request","msg":"bad input","error":{"retryable":false}}"#
        );
        assert_eq!(
            serde_json::to_string(&ApiResponse::<()>::error_with_details(
                AsterErrorCode::DatabaseError,
                "database unavailable",
                Some(true),
            ))
            .unwrap(),
            r#"{"code":"database.error","msg":"database unavailable","error":{"retryable":true}}"#
        );
        assert_eq!(
            serde_json::to_string(&ApiResponse::<()>::error_body(
                AsterErrorCode::RateLimited,
                "retry later",
                None,
            ))
            .unwrap(),
            r#"{"code":"rate_limited","msg":"retry later"}"#
        );
    }
}
