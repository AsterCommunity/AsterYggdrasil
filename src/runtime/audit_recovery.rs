//! Preserve unconfirmed audit requests for operator reconciliation on failed drain.
//!
//! Forge owns the recovery-file mechanics (`aster_forge_audit::preserve_unconfirmed`);
//! this module only binds the shutdown tail to the product's owned audit writer. The
//! owned runtime has completed cleanup before this is called. Never replay these
//! requests automatically: an interrupted commit may already be present in the DB.

use std::{io, path::Path};

use aster_forge_audit::AuditWriter;

use crate::services::ops::audit::runtime::AUDIT_DRAIN_TIMEOUT;

/// Private directory that receives unconfirmed audit records for operator
/// reconciliation. Forge creates it owner-only and never replays its contents.
pub(super) fn recovery_directory() -> std::path::PathBuf {
    Path::new(crate::utils::paths::DEFAULT_DATA_DIR).join("audit-recovery")
}

/// Stops the writer, preserves any unconfirmed records, and returns the service result.
pub(super) async fn finish_audit(
    result: io::Result<()>,
    writer: &AuditWriter,
    directory: &Path,
) -> io::Result<()> {
    aster_forge_audit::finish_audit(result, writer, directory, AUDIT_DRAIN_TIMEOUT).await
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::finish_audit;
    use aster_forge_audit::AuditWriter;
    use aster_forge_db::AuditLogCreate;

    async fn failed_writer() -> AuditWriter {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let writer = AuditWriter::new(db);
        writer.start().await.unwrap();
        writer
            .record(AuditLogCreate {
                user_id: 0,
                action: "server_shutdown".into(),
                entity_type: "system_config".into(),
                entity_id: None,
                entity_name: Some("original name".into()),
                details: Some(r#"{"safe":true}"#.into()),
                ip_address: None,
                user_agent: None,
                created_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
        assert!(
            writer
                .flush(tokio::time::Instant::now() + std::time::Duration::from_secs(5))
                .await
                .is_err()
        );
        writer
    }

    #[tokio::test]
    async fn clean_writer_preserves_a_successful_runtime() {
        let directory = aster_forge_test::temp::TestTempDir::new("audit-recovery-clean");
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let writer = AuditWriter::new(db);
        writer.start().await.unwrap();

        finish_audit(Ok(()), &writer, directory.path())
            .await
            .expect("a clean drain should preserve success");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn clean_writer_preserves_the_original_runtime_error() {
        let directory = aster_forge_test::temp::TestTempDir::new("audit-recovery-error");
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let writer = AuditWriter::new(db);
        writer.start().await.unwrap();

        let error = finish_audit(
            Err(io::Error::other("original runtime failure")),
            &writer,
            directory.path(),
        )
        .await
        .expect_err("the original runtime error should be returned");
        assert_eq!(error.to_string(), "original runtime failure");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_drain_preserves_originals_in_private_unique_files() {
        let temp = aster_forge_test::temp::TestTempDir::new("audit-recovery-failed");
        // Forge recovery files open every path component with NOFOLLOW, so the
        // target must be fully resolved (macOS /var is a symlink).
        let directory = temp.path().canonicalize().unwrap().join("audit-recovery");
        let writer = failed_writer().await;

        let error = finish_audit(Ok(()), &writer, &directory)
            .await
            .expect_err("unconfirmed records must fail the shutdown tail");
        assert!(
            error.to_string().contains("unconfirmed audit records"),
            "{error}"
        );
        let recovery_dir = &directory;
        let entries: Vec<_> = std::fs::read_dir(recovery_dir)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "each failed drain writes exactly one unique recovery file"
        );
        let file_name = entries[0].file_name();
        assert!(
            file_name.to_string_lossy().starts_with("audit-"),
            "recovery files use unique audit-{{uuid}}.json names: {file_name:?}"
        );
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(entries[0].path()).unwrap().permissions();
            assert_eq!(mode.mode() & 0o777, 0o600, "recovery files stay private");
            let directory_mode = std::fs::metadata(recovery_dir).unwrap().permissions();
            assert_eq!(
                directory_mode.mode() & 0o777,
                0o700,
                "the recovery directory stays owner-only"
            );
        }
        let payload = std::fs::read_to_string(entries[0].path()).unwrap();
        assert!(
            payload.contains(r#""requires_database_reconciliation": true"#),
            "the envelope must demand manual reconciliation"
        );
        assert!(payload.contains("original name"), "records are preserved");
        assert!(
            !error.to_string().contains("safe"),
            "driver errors and payloads stay out of the public error"
        );
    }
}
