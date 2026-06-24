//! RAII cleanup guards for short-lived runtime resources.

pub use aster_forge_utils::raii::{TempDirGuard, TempFileGuard};

#[cfg(test)]
mod tests {
    use super::{TempDirGuard, TempFileGuard};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_ID: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "asteryggdrasil-raii-{label}-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn temp_file_guard_removes_file_on_drop_and_exposes_path() {
        let path = unique_temp_path("file");
        std::fs::write(&path, b"temporary").unwrap();

        {
            let guard = TempFileGuard::new(path.clone(), "test-temp-file");
            assert_eq!(guard.path(), path.as_path());
            assert!(path.exists());
        }

        assert!(!path.exists());
    }

    #[test]
    fn temp_file_guard_tolerates_missing_file() {
        let path = unique_temp_path("missing-file");
        let guard = TempFileGuard::new(path.clone(), "test-missing-file");

        drop(guard);

        assert!(!path.exists());
    }

    #[test]
    fn temp_dir_guard_removes_directory_tree_on_drop_and_exposes_path() {
        let path = unique_temp_path("dir");
        let nested = path.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("payload.txt"), b"temporary").unwrap();

        {
            let guard = TempDirGuard::new(path.clone(), "test-temp-dir");
            assert_eq!(guard.path(), path.as_path());
            assert!(nested.join("payload.txt").exists());
        }

        assert!(!path.exists());
    }

    #[test]
    fn temp_dir_guard_tolerates_missing_directory() {
        let path = unique_temp_path("missing-dir");
        let guard = TempDirGuard::new(path.clone(), "test-missing-dir");

        drop(guard);

        assert!(!path.exists());
    }
}
