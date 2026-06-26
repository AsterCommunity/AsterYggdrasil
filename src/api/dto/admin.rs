//! Admin API DTOs.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::services::config_service::ConfigActionType;
use crate::services::external_auth_service::{
    CreateExternalAuthProviderInput, ExternalAuthProviderTestParamsInput,
    UpdateExternalAuthProviderInput,
};
use crate::services::yggdrasil_session_forward_service::{
    CreateYggdrasilSessionForwardServerInput, UpdateYggdrasilSessionForwardServerInput,
};
use crate::types::{
    external_auth::ExternalAuthKind, external_auth::ExternalAuthProviderOptions,
    task::BackgroundTaskKind, task::BackgroundTaskStatus, user::OperatorScope, user::UserBanScope,
    user::UserBanStatus, user::UserRole, user::UserStatus,
    yggdrasil::YggdrasilSessionForwardEndpointKind, yggdrasil::YggdrasilSessionForwardServerSortBy,
};
use aster_forge_api::NullablePatch;
use aster_forge_config::ConfigValue;
use aster_forge_config::ConfigVisibility;

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct SetConfigReq {
    pub value: ConfigValue,
    pub visibility: Option<ConfigVisibility>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExecuteConfigActionReq {
    pub action: ConfigActionType,
    pub values: Option<BTreeMap<String, ConfigValue>>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExecuteConfigActionResp {
    pub message: String,
    pub value: Option<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RemovedCountResponse {
    pub removed: u64,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminTaskListQuery {
    pub kind: Option<BackgroundTaskKind>,
    pub status: Option<BackgroundTaskStatus>,
    pub after_updated_at: Option<DateTime<Utc>>,
    pub after_id: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AdminTaskCleanupReq {
    pub finished_before: DateTime<Utc>,
    pub kind: Option<BackgroundTaskKind>,
    pub status: Option<BackgroundTaskStatus>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminUserListQuery {
    #[validate(length(max = 96, message = "keyword must not exceed 96 characters"))]
    pub keyword: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
    pub after_created_at: Option<DateTime<Utc>>,
    pub after_id: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminUserBanListQuery {
    pub user_id: Option<i64>,
    pub scope: Option<UserBanScope>,
    pub status: Option<UserBanStatus>,
    #[serde(default)]
    pub effective_only: bool,
    pub after_created_at: Option<DateTime<Utc>>,
    pub after_id: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateUserBanReq {
    pub scopes: Vec<UserBanScope>,
    #[validate(length(min = 1, max = 128, message = "reason must be 1-128 characters"))]
    pub reason: String,
    #[validate(length(max = 1000, message = "public_reason must not exceed 1000 characters"))]
    pub public_reason: Option<String>,
    #[validate(length(max = 1000, message = "admin_note must not exceed 1000 characters"))]
    pub admin_note: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateUserBanReq {
    pub scopes: Option<Vec<UserBanScope>>,
    #[validate(length(min = 1, max = 128, message = "reason must be 1-128 characters"))]
    pub reason: Option<String>,
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub public_reason: Option<NullablePatch<String>>,
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub admin_note: Option<NullablePatch<String>>,
    pub starts_at: Option<DateTime<Utc>>,
    #[serde(
        default,
        deserialize_with = "aster_forge_api::deserialize_nullable_patch_option"
    )]
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub expires_at: Option<NullablePatch<DateTime<Utc>>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct RevokeUserBanReq {
    #[validate(length(max = 1000, message = "revoke_note must not exceed 1000 characters"))]
    pub revoke_note: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateAdminUserReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_auth_username"))]
    pub username: String,
    #[validate(email(message = "email must be a valid email address"))]
    pub email: String,
    pub password: Option<String>,
    pub must_change_password: Option<bool>,
    pub role: Option<UserRole>,
    pub operator_scopes: Option<Vec<OperatorScope>>,
    pub status: Option<UserStatus>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateUserInvitationReq {
    #[validate(email(message = "email must be a valid email address"))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateAdminUserReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_auth_username"))]
    pub username: Option<String>,
    #[validate(email(message = "email must be a valid email address"))]
    pub email: Option<String>,
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_auth_password"))]
    pub password: Option<String>,
    pub role: Option<UserRole>,
    pub operator_scopes: Option<Vec<OperatorScope>>,
    pub status: Option<UserStatus>,
    pub must_change_password: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminMinecraftProfileListQuery {
    pub user_id: Option<i64>,
    #[validate(custom(
        function = "crate::api::dto::validation::validate_optional_minecraft_profile_name"
    ))]
    pub name: Option<String>,
    #[validate(custom(function = "crate::api::dto::validation::validate_optional_unsigned_uuid"))]
    pub uuid: Option<String>,
    #[validate(length(max = 64, message = "query must not exceed 64 characters"))]
    pub query: Option<String>,
    pub after_id: Option<i64>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateExternalAuthProviderReq {
    pub provider_kind: ExternalAuthKind,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: String,
    pub icon_url: Option<String>,
    pub options: Option<ExternalAuthProviderOptions>,
    pub issuer_url: Option<String>,
    pub authorization_url: Option<String>,
    pub authorize_url: Option<String>,
    pub token_url: Option<String>,
    pub userinfo_url: Option<String>,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 255, message = "client_id must not exceed 255 bytes"))]
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Option<String>,
    pub enabled: Option<bool>,
    pub auto_provision_enabled: Option<bool>,
    pub auto_link_verified_email_enabled: Option<bool>,
    pub require_email_verified: Option<bool>,
    pub subject_claim: Option<String>,
    pub username_claim: Option<String>,
    pub display_name_claim: Option<String>,
    pub email_claim: Option<String>,
    pub email_verified_claim: Option<String>,
    pub groups_claim: Option<String>,
    pub avatar_url_claim: Option<String>,
    pub allowed_domains: Option<Vec<String>>,
}

impl From<CreateExternalAuthProviderReq> for CreateExternalAuthProviderInput {
    fn from(value: CreateExternalAuthProviderReq) -> Self {
        Self {
            provider_kind: value.provider_kind,
            display_name: value.display_name,
            icon_url: value.icon_url,
            options: value.options,
            issuer_url: value.issuer_url,
            authorization_url: value.authorization_url.or(value.authorize_url),
            token_url: value.token_url,
            userinfo_url: value.userinfo_url,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_enabled: value.auto_provision_enabled,
            auto_link_verified_email_enabled: value.auto_link_verified_email_enabled,
            require_email_verified: value.require_email_verified,
            subject_claim: value.subject_claim,
            username_claim: value.username_claim,
            display_name_claim: value.display_name_claim,
            email_claim: value.email_claim,
            email_verified_claim: value.email_verified_claim,
            groups_claim: value.groups_claim,
            avatar_url_claim: value.avatar_url_claim,
            allowed_domains: value.allowed_domains,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateExternalAuthProviderReq {
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub icon_url: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub issuer_url: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub authorization_url: Option<NullablePatch<String>>,
    pub authorize_url: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub token_url: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub userinfo_url: Option<NullablePatch<String>>,
    pub options: Option<ExternalAuthProviderOptions>,
    #[validate(length(max = 255, message = "client_id must not exceed 255 bytes"))]
    pub client_id: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub client_secret: Option<NullablePatch<String>>,
    pub scopes: Option<String>,
    pub enabled: Option<bool>,
    pub auto_provision_enabled: Option<bool>,
    pub auto_link_verified_email_enabled: Option<bool>,
    pub require_email_verified: Option<bool>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub subject_claim: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub username_claim: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub display_name_claim: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_claim: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub email_verified_claim: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub groups_claim: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub avatar_url_claim: Option<NullablePatch<String>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<Vec<String>>))]
    pub allowed_domains: Option<NullablePatch<Vec<String>>>,
}

impl From<UpdateExternalAuthProviderReq> for UpdateExternalAuthProviderInput {
    fn from(value: UpdateExternalAuthProviderReq) -> Self {
        Self {
            display_name: value.display_name,
            icon_url: value.icon_url,
            issuer_url: value.issuer_url,
            authorization_url: value
                .authorization_url
                .or_else(|| value.authorize_url.map(NullablePatch::Value)),
            token_url: value.token_url,
            userinfo_url: value.userinfo_url,
            options: value.options,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_enabled: value.auto_provision_enabled,
            auto_link_verified_email_enabled: value.auto_link_verified_email_enabled,
            require_email_verified: value.require_email_verified,
            subject_claim: value.subject_claim,
            username_claim: value.username_claim,
            display_name_claim: value.display_name_claim,
            email_claim: value.email_claim,
            email_verified_claim: value.email_verified_claim,
            groups_claim: value.groups_claim,
            avatar_url_claim: value.avatar_url_claim,
            allowed_domains: value.allowed_domains,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExternalAuthProviderTestParamsReq {
    pub kind: Option<ExternalAuthKind>,
    pub provider_kind: Option<ExternalAuthKind>,
    pub options: Option<ExternalAuthProviderOptions>,
    pub issuer_url: Option<String>,
    pub authorization_url: Option<String>,
    pub authorize_url: Option<String>,
    pub token_url: Option<String>,
    pub userinfo_url: Option<String>,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 255, message = "client_id must not exceed 255 bytes"))]
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Option<String>,
}

impl From<ExternalAuthProviderTestParamsReq> for ExternalAuthProviderTestParamsInput {
    fn from(value: ExternalAuthProviderTestParamsReq) -> Self {
        Self {
            provider_kind: value
                .provider_kind
                .or(value.kind)
                .unwrap_or(ExternalAuthKind::Oidc),
            options: value.options,
            issuer_url: value.issuer_url,
            authorization_url: value.authorization_url.or(value.authorize_url),
            token_url: value.token_url,
            userinfo_url: value.userinfo_url,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
        }
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AdminYggdrasilSessionForwardServerListQuery {
    pub limit: Option<u64>,
    pub sort_by: Option<YggdrasilSessionForwardServerSortBy>,
    pub after_id: Option<i64>,
    pub after_enabled: Option<bool>,
    pub after_priority: Option<i32>,
}

impl AdminYggdrasilSessionForwardServerListQuery {
    pub fn limit_or(&self, default: u64, max: u64) -> u64 {
        self.limit
            .map(|value| value.clamp(1, max))
            .unwrap_or(default)
    }

    pub fn sort_by(&self) -> YggdrasilSessionForwardServerSortBy {
        self.sort_by.unwrap_or_default()
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CreateYggdrasilSessionForwardServerReq {
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: String,
    #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
    pub base_url: String,
    pub endpoint_kind: Option<YggdrasilSessionForwardEndpointKind>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub weight: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub texture_forward_enabled: Option<bool>,
}

impl From<CreateYggdrasilSessionForwardServerReq> for CreateYggdrasilSessionForwardServerInput {
    fn from(value: CreateYggdrasilSessionForwardServerReq) -> Self {
        Self {
            display_name: value.display_name,
            base_url: value.base_url,
            endpoint_kind: value.endpoint_kind,
            enabled: value.enabled,
            priority: value.priority,
            weight: value.weight,
            timeout_ms: value.timeout_ms,
            texture_forward_enabled: value.texture_forward_enabled,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct UpdateYggdrasilSessionForwardServerReq {
    #[validate(length(max = 128, message = "display_name must not exceed 128 bytes"))]
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub endpoint_kind: Option<YggdrasilSessionForwardEndpointKind>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub weight: Option<i32>,
    pub timeout_ms: Option<i32>,
    pub texture_forward_enabled: Option<bool>,
}

impl From<UpdateYggdrasilSessionForwardServerReq> for UpdateYggdrasilSessionForwardServerInput {
    fn from(value: UpdateYggdrasilSessionForwardServerReq) -> Self {
        Self {
            display_name: value.display_name,
            base_url: value.base_url,
            endpoint_kind: value.endpoint_kind,
            enabled: value.enabled,
            priority: value.priority,
            weight: value.weight,
            timeout_ms: value.timeout_ms,
            texture_forward_enabled: value.texture_forward_enabled,
        }
    }
}
