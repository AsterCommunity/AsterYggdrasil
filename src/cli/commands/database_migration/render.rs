//! Human and JSON rendering for the migration report.

use super::model::DatabaseMigrationReport;
use crate::cli::common::output::{
    OutputFormat, ResolvedOutputFormat, render_error_json, render_success_envelope,
};
use crate::errors::Result;

/// Renders the report or error for the requested output format.
pub(in crate::cli) fn render_result(
    output_format: &OutputFormat,
    result: &Result<DatabaseMigrationReport>,
) -> String {
    match result {
        Ok(report) => match output_format.resolve() {
            ResolvedOutputFormat::Human => render_human(report),
            ResolvedOutputFormat::Json | ResolvedOutputFormat::PrettyJson => {
                render_success_envelope(report, true)
            }
        },
        Err(error) => match output_format.resolve() {
            ResolvedOutputFormat::Human => {
                format!("Database migration failed\nError: {error}")
            }
            ResolvedOutputFormat::Json | ResolvedOutputFormat::PrettyJson => {
                render_error_json(error, true)
            }
        },
    }
}

/// Badge for a stage/status token in human output.
fn badge(status: &str) -> &'static str {
    match status {
        "ok" | "completed" => "[OK]",
        "attention" => "[WARN]",
        "planned" => "[PLAN]",
        "skipped" => "[SKIP]",
        "running" => "[RUN]",
        "failed" => "[FAIL]",
        _ => "[INFO]",
    }
}

/// Renders duration in the `1.2m` / `0.85s` / `123ms` ladder.
fn render_duration(duration_ms: i64) -> String {
    if duration_ms >= 60_000 {
        format!("{:.1}m", duration_ms as f64 / 60_000.0)
    } else if duration_ms >= 1_000 {
        format!("{:.2}s", duration_ms as f64 / 1_000.0)
    } else {
        format!("{duration_ms}ms")
    }
}

/// Full human-readable report block.
fn render_human(report: &DatabaseMigrationReport) -> String {
    let mut out = Vec::new();
    let title = match report.mode.as_str() {
        "dry_run" => "Database migration dry run complete",
        "verify_only" => {
            if report.ready_to_cutover {
                "Database verification complete"
            } else {
                "Database verification needs attention"
            }
        }
        _ => {
            if report.ready_to_cutover {
                "Database migration complete"
            } else {
                "Database migration complete with attention"
            }
        }
    };
    out.push(title.to_string());
    out.push("-".repeat(54));
    out.push(format!("{:<14}: {}", "Mode", report.mode));
    out.push(format!(
        "{:<14}: {} ({})",
        "Source", report.source.backend, report.source.database_url
    ));
    out.push(format!(
        "{:<14}: {} ({})",
        "Target", report.target.backend, report.target.database_url
    ));
    out.push(format!("{:<14}: {}", "Tables", report.totals.tables));
    out.push(format!(
        "{:<14}: {}/{} copied",
        "Rows", report.totals.copied_rows, report.totals.source_rows
    ));
    out.push(format!(
        "{:<14}: {}",
        "Duration",
        render_duration(report.totals.duration_ms)
    ));
    out.push(format!(
        "{:<14}: {} ready for cutover",
        "Cutover",
        if report.ready_to_cutover {
            "[OK]"
        } else {
            "[WARN] blocked by verification issues"
        }
    ));
    out.push(format!(
        "{:<14}: {}",
        "Verification",
        if report.verification.checked {
            report.verification.message()
        } else {
            "not run".to_string()
        }
    ));
    out.push(String::new());
    out.push("Stages:".to_string());
    for stage in &report.stages {
        out.push(format!(
            "  {} {:<18} {}",
            badge(&stage.status),
            stage.name,
            stage.message
        ));
    }
    if report.resume.enabled {
        out.push(String::new());
        out.push("Resume:".to_string());
        out.push(format!(
            "  Checkpoint        {} ({})",
            report
                .resume
                .migration_key
                .as_deref()
                .unwrap_or("<unknown>"),
            if report.resume.resumed {
                "resumed existing"
            } else {
                "created new"
            }
        ));
        if let (Some(status), Some(stage)) = (&report.resume.status, &report.resume.stage) {
            out.push(format!("  Stage             {stage} ({status})"));
        }
        if let (Some(copied), Some(total)) = (report.resume.copied_rows, report.resume.total_rows) {
            out.push(format!("  Rows              {copied}/{total}"));
        }
    }
    if !report.ready_to_cutover && report.verification.checked {
        out.push(String::new());
        out.push("Verification details:".to_string());
        for mismatch in &report.verification.count_mismatches {
            out.push(format!(
                "  count mismatch    {} source={} target={}",
                mismatch.table, mismatch.source_rows, mismatch.target_rows
            ));
        }
        for conflict in &report.verification.unique_conflicts {
            out.push(format!(
                "  unique conflict   {} {} ({}) violations={}",
                conflict.table,
                conflict.constraint,
                conflict.columns.join(","),
                conflict.violations
            ));
        }
        for violation in &report.verification.foreign_key_violations {
            out.push(format!(
                "  foreign key       {} {} ({}) violations={}",
                violation.table,
                violation.constraint,
                violation.columns.join(","),
                violation.violations
            ));
        }
    }
    out.join("\n")
}
