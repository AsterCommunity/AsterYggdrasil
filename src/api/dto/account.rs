//! Current-user account DTOs.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};

use crate::services::{audit_service::AuditLogEntry, ban_service::UserBanInfo};
use aster_yggdrasil_model::types::audit::AuditEntityType;
use aster_yggdrasil_model::types::user::{UserBanScope, UserBanStatus};
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AccountOverviewResp {
    pub profile_count: u64,
    pub recent_activity: Vec<AuditLogEntry>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AccountAuditLogFilterQuery {
    pub action: Option<String>,
    pub entity_type: Option<AuditEntityType>,
    pub entity_id: Option<i64>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub after_created_at: Option<DateTime<Utc>>,
    pub after_id: Option<i64>,
}

impl AccountAuditLogFilterQuery {
    pub fn into_filters_for_user(
        self,
        user_id: i64,
    ) -> crate::services::audit_service::AuditLogFilters {
        crate::services::audit_service::AuditLogFilters::from_query(
            &crate::services::audit_service::AuditLogFilterQuery {
                user_id: Some(user_id),
                action: self.action,
                entity_type: self.entity_type,
                entity_id: self.entity_id,
                after: self.after,
                before: self.before,
                after_created_at: self.after_created_at,
                after_id: self.after_id,
            },
        )
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AccountUserBanListQuery {
    pub scope: Option<UserBanScope>,
    pub status: Option<UserBanStatus>,
    pub effective_only: Option<bool>,
    pub after_created_at: Option<DateTime<Utc>>,
    pub after_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AccountUserBanInfo {
    pub id: i64,
    pub scopes: Vec<UserBanScope>,
    pub status: UserBanStatus,
    pub effective_status: UserBanStatus,
    pub effective: bool,
    pub reason: String,
    pub public_reason: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub starts_at: DateTime<Utc>,
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub expires_at: Option<DateTime<Utc>>,
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub revoked_at: Option<DateTime<Utc>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTime<Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTime<Utc>,
}

impl From<UserBanInfo> for AccountUserBanInfo {
    fn from(value: UserBanInfo) -> Self {
        Self {
            id: value.id,
            scopes: value.scopes,
            status: value.status,
            effective_status: value.effective_status,
            effective: value.effective,
            reason: value.reason,
            public_reason: value.public_reason,
            starts_at: value.starts_at,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
