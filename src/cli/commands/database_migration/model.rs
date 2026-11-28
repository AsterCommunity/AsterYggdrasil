//! Report structures for the database migration command.

use serde::Serialize;

/// Exit stages of a migration run; a stage either completes, plans ahead
/// (dry-run), skips (not applicable), or needs attention.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StageReport {
    pub name: String,
    pub status: String,
    pub message: String,
}

/// Backend identity and migration state of one database endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EndpointReport {
    pub database_url: String,
    pub backend: String,
    pub pending_migrations: Vec<String>,
}

/// Per-table counters mirrored between source and target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TableReport {
    pub name: String,
    pub primary_key: Vec<String>,
    pub source_rows: i64,
    pub target_rows: Option<i64>,
    pub copied_rows: i64,
    pub sequence_reset: bool,
}

/// One row-count disagreement between source and target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CountMismatch {
    pub table: String,
    pub source_rows: i64,
    pub target_rows: i64,
}

/// One violated unique or foreign-key constraint.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ConstraintCheck {
    pub table: String,
    pub constraint: String,
    pub columns: Vec<String>,
    pub violations: i64,
}

/// Post-copy verification outcome; problems surface here, never in the exit
/// code.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct VerificationReport {
    pub checked: bool,
    pub checked_unique_constraints: usize,
    pub checked_foreign_keys: usize,
    pub count_mismatches: Vec<CountMismatch>,
    pub unique_conflicts: Vec<ConstraintCheck>,
    pub foreign_key_violations: Vec<ConstraintCheck>,
}

impl VerificationReport {
    /// Whether verification actually ran and every row count and constraint
    /// check passed. An unrun verification is never "ready".
    pub fn ready(&self) -> bool {
        self.checked
            && self.count_mismatches.is_empty()
            && self.unique_conflicts.is_empty()
            && self.foreign_key_violations.is_empty()
    }

    /// Human summary rendered into the verification stage message.
    pub fn message(&self) -> String {
        if !self.checked {
            return "verification did not run".to_string();
        }
        if self.ready() {
            format!(
                "row counts matched; checked {} unique constraints and {} foreign keys",
                self.checked_unique_constraints, self.checked_foreign_keys
            )
        } else {
            format!(
                "counts={} unique_conflicts={} foreign_key_violations={}",
                self.count_mismatches.len(),
                self.unique_conflicts.len(),
                self.foreign_key_violations.len()
            )
        }
    }
}

/// Checkpoint state surfaced in the final report.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ResumeReport {
    pub enabled: bool,
    pub resumed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_table: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_table_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_table_offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copied_rows: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_rows: Option<i64>,
}

impl ResumeReport {
    /// Disabled variant for modes without a checkpoint lifecycle.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            resumed: false,
            ..Self::default()
        }
    }

    /// Enabled variant derived from the final checkpoint row.
    pub fn from_checkpoint(
        checkpoint: &super::checkpoint::MigrationCheckpoint,
        resumed: bool,
    ) -> Self {
        Self {
            enabled: true,
            resumed,
            migration_key: Some(checkpoint.migration_key.clone()),
            status: Some(checkpoint.status.clone()),
            stage: Some(checkpoint.stage.clone()),
            current_table: checkpoint.current_table.clone(),
            current_table_index: Some(
                usize::try_from(checkpoint.current_table_index.max(0)).unwrap_or(0),
            ),
            current_table_offset: Some(checkpoint.current_table_offset),
            copied_rows: Some(checkpoint.copied_rows),
            total_rows: Some(checkpoint.total_rows),
        }
    }
}

/// Aggregate counters for the run.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TotalsReport {
    pub tables: usize,
    pub source_rows: i64,
    pub target_rows: i64,
    pub copied_rows: i64,
    pub duration_ms: i64,
}

/// The full database migration command report, rendered as the success
/// envelope payload.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DatabaseMigrationReport {
    pub mode: String,
    pub ready_to_cutover: bool,
    pub rolled_back: bool,
    pub source: EndpointReport,
    pub target: EndpointReport,
    pub stages: Vec<StageReport>,
    pub tables: Vec<TableReport>,
    pub verification: VerificationReport,
    pub totals: TotalsReport,
    pub resume: ResumeReport,
}
