//! Apply mode: prepare the target schema, initialize the checkpoint, copy,
//! verify, and record the final state.

pub(in crate::cli) mod convert;
pub(in crate::cli) mod copy;

use super::checkpoint::{self, MigrationCheckpoint};
use super::model::{StageReport, VerificationReport};
use super::plan::TablePlan;
use crate::errors::{AsterError, MapAsterErr, Result};

/// Everything apply mode needs, built by execute.
pub(super) struct ApplyModeContext<'a> {
    pub source_db: &'a sea_orm::DatabaseConnection,
    pub target_db: &'a sea_orm::DatabaseConnection,
    pub source_url: &'a str,
    pub target_url: &'a str,
    pub plans: &'a [TablePlan],
}

/// Outputs of a completed apply run.
pub(super) struct ApplyExecution {
    pub stages: Vec<StageReport>,
    pub verification: VerificationReport,
    pub checkpoint: MigrationCheckpoint,
    pub resumed: bool,
}

/// Runs the full apply pipeline; every failure marks the checkpoint failed
/// (best effort) before returning.
pub(super) async fn execute_apply_mode(ctx: ApplyModeContext<'_>) -> Result<ApplyExecution> {
    let backend = ctx.target_db.get_database_backend();
    let mut stages = Vec::new();

    // 1. Target schema comes from replaying this product's own migrations.
    aster_yggdrasil_migration::Migrator::up(ctx.target_db, None)
        .await
        .map_aster_err(AsterError::database_operation)?;
    let pending_after_prepare =
        crate::cli::common::database::pending_migrations(ctx.target_db).await?;
    if !pending_after_prepare.is_empty() {
        return Err(AsterError::database_operation(format!(
            "target database still has pending migrations after prepare: {}",
            pending_after_prepare.join(", ")
        )));
    }
    stages.push(StageReport {
        name: "structure_prepare".to_string(),
        status: "ok".to_string(),
        message: format!(
            "migrations applied; pending_target=0 backend={}",
            crate::cli::common::database::backend_name(backend)
        ),
    });

    // 2. Checkpoint lifecycle.
    checkpoint::ensure_checkpoint_table(ctx.target_db).await?;
    let initialized =
        checkpoint::initialize_checkpoint(ctx.source_url, ctx.target_url, ctx.target_db, ctx.plans)
            .await?;
    let mut checkpoint = initialized.checkpoint;
    let resumed = initialized.resumed;
    if resumed {
        tracing::info!("{}", checkpoint::resume_message(&checkpoint));
    }

    // 3. Copy.
    let copy_stage = match run_copy(&ctx, &mut checkpoint).await {
        Ok(stage) => stage,
        Err(error) => {
            checkpoint::mark_checkpoint_failed_best_effort(ctx.target_db, &mut checkpoint, &error)
                .await;
            return Err(error);
        }
    };
    stages.push(copy_stage);

    // 4. Sequence maintenance.
    if let Err(error) = copy::reset_sequences(ctx.target_db, ctx.plans).await {
        checkpoint::mark_checkpoint_failed_best_effort(ctx.target_db, &mut checkpoint, &error)
            .await;
        return Err(error);
    }

    // 5. Verification.
    checkpoint.stage = "verification".to_string();
    checkpoint.status = "running".to_string();
    checkpoint.current_table = None;
    checkpoint.current_table_index = ctx.plans.len() as i64;
    checkpoint.current_table_offset = 0;
    checkpoint.heartbeat_at_ms = chrono::Utc::now().timestamp_millis();
    checkpoint.updated_at_ms = checkpoint.heartbeat_at_ms;
    checkpoint::update_checkpoint(ctx.target_db, &checkpoint).await?;

    let verification = super::verify::verify_target(ctx.target_db, ctx.plans).await?;
    let ready = verification.ready();
    stages.push(StageReport {
        name: "verification".to_string(),
        status: if ready {
            "ok".to_string()
        } else {
            "attention".to_string()
        },
        message: verification.message(),
    });

    Ok(ApplyExecution {
        stages,
        verification,
        checkpoint,
        resumed,
    })
}

/// Runs the copy loop and returns the data_copy stage report on success.
async fn run_copy(
    ctx: &ApplyModeContext<'_>,
    checkpoint: &mut MigrationCheckpoint,
) -> Result<StageReport> {
    let target_hints = super::schema::load_target_type_hints(
        ctx.target_db,
        ctx.target_db.get_database_backend(),
        ctx.plans,
    )
    .await?;
    let total_rows = ctx.plans.iter().map(|plan| plan.source_rows).sum();
    let mut progress = copy::ProgressReporter::new(
        total_rows,
        "database-migrate".to_string(),
        checkpoint.copied_rows,
    );
    copy::copy_tables_with_resume(
        ctx.source_db,
        ctx.target_db,
        ctx.plans,
        &target_hints,
        checkpoint,
        &mut progress,
    )
    .await?;
    Ok(copy::data_copy_stage(
        format!(
            "copied {} tables and {} rows",
            ctx.plans.len(),
            checkpoint.copied_rows
        ),
        "ok",
    ))
}
