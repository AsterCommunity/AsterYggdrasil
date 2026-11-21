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
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: i64,
    pub action: AuditAction,
    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub entity_name: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
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
