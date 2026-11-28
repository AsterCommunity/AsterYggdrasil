//! Exercises the same provenance helpers used by the build script.

#[path = "../build_support/identity.rs"]
mod identity;

#[test]
fn cargo_rebuilds_build_identity_for_source_changes_but_not_target_artifacts() {
    use std::process::Command;

    let root = std::env::temp_dir().join(format!(
        "aster-build-watch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/build_identity_watch");
    copy_tree(&fixture, &root);

    let target = root.with_extension("cargo-target");
    let manifest = root.join("Cargo.toml");
    let build = || {
        let output = Command::new(env!("CARGO"))
            .args(["build", "--quiet", "--manifest-path"])
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", &target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Command::new(target.join("debug/build_identity_watch"))
            .output()
            .unwrap()
            .stdout
    };

    let initial = build();
    assert!(!initial.is_empty());
    assert_eq!(build(), initial, "an unchanged package must stay fresh");

    std::fs::write(root.join("scratch.txt"), "untracked source change").unwrap();
    let after_new_root_file = build();
    assert_ne!(after_new_root_file, initial);

    std::fs::write(root.join("src/lib.rs"), "pub fn changed() {}\n").unwrap();
    let after_source_edit = build();
    assert_ne!(after_source_edit, after_new_root_file);

    std::fs::create_dir_all(root.join("target")).unwrap();
    std::fs::write(root.join("target/ignored-artifact"), "generated").unwrap();
    assert_eq!(
        build(),
        after_source_edit,
        "package target artifacts must not rerun the build script"
    );

    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(&target).unwrap();
}

fn copy_tree(source: &std::path::Path, destination: &std::path::Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source = entry.path();
        let destination = destination.join(entry.file_name());
        if source.is_dir() {
            copy_tree(&source, &destination);
        } else {
            std::fs::copy(source, destination).unwrap();
        }
    }
}
