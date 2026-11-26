//! Audit log entity.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::types::audit::AuditAction;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[cfg_attr(all(debug_assertions, feature = "openapi"), schema(as = AuditLogModel))]
#[sea_orm(table_name = "audit_logs")]
/// Persisted audit trail entry for security-relevant actions.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Acting user id; `0` is the reserved system actor for background and startup events.
    pub user_id: i64,
    /// What happened, from the fixed product audit action set.
    pub action: AuditAction,
    /// Kind of object the action targeted (for example `user` or `minecraft_texture`).
    pub entity_type: String,
    /// Numeric id of the targeted object, when it has one.
    pub entity_id: Option<i64>,
    /// Human-readable name of the targeted object captured at action time.
    pub entity_name: Option<String>,
    /// Action-specific detail payload stored as serialized text.
    pub details: Option<String>,
    /// Client IP (truncated to 45 bytes) when one could be resolved.
    pub ip_address: Option<String>,
    /// Client User-Agent header (truncated to 512 bytes) when present.
    pub user_agent: Option<String>,
    /// When the action was recorded.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No foreign-key relations; audit rows are standalone.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Converts a shared Forge audit-log row, rejecting unknown action wire values.
    ///
    /// # Errors
    /// Returns a descriptive message when the stored action is not part of the
    /// product audit action set.
    pub fn try_from_forge_row(value: aster_forge_db::audit_log::Model) -> Result<Self, String> {
        let action = AuditAction::from_str_name(&value.action).ok_or_else(|| {
            format!(
                "unsupported audit action in audit log row {}: {}",
                value.id, value.action
            )
        })?;

        Ok(Self {
            id: value.id,
            user_id: value.user_id,
            action,
            entity_type: value.entity_type,
            entity_id: value.entity_id,
            entity_name: value.entity_name,
            details: value.details,
            ip_address: value.ip_address,
            user_agent: value.user_agent,
            created_at: value.created_at,
        })
    }
}
