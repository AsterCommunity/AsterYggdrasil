//! Current-user account DTOs.

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::{IntoParams, ToSchema};

use crate::services::{ops::audit::AuditLogEntry, user::ban::UserBanInfo};
use aster_yggdrasil_model::types::audit::AuditEntityType;
use aster_yggdrasil_model::types::permission::PermissionId;
use aster_yggdrasil_model::types::user::UserBanStatus;

/// Account overview shown on the current user's home page.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AccountOverviewResp {
    /// Number of Minecraft profiles owned by the current user.
    pub profile_count: u64,
    /// Recent audit log entries covering the user's own activity.
    pub recent_activity: Vec<AuditLogEntry>,
}

/// Filter query for the current user's audit log list, ordered by creation
/// time with id as tie breaker.
#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AccountAuditLogFilterQuery {
    /// Optional audit action filter.
    pub action: Option<String>,
    /// Optional entity type filter.
    pub entity_type: Option<AuditEntityType>,
    /// Optional entity id filter.
    pub entity_id: Option<i64>,
    /// Optional inclusive lower time bound; RFC 3339 string.
    pub after: Option<String>,
    /// Optional inclusive upper time bound; RFC 3339 string.
    pub before: Option<String>,
    /// Cursor: creation timestamp of the last entry on the previous page.
    pub after_created_at: Option<DateTime<Utc>>,
    /// Cursor: id of the last entry, used as a stable tie breaker.
    pub after_id: Option<i64>,
}

impl AccountAuditLogFilterQuery {
    /// Converts the query into audit log filters scoped to the given user.
    pub fn into_filters_for_user(
        self,
        user_id: i64,
    ) -> crate::services::ops::audit::AuditLogFilters {
        crate::services::ops::audit::AuditLogFilters::from_query(
            &crate::services::ops::audit::AuditLogFilterQuery {
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

/// Filter query for the current user's ban list, ordered by creation time with
/// id as tie breaker.
#[derive(Debug, Deserialize)]
#[cfg_attr(
    all(debug_assertions, feature = "openapi"),
    derive(IntoParams, ToSchema)
)]
pub struct AccountUserBanListQuery {
    /// Optional ban scope filter.
    pub scope: Option<PermissionId>,
    /// Optional stored ban status filter.
    pub status: Option<UserBanStatus>,
    /// When true, only bans whose effective status is currently active are
    /// returned.
    pub effective_only: Option<bool>,
    /// Cursor: creation timestamp of the last ban on the previous page.
    pub after_created_at: Option<DateTime<Utc>>,
    /// Cursor: id of the last ban, used as a stable tie breaker.
    pub after_id: Option<i64>,
}

/// Ban record as visible to the banned user.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AccountUserBanInfo {
    /// Ban record id.
    pub id: i64,
    /// Capability scopes the user loses while this ban is effective.
    pub scopes: Vec<PermissionId>,
    /// Stored status of the ban record.
    pub status: UserBanStatus,
    /// Status after applying the time window; e.g. an active ban is expired
    /// once `expires_at` has passed.
    pub effective_status: UserBanStatus,
    /// Whether the ban is currently in force.
    pub effective: bool,
    /// Internal reason recorded by the moderator.
    pub reason: String,
    /// Optional reason shown to the banned user.
    pub public_reason: Option<String>,
    /// Ban start time.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub starts_at: DateTime<Utc>,
    /// Ban expiry time; absent for a ban that lasts until revoked.
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub expires_at: Option<DateTime<Utc>>,
    /// When the ban was revoked; absent while the ban is not revoked.
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub revoked_at: Option<DateTime<Utc>>,
    /// When the ban record was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTime<Utc>,
    /// When the ban record was last updated.
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
