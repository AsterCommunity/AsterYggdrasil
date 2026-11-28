//! Orchestration: mode selection, endpoint connection, preflight, and the
//! dry-run / verify-only / apply dispatch.

use super::{DatabaseMigrateArgs, apply, checkpoint, model, plan, schema, verify};

use crate::cli::common::database::{
    DatabaseAccess, backend_name, connect_existing_database, pending_migrations,
    redact_database_url,
};
use crate::errors::{AsterError, Result};
use model::{DatabaseMigrationReport, EndpointReport, ResumeReport, StageReport, TotalsReport};

/// Runs the migration pipeline end to end.
pub(in crate::cli) async fn execute_database_migration(
    args: &DatabaseMigrateArgs,
) -> Result<DatabaseMigrationReport> {
    let started = std::time::Instant::now();
    if args.dry_run && args.verify_only {
        return Err(AsterError::validation_error(
            "--dry-run and --verify-only are mutually exclusive",
        ));
    }
    if args.source_database_url == args.target_database_url {
        return Err(AsterError::validation_error(
            "source and target database URLs must differ",
        ));
    }
    let mode = if args.dry_run {
        "dry_run"
    } else if args.verify_only {
        "verify_only"
    } else {
        "apply"
    };

    // Source is always read-only; the target is read-write only in apply mode.
    let source_db =
        connect_existing_database(&args.source_database_url, DatabaseAccess::ReadOnly).await?;
    let target_access = if mode == "apply" {
        DatabaseAccess::ReadWrite
    } else {
        DatabaseAccess::ReadOnly
    };
    let target_db = connect_existing_database(&args.target_database_url, target_access).await?;

    let source_backend = source_db.get_database_backend();
    let target_backend = target_db.get_database_backend();
    schema::validate_backends(source_backend, target_backend)?;

    let pending_source = pending_migrations(&source_db).await?;
    if !pending_source.is_empty() {
        return Err(AsterError::validation_error(format!(
            "source database has pending migrations: {}; \
             run the server against it once before migrating",
            pending_source.join(", ")
        )));
    }

    let pending_source = pending_migrations(&source_db).await?;
    if !pending_source.is_empty() {
        return Err(AsterError::validation_error(format!(
            "source database has pending migrations: {}; \
             run the server against it once before migrating",
            pending_source.join(", ")
        )));
    }

    let plans = plan::load_source_plans(&source_db).await?;
    let mut target_pending_before = pending_migrations(&target_db).await?;
    if mode == "verify_only" && !target_pending_before.is_empty() {
        return Err(AsterError::validation_error(format!(
            "verify-only requires target schema to be current, but these migrations are pending: {}",
            target_pending_before.join(", ")
        )));
    }

    let mut stages = vec![StageReport {
        name: "preflight".to_string(),
        status: "ok".to_string(),
        message: format!(
            "source={} target={} pending_source=0 pending_target={}",
            backend_name(source_backend),
            backend_name(target_backend),
            target_pending_before.len()
        ),
    }];

    let mut tables = plan::plans_to_reports(&plans);
    let mut verification = model::VerificationReport::default();
    let mut resume = ResumeReport::disabled();

    match mode {
        "dry_run" => {
            let pending_label = if target_pending_before.is_empty() {
                "target schema already matches current migrations".to_string()
            } else {
                format!(
                    "would apply {} pending migrations: {}",
                    target_pending_before.len(),
                    target_pending_before.join(", ")
                )
            };
            stages.push(StageReport {
                name: "structure_prepare".to_string(),
                status: "planned".to_string(),
                message: pending_label,
            });
            stages.push(StageReport {
                name: "data_copy".to_string(),
                status: "planned".to_string(),
                message: format!(
                    "would copy {} tables and {} rows",
                    plans.len(),
                    plan::total_source_rows(&plans)
                ),
            });
            stages.push(StageReport {
                name: "verification".to_string(),
                status: "skipped".to_string(),
                message: "dry-run does not mutate target or execute post-copy verification"
                    .to_string(),
            });
        }
        "verify_only" => {
            stages.push(StageReport {
                name: "structure_prepare".to_string(),
                status: "ok".to_string(),
                message: "target schema matches current migrations".to_string(),
            });
            stages.push(StageReport {
                name: "data_copy".to_string(),
                status: "skipped".to_string(),
                message: "verify-only does not copy data".to_string(),
            });
            verification = verify::verify_target(&target_db, &plans).await?;
            schema::refresh_target_rows(&target_db, &mut tables).await?;
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
        }
        _ => {
            let execution = apply::execute_apply_mode(apply::ApplyModeContext {
                source_db: &source_db,
                target_db: &target_db,
                source_url: &args.source_database_url,
                target_url: &args.target_database_url,
                plans: &plans,
            })
            .await?;
            stages.extend(execution.stages);
            verification = execution.verification;
            let resumed = execution.resumed;
            let mut checkpoint = execution.checkpoint;

            // Record the final report inside the checkpoint row.
            schema::refresh_target_rows(&target_db, &mut tables).await?;
            let ready = verification.ready();
            let copied_rows: i64 = tables.iter().map(|table| table.copied_rows).sum();
            let final_report = DatabaseMigrationReport {
                mode: mode.to_string(),
                ready_to_cutover: ready,
                rolled_back: false,
                source: endpoint_report(&args.source_database_url, source_backend, Vec::new()),
                target: endpoint_report(
                    &args.target_database_url,
                    target_backend,
                    pending_migrations(&target_db).await.unwrap_or_default(),
                ),
                stages: Vec::new(),
                tables: tables.clone(),
                verification: verification.clone(),
                totals: totals(&tables, started),
                resume: ResumeReport::disabled(),
            };
            checkpoint.status = if ready { "completed" } else { "attention" }.to_string();
            checkpoint.stage = if ready { "complete" } else { "verification" }.to_string();
            checkpoint.result_json = Some(serde_json::to_string(&final_report).unwrap_or_default());
            checkpoint.last_error = None;
            checkpoint.current_table = None;
            checkpoint.current_table_index = plans.len() as i64;
            checkpoint.current_table_offset = 0;
            checkpoint.copied_rows = copied_rows;
            checkpoint.heartbeat_at_ms = chrono::Utc::now().timestamp_millis();
            checkpoint.updated_at_ms = checkpoint.heartbeat_at_ms;
            checkpoint::update_checkpoint(&target_db, &checkpoint).await?;
            resume = ResumeReport::from_checkpoint(&checkpoint, resumed);
        }
    }
    target_pending_before = pending_migrations(&target_db).await?;

    let ready = verification.ready();
    let totals = totals(&tables, started);
    Ok(DatabaseMigrationReport {
        mode: mode.to_string(),
        ready_to_cutover: ready,
        rolled_back: false,
        source: endpoint_report(&args.source_database_url, source_backend, Vec::new()),
        target: endpoint_report(
            &args.target_database_url,
            target_backend,
            target_pending_before,
        ),
        stages,
        tables,
        verification,
        totals,
        resume,
    })
}

/// Builds one endpoint report; URLs leave this function redacted.
fn endpoint_report(url: &str, backend: sea_orm::DbBackend, pending: Vec<String>) -> EndpointReport {
    EndpointReport {
        database_url: redact_database_url(url),
        backend: backend_name(backend).to_string(),
        pending_migrations: pending,
    }
}

/// Aggregates table counters with wall-clock duration.
fn totals(tables: &[model::TableReport], started: std::time::Instant) -> TotalsReport {
    TotalsReport {
        tables: tables.len(),
        source_rows: tables.iter().map(|table| table.source_rows).sum(),
        target_rows: tables
            .iter()
            .map(|table| table.target_rows.unwrap_or(0))
            .sum(),
        copied_rows: tables.iter().map(|table| table.copied_rows).sum(),
        duration_ms: aster_forge_utils::numbers::u128_to_u64_saturating(
            started.elapsed().as_millis(),
        ) as i64,
    }
}
