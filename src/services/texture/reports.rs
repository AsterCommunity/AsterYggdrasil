//! Texture-library report intake and admin handling.

use super::moderation::{
    ensure_texture_library_enabled, normalize_admin_report_note, pending_admin_texture_report,
    texture_report_info,
};
use crate::db::repository::minecraft_texture_report_repo;
use crate::errors::Result;
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use aster_yggdrasil_model::types::yggdrasil::MinecraftTextureReportStatus;

use super::*;

/// Marks a pending public-library texture report as rejected on behalf of the
/// given admin user, returning the handled report. Fails if the texture
/// library feature is disabled or the report is not pending.
pub async fn reject_texture_library_report<S>(
    state: &S,
    handler_user_id: i64,
    report_id: i64,
    admin_note: Option<String>,
) -> Result<TextureReportInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    ensure_texture_library_enabled(state)?;
    let report = pending_admin_texture_report(state, report_id).await?;
    let report = minecraft_texture_report_repo::handle(
        state.writer_db(),
        report,
        minecraft_texture_report_repo::HandleTextureReport {
            status: MinecraftTextureReportStatus::Rejected,
            admin_note: normalize_admin_report_note(admin_note)?,
            handled_by_user_id: handler_user_id,
            handled_at: chrono::Utc::now(),
        },
    )
    .await?;
    texture_report_info(state, &report).await
}
