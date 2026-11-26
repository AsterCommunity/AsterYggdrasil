use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use aster_yggdrasil_model::entities::user;
use aster_yggdrasil_model::types::audit::{AuditAction, AuditEntityType};
use aster_yggdrasil_model::types::user::{UserRole, UserStatus};
/// Acting user summary joined into audit entries; `None` on the wire when the
/// actor is the reserved system user or has been deleted.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuditUserSummary {
    /// User ID; 0 marks the reserved system actor.
    pub id: i64,
    /// Display name at query time.
    pub username: String,
    /// Account email at query time.
    pub email: String,
    /// Account role at query time.
    pub role: UserRole,
    /// Account status at query time.
    pub status: UserStatus,
}

/// One audit log record as served to the admin audit query API.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuditLogEntry {
    /// Audit log row ID.
    pub id: i64,
    /// Acting user ID; 0 marks the reserved system actor.
    pub user_id: i64,
    /// Joined actor summary, absent for the system actor or deleted users.
    pub user: Option<AuditUserSummary>,
    /// Audited action kind.
    pub action: AuditAction,
    /// Entity class the action targeted.
    pub entity_type: AuditEntityType,
    /// ID of the targeted entity row, when the action addresses one.
    pub entity_id: Option<i64>,
    /// Human-readable entity name captured at write time; survives entity
    /// deletion.
    pub entity_name: Option<String>,
    /// Raw JSON action payload serialized at write time.
    pub details: Option<String>,
    /// Precomputed display metadata for the admin UI, when the action has a
    /// presentation mapping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation: Option<AuditPresentation>,
    /// Client IP captured from the request, truncated to the audit limit.
    pub ip_address: Option<String>,
    /// User-Agent captured from the request, truncated to the audit limit.
    pub user_agent: Option<String>,
    /// Record creation timestamp (UTC).
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTime<Utc>,
}

/// Localizable display message: a presentation code plus the parameters the
/// client substitutes into the translated template.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuditPresentationMessage {
    /// Presentation code the client maps to a translation key.
    pub code: String,
    /// Template parameters keyed by placeholder name; omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, serde_json::Value>,
}

/// Precomputed display metadata for one audit entry, letting clients render
/// localized lines without re-parsing the raw `details` JSON.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct AuditPresentation {
    /// One-line summary of what happened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<AuditPresentationMessage>,
    /// Line describing the targeted entity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<AuditPresentationMessage>,
    /// Line rendering the structured `details` payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<AuditPresentationMessage>,
}

impl From<user::Model> for AuditUserSummary {
    fn from(model: user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
            role: model.role,
            status: model.status,
        }
    }
}
