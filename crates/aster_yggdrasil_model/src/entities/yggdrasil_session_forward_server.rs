//! SeaORM entity for upstream Yggdrasil session server forwarding.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::types::{
    yggdrasil::YggdrasilSessionForwardEndpointKind, yggdrasil::YggdrasilSessionForwardProviderKind,
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "yggdrasil_session_forward_servers")]
/// Upstream Yggdrasil session server that join/hasJoined checks may be forwarded to.
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// Name shown in the admin forward-server list.
    pub display_name: String,
    /// Whether the entry points at this deployment (`local`) or another server (`remote`).
    pub provider_kind: YggdrasilSessionForwardProviderKind,
    /// Upstream API dialect (`authlib_injector` or `mojang_session`).
    pub endpoint_kind: YggdrasilSessionForwardEndpointKind,
    /// Upstream API root URL; unique, and set for remote entries only.
    pub base_url: Option<String>,
    /// Marks entries seeded by the product; builtin entries cannot be deleted.
    pub builtin: bool,
    /// Whether the server participates in forwarding.
    pub enabled: bool,
    /// Position in the forwarding call order among enabled servers; ties are broken by id.
    pub priority: i32,
    /// Relative weight recorded for the server (validated, default 1).
    pub weight: i32,
    /// Upstream request timeout in milliseconds.
    pub timeout_ms: i32,
    /// Whether texture lookups may also be forwarded to this server.
    pub texture_forward_enabled: bool,
    /// When the server was last health-checked.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_checked_at: Option<DateTimeUtc>,
    /// When the last health check succeeded.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_success_at: Option<DateTimeUtc>,
    /// When the last health check failed.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub last_failure_at: Option<DateTimeUtc>,
    /// Message from the most recent failed health check.
    pub last_failure_message: Option<String>,
    /// When the entry was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the entry was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No foreign-key relations; forward-server rows stand alone.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
