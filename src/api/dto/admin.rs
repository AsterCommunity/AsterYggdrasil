//! Admin API DTOs.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::api::pagination::{AdminTaskSortBy, AdminUserSortBy, SortOrder};
use crate::services::config_service::{ConfigActionType, SystemConfigValue};
use crate::services::external_auth_service::{
    CreateExternalAuthProviderInput, ExternalAuthProviderTestParamsInput,
    UpdateExternalAuthProviderInput,
};
use crate::types::{
    BackgroundTaskKind, BackgroundTaskStatus, ExternalAuthKind, ExternalAuthProviderOptions,
    NullablePatch, OperatorScope, SystemConfigVisibility, UserRole, UserStatus,
};

#[derive(Debug, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct SetConfigReq {
    pub value: SystemConfigValue,
    pub visibility: Option<SystemConfigVisibility>,
}

#[derive(Debug, Deserialize, Validate)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExecuteConfigActionReq {
    pub action: ConfigActionType,
    pub values: Option<BTreeMap<String, SystemConfigValue>>,
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
    pub sort_by: Option<AdminTaskSortBy>,
    pub sort_order: Option<SortOrder>,
}

impl AdminTaskListQuery {
    pub fn sort_by(&self) -> AdminTaskSortBy {
        self.sort_by.unwrap_or_default()
    }

    pub fn sort_order(&self) -> SortOrder {
        self.sort_order.unwrap_or(SortOrder::Desc)
    }
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
    pub sort_by: Option<AdminUserSortBy>,
    pub sort_order: Option<SortOrder>,
}

impl AdminUserListQuery {
    pub fn sort_by(&self) -> AdminUserSortBy {
        self.sort_by.unwrap_or_default()
    }

    pub fn sort_order(&self) -> SortOrder {
        self.sort_order.unwrap_or(SortOrder::Desc)
    }
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
