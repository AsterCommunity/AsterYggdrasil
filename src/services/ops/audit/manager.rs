use chrono::Utc;
use sea_orm::DatabaseConnection;

use super::context::AuditContext;
use crate::config::RuntimeConfig;
use crate::runtime::{AuditRuntimeState, RuntimeConfigRuntimeState, SharedRuntimeState};
use aster_yggdrasil_model::types::audit::{AuditAction, AuditEntityType};

/// Checks the runtime configuration's audit recording allowlist for one
/// action; unlisted actions are dropped before any write work happens.
pub fn should_record<S: RuntimeConfigRuntimeState>(state: &S, action: AuditAction) -> bool {
    state.runtime_config().should_record_audit_action(action)
}

/// [`should_record`] variant taking the runtime configuration explicitly, for
/// call sites that hold a config reference instead of full runtime state.
pub fn should_record_with_config(runtime_config: &RuntimeConfig, action: AuditAction) -> bool {
    runtime_config.should_record_audit_action(action)
}

/// Audit record skeleton for the lazy-details entry points; details JSON is
/// built only after the allowlist check passes.
#[derive(Clone, Copy)]
pub struct AuditLogInput<'a> {
    /// Actor identity and request metadata.
    pub ctx: &'a AuditContext,
    /// Audited action kind.
    pub action: AuditAction,
    /// Entity class the action targeted.
    pub entity_type: AuditEntityType,
    /// ID of the targeted entity row, when the action addresses one.
    pub entity_id: Option<i64>,
    /// Human-readable entity name captured at write time.
    pub entity_name: Option<&'a str>,
}

fn audit_log_request(
    ctx: &AuditContext,
    action: AuditAction,
    entity_type: AuditEntityType,
    entity_id: Option<i64>,
    entity_name: Option<&str>,
    details: Option<serde_json::Value>,
) -> aster_forge_db::AuditLogCreate {
    aster_forge_db::AuditLogCreate {
        user_id: ctx.user_id,
        action: action.as_str().to_string(),
        entity_type: entity_type.as_str().to_string(),
        entity_id,
        entity_name: entity_name.map(ToOwned::to_owned),
        details: details.map(|value| value.to_string()),
        ip_address: ctx.ip_address.clone(),
        user_agent: ctx.user_agent.clone(),
        created_at: Utc::now(),
    }
}

async fn record_prechecked<S: AuditRuntimeState>(
    state: &S,
    ctx: &AuditContext,
    action: AuditAction,
    entity_type: AuditEntityType,
    entity_id: Option<i64>,
    entity_name: Option<&str>,
    details: Option<serde_json::Value>,
) {
    let request = audit_log_request(ctx, action, entity_type, entity_id, entity_name, details);

    // Admission is best-effort: overflow records fall back to direct writes inside
    // the writer, and the shutdown drain reconciles whatever remains queued.
    if let Err(error) = state.audit_writer().record(request).await {
        tracing::warn!(
            ?error,
            action = action.as_str(),
            "audit record admission failed"
        );
    }
}

async fn record_prechecked_with_db(
    db: &DatabaseConnection,
    ctx: &AuditContext,
    action: AuditAction,
    entity_type: AuditEntityType,
    entity_id: Option<i64>,
    entity_name: Option<&str>,
    details: Option<serde_json::Value>,
) {
    let request = audit_log_request(ctx, action, entity_type, entity_id, entity_name, details);
    if let Err(error) = aster_forge_db::create_audit_log_row(db, request).await {
        tracing::warn!(
            ?error,
            action = action.as_str(),
            "direct audit write failed"
        );
    }
}

/// Records one audit entry through the buffered writer after the allowlist
/// check; details JSON is passed pre-built.
pub async fn log<S>(
    state: &S,
    ctx: &AuditContext,
    action: AuditAction,
    entity_type: AuditEntityType,
    entity_id: Option<i64>,
    entity_name: Option<&str>,
    details: Option<serde_json::Value>,
) where
    S: SharedRuntimeState,
{
    if !should_record(state, action) {
        return;
    }

    record_prechecked(
        state,
        ctx,
        action,
        entity_type,
        entity_id,
        entity_name,
        details,
    )
    .await;
}

/// Writes one audit entry directly to the given database connection, skipping
/// the buffered writer. Intended for in-transaction paths that must not lose
/// the record when the writer queue is unavailable; details JSON is built
/// lazily only after the allowlist check passes.
pub async fn log_with_db_and_config<F>(
    db: &DatabaseConnection,
    runtime_config: &RuntimeConfig,
    input: AuditLogInput<'_>,
    details: F,
) where
    F: FnOnce() -> Option<serde_json::Value>,
{
    if !should_record_with_config(runtime_config, input.action) {
        return;
    }

    let details = details();
    record_prechecked_with_db(
        db,
        input.ctx,
        input.action,
        input.entity_type,
        input.entity_id,
        input.entity_name,
        details,
    )
    .await;
}

/// [`log`] variant building the details JSON lazily, so serialization cost is
/// only paid for actions the allowlist admits.
pub async fn log_with_details<S, F>(
    state: &S,
    ctx: &AuditContext,
    action: AuditAction,
    entity_type: AuditEntityType,
    entity_id: Option<i64>,
    entity_name: Option<&str>,
    details: F,
) where
    S: SharedRuntimeState,
    F: FnOnce() -> Option<serde_json::Value>,
{
    if !should_record(state, action) {
        return;
    }

    record_prechecked(
        state,
        ctx,
        action,
        entity_type,
        entity_id,
        entity_name,
        details(),
    )
    .await;
}
