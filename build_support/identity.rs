//! Git provenance and Cargo feature normalization for build metadata.

use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn nearest_existing_watch_path(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|candidate| candidate.exists())
        .map(Path::to_path_buf)
}

pub(crate) fn feature_names(keys: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut features: Vec<_> = keys
        .into_iter()
        .filter_map(|key| {
            let feature = key.as_ref().strip_prefix("CARGO_FEATURE_")?;
            if feature.is_empty() || matches!(feature, "DEFAULT" | "FULL") {
                return None;
            }
            Some(feature.to_ascii_lowercase().replace('_', "-"))
        })
        .collect();
    features.sort_unstable();
    features
}

#[derive(Debug)]
pub(crate) struct SourceIdentity {
    pub(crate) revision: String,
    pub(crate) dirty: Option<bool>,
    pub(crate) watch_paths: Vec<PathBuf>,
}

pub(crate) fn source_identity(
    manifest_dir: &Path,
    revision_override: Option<&str>,
) -> io::Result<SourceIdentity> {
    let override_value = revision_override.map(str::trim);
    if let Some(value) = override_value
        && value != "unknown"
        && !valid_revision(value)
    {
        return Err(io::Error::other(
            "ASTER_BUILD_REVISION must be 'unknown' or a 7-64 character hexadecimal Git revision",
        ));
    }
    let root = git_output(manifest_dir, &["rev-parse", "--show-toplevel"])
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|value| PathBuf::from(value.trim()));
    let own_repository = root.as_deref().and_then(|path| path.canonicalize().ok())
        == manifest_dir.canonicalize().ok()
        && root.is_some();
    let head = own_repository
        .then(|| git_output(manifest_dir, &["rev-parse", "--verify", "HEAD"]))
        .flatten()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| valid_revision(value));
    let revision = override_value
        .map(str::to_owned)
        .or_else(|| head.clone())
        .unwrap_or_else(|| "unknown".into());
    let dirty = head
        .as_deref()
        .filter(|head| {
            head.get(..revision.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&revision))
        })
        .and_then(|_| {
            git_output(
                manifest_dir,
                &["status", "--porcelain", "--untracked-files=normal"],
            )
        })
        .map(|status| !status.is_empty());
    let mut watch_paths = Vec::new();
    if own_repository {
        for git_path in ["HEAD", "index", "packed-refs"] {
            if let Some(path) = git_output(manifest_dir, &["rev-parse", "--git-path", git_path])
                .and_then(|bytes| String::from_utf8(bytes).ok())
            {
                let path = PathBuf::from(path.trim());
                watch_paths.push(if path.is_absolute() {
                    path
                } else {
                    manifest_dir.join(path)
                });
            }
        }
        if let Some(reference) = git_output(manifest_dir, &["symbolic-ref", "--quiet", "HEAD"])
            .and_then(|bytes| String::from_utf8(bytes).ok())
            && let Some(path) =
                git_output(manifest_dir, &["rev-parse", "--git-path", reference.trim()])
                    .and_then(|bytes| String::from_utf8(bytes).ok())
        {
            let path = PathBuf::from(path.trim());
            watch_paths.push(if path.is_absolute() {
                path
            } else {
                manifest_dir.join(path)
            });
        }
        if let Some(files) = git_output(
            manifest_dir,
            &[
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ],
        ) {
            for file in files
                .split(|byte| *byte == 0)
                .filter(|path| !path.is_empty())
            {
                if let Ok(file) = std::str::from_utf8(file)
                    && !file.contains(['\r', '\n'])
                {
                    watch_paths.push(manifest_dir.join(file));
                }
            }
        }
    }
    Ok(SourceIdentity {
        revision,
        dirty,
        watch_paths,
    })
}

fn valid_revision(value: &str) -> bool {
    (7..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn git_output(directory: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aster_forge_utils::raii::TempDirGuard;

    fn directory(label: &str) -> TempDirGuard {
        let guard = TempDirGuard::new(
            std::env::temp_dir().join(format!("aster-build-{label}-{}", uuid::Uuid::new_v4())),
            "build identity fixture",
        );
        std::fs::create_dir_all(guard.path()).unwrap();
        guard
    }

    fn git(directory: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=Build Identity Test",
                "-c",
                "user.email=build-test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=empty-hooks",
                "-c",
                "core.fsmonitor=false",
            ])
            .args(args)
            .current_dir(directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git fixture failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn repository(path: &Path) -> String {
        git(path, &["init", "--quiet"]);
        std::fs::write(path.join("source.rs"), "original\n").unwrap();
        git(path, &["add", "source.rs"]);
        git(path, &["commit", "--quiet", "-m", "fixture"]);
        git(path, &["rev-parse", "HEAD"])
    }

    #[test]
    fn feature_names_are_normalized_sorted_and_omit_aggregate_flags() {
        assert_eq!(
            feature_names([
                "CARGO_FEATURE_SERVER",
                "CARGO_FEATURE_OPERATOR_CLI",
                "CARGO_FEATURE_OTLP_METRICS",
                "CARGO_FEATURE_FULL",
                "CARGO_FEATURE_DEFAULT",
                "CARGO_FEATURE_",
                "CARGO_PKG_NAME",
            ]),
            ["operator-cli", "otlp-metrics", "server"]
        );
        assert!(feature_names(["CARGO_FEATURE_DEFAULT"]).is_empty());
    }

    #[test]
    fn explicit_revisions_accept_inclusive_hex_boundaries() {
        let archive = directory("hex-boundaries");
        for value in ["abcdef0", &"F".repeat(64), "unknown"] {
            let identity = source_identity(archive.path(), Some(value)).unwrap();
            assert_eq!(identity.revision, value);
            assert_eq!(identity.dirty, None);
        }
        for value in ["abcdef", "abcdefg", "åbcdef0", &"a".repeat(65)] {
            assert!(source_identity(archive.path(), Some(value)).is_err());
        }
    }

    #[test]
    fn revision_and_dirty_follow_clean_unstaged_staged_and_untracked_source() {
        let root = directory("repository");
        let head = repository(root.path());
        let clean = source_identity(root.path(), None).unwrap();
        assert_eq!(clean.revision, head);
        assert_eq!(clean.dirty, Some(false));
        assert!(clean.watch_paths.iter().any(|path| path.ends_with("HEAD")));
        assert!(clean.watch_paths.iter().any(|path| path.ends_with("index")));
        assert!(clean.watch_paths.contains(&root.path().join("source.rs")));

        std::fs::write(root.path().join("source.rs"), "modified\n").unwrap();
        assert_eq!(
            source_identity(root.path(), None).unwrap().dirty,
            Some(true)
        );
        git(root.path(), &["add", "source.rs"]);
        assert_eq!(
            source_identity(root.path(), None).unwrap().dirty,
            Some(true)
        );
        git(root.path(), &["commit", "--quiet", "-m", "update"]);
        let updated = source_identity(root.path(), None).unwrap();
        assert_ne!(updated.revision, head);
        assert_eq!(updated.dirty, Some(false));

        std::fs::write(root.path().join("new.rs"), "untracked\n").unwrap();
        let untracked = source_identity(root.path(), None).unwrap();
        assert_eq!(untracked.dirty, Some(true));
        assert!(untracked.watch_paths.contains(&root.path().join("new.rs")));
    }

    #[test]
    fn missing_git_and_source_paths_fall_back_to_existing_parents() {
        let root = directory("missing-watch-paths");
        let head = repository(root.path());
        let packed_refs = root.path().join(".git/packed-refs");
        if packed_refs.exists() {
            std::fs::remove_file(&packed_refs).unwrap();
        }
        let packed_refs_identity = source_identity(root.path(), None).unwrap();
        assert!(packed_refs_identity.watch_paths.contains(&packed_refs));
        assert_eq!(
            nearest_existing_watch_path(&packed_refs),
            packed_refs.parent().map(Path::to_path_buf)
        );

        let branch = git(root.path(), &["symbolic-ref", "--short", "HEAD"]);
        let loose_ref = root.path().join(git(
            root.path(),
            &["rev-parse", "--git-path", &format!("refs/heads/{branch}")],
        ));
        std::fs::remove_file(&loose_ref).unwrap();
        let deleted_ref_identity = source_identity(root.path(), None).unwrap();
        assert_eq!(deleted_ref_identity.revision, "unknown");
        assert!(deleted_ref_identity.watch_paths.contains(&loose_ref));
        assert_eq!(
            nearest_existing_watch_path(&loose_ref),
            loose_ref.parent().map(Path::to_path_buf)
        );

        let root = directory("deleted-tracked-watch");
        let _head = repository(root.path());
        let tracked = root.path().join("source.rs");
        std::fs::remove_file(&tracked).unwrap();
        let deleted_file_identity = source_identity(root.path(), None).unwrap();
        assert!(deleted_file_identity.watch_paths.contains(&tracked));
        assert_eq!(
            nearest_existing_watch_path(&tracked),
            tracked.parent().map(Path::to_path_buf)
        );

        // Keep the fixture's revision in scope so this test also proves the
        // repository remained readable before the deliberate ref deletion.
        assert!(!head.is_empty());
    }

    #[test]
    fn revision_override_retains_unknown_provenance_in_archives_and_unrelated_revisions() {
        let root = directory("archive");
        let head = repository(root.path());
        let short = &head[..12];
        assert_eq!(
            source_identity(root.path(), Some(short)).unwrap().dirty,
            Some(false)
        );
        assert_eq!(
            source_identity(root.path(), Some("abcdef012345"))
                .unwrap()
                .dirty,
            None
        );
        assert_eq!(
            source_identity(root.path(), Some("unknown")).unwrap().dirty,
            None
        );
        for invalid in [
            "",
            " ",
            "bad",
            "not-a-git-revision",
            "abcdef0\ncredential-marker",
            &"a".repeat(65),
        ] {
            let error = source_identity(root.path(), Some(invalid))
                .unwrap_err()
                .to_string();
            assert!(!error.contains("credential-marker"));
        }

        // A source archive nested inside another checkout must not inherit that checkout's revision.
        let archive = root.path().join("archive");
        std::fs::create_dir_all(&archive).unwrap();
        let unknown = source_identity(&archive, None).unwrap();
        assert_eq!(unknown.revision, "unknown");
        assert_eq!(unknown.dirty, None);
        assert!(unknown.watch_paths.is_empty());
        let explicit = source_identity(&archive, Some(short)).unwrap();
        assert_eq!(explicit.revision, short);
        assert_eq!(explicit.dirty, None);
    }

    #[test]
    fn linked_worktree_uses_its_own_head_and_index() {
        let root = directory("worktree");
        let repository_path = root.path().join("repository");
        std::fs::create_dir_all(&repository_path).unwrap();
        let head = repository(&repository_path);
        let checkout = root.path().join("linked");
        git(
            &repository_path,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                checkout.to_str().unwrap(),
                "HEAD",
            ],
        );
        assert!(checkout.join(".git").is_file());
        let clean = source_identity(&checkout, None).unwrap();
        assert_eq!(clean.revision, head);
        assert_eq!(clean.dirty, Some(false));
        let index = PathBuf::from(git(&checkout, &["rev-parse", "--git-path", "index"]));
        assert!(clean.watch_paths.contains(&index));
        std::fs::write(checkout.join("source.rs"), "linked changes\n").unwrap();
        assert_eq!(source_identity(&checkout, None).unwrap().dirty, Some(true));
        assert_eq!(
            source_identity(&repository_path, None).unwrap().dirty,
            Some(false)
        );
    }
}
