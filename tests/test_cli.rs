//! 集成测试：正式 CLI 进程边界。
//!
//! 这些测试驱动真实二进制：help/版本输出、serve 角色参数校验、hash-password
//! 的管道输入契约。全部用例在临时目录运行，并证明 CLI 解析与运维子命令不会
//! 触发服务器启动路径（配置加载、数据库、日志）。

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn aster_yggdrasil_bin() -> &'static str {
    env!("CARGO_BIN_EXE_aster_yggdrasil")
}

fn temp_cli_dir(label: &'static str) -> aster_forge_utils::raii::TempDirGuard {
    let dir =
        std::env::temp_dir().join(format!("aster-yggdrasil-cli-{label}-{}", uuid_for_tests()));
    std::fs::create_dir_all(&dir).expect("temp cli dir should create");
    // Dropping the guard cleans up even when an assertion panics mid-test.
    aster_forge_utils::raii::TempDirGuard::new(dir, "CLI test temp dir")
}

fn uuid_for_tests() -> String {
    // getrandom-backed uuid v4 through the workspace dependency; a plain unique
    // suffix is enough for temp-dir collisions, so format it directly.
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should advance")
        .as_nanos();
    let pid = std::process::id();
    format!(
        "{nanos}-{}-{pid}",
        std::thread::current().name().unwrap_or("t")
    )
}

fn run_in_dir(args: &[&str], dir: &std::path::Path) -> Output {
    Command::new(aster_yggdrasil_bin())
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("aster_yggdrasil should spawn")
}

fn run_hash_password_in_dir(input: &[u8], dir: &std::path::Path) -> Output {
    let mut child = Command::new(aster_yggdrasil_bin())
        .arg("hash-password")
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("hash-password should spawn");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input)
        .expect("test input should write");
    child
        .wait_with_output()
        .expect("hash-password should finish")
}

#[test]
fn help_outputs_are_stable_and_never_touch_server_state() {
    let dir = temp_cli_dir("help");
    for args in [
        vec!["--help"],
        vec!["-h"],
        vec!["serve", "--help"],
        vec!["serve", "-h"],
        vec!["hash-password", "--help"],
        vec!["--version"],
        vec!["-V"],
    ] {
        let output = run_in_dir(&args, dir.path());
        assert!(
            output.status.success(),
            "args {args:?} should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("help stdout should be utf-8");
        let expected_banner = concat!("AsterYggdrasil ", env!("CARGO_PKG_VERSION"));
        if args == ["--version"] || args == ["-V"] {
            assert!(
                stdout.contains(env!("CARGO_PKG_VERSION")),
                "version output should contain the package version: {stdout}"
            );
        } else {
            assert_eq!(
                stdout.lines().next(),
                Some(expected_banner),
                "help banner should lead with name and version for {args:?}"
            );
        }
        assert!(
            !stdout.contains("[env:"),
            "help must never leak environment values: {stdout}"
        );
        assert!(
            !dir.path().join("data").exists(),
            "CLI parsing must precede configuration/database initialization"
        );
    }
}

#[test]
fn root_help_lists_every_subcommand_and_the_no_argument_behavior() {
    let dir = temp_cli_dir("root-help");
    let output = run_in_dir(&["--help"], dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help stdout should be utf-8");

    assert!(
        stdout.contains(env!("CARGO_PKG_DESCRIPTION")),
        "root help should include the package description: {stdout}"
    );
    assert!(stdout.contains("serve"));
    assert!(stdout.contains("Run the server with all, API, or worker responsibilities"));
    assert!(stdout.contains("hash-password"));
    assert!(stdout.contains(
        "Generate an authentication verifier (masked terminal input, or stdin until EOF)"
    ));
    assert!(stdout.contains("--config-file <PATH>"));
    assert!(stdout.contains("--data-dir <PATH>"));
    assert!(stdout.contains("Examples:"));
    assert!(stdout.contains("Running without a command starts all server roles."));
    assert!(
        !stdout.contains("\n\n\n"),
        "root help should stay compact: {stdout}"
    );
}

#[test]
fn location_arguments_route_configuration_loading() {
    // --data-dir: the server reads config.toml under the given directory. A
    // pre-written invalid config proves the file at that location is the one
    // actually loaded, while keeping the process from starting.
    let dir = temp_cli_dir("data-dir");
    let data_dir = dir.path().join("instance");
    let config_root = data_dir.join("data");
    std::fs::create_dir_all(&config_root).expect("data root should create");
    std::fs::write(config_root.join("config.toml"), "not valid TOML")
        .expect("invalid config should write");

    let output = run_in_dir(
        &["--data-dir", data_dir.to_str().unwrap(), "serve"],
        dir.path(),
    );
    assert!(
        !output.status.success(),
        "serve must load the config under --data-dir and fail on it: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // --config-file: an explicit file is honored; failure surfaces its content.
    let explicit_dir = temp_cli_dir("config-file");
    let config_path = explicit_dir.path().join("custom.toml");
    std::fs::write(&config_path, "not valid TOML").expect("explicit config should write");
    let output = run_in_dir(
        &["--config-file", config_path.to_str().unwrap(), "serve"],
        explicit_dir.path(),
    );
    assert!(!output.status.success());
    assert!(
        !explicit_dir.path().join("data/config.toml").exists(),
        "explicit config must not trigger default config generation (the data/ dir itself is the crash-log target)"
    );
}

#[test]
fn config_file_and_data_dir_are_mutually_exclusive() {
    let dir = temp_cli_dir("location-conflict");
    let output = run_in_dir(
        &[
            "--config-file",
            "a.toml",
            "--data-dir",
            "/srv/ygg",
            "hash-password",
        ],
        dir.path(),
    );
    assert!(
        !output.status.success(),
        "conflicting location args must fail"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot be used with"),
        "clap should explain the conflict: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn serve_help_documents_the_role_values() {
    let dir = temp_cli_dir("serve-help");
    let output = run_in_dir(&["serve", "--help"], dir.path());
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("serve help stdout should be utf-8");
    assert!(stdout.contains("Run the server with all, API, or worker responsibilities"));
    for role in ["all", "api", "worker"] {
        assert!(stdout.contains(role), "serve help should mention '{role}'");
    }
}

#[test]
fn invalid_serve_role_arguments_fail_without_startup_side_effects() {
    let dir = temp_cli_dir("invalid-role");
    for args in [
        vec!["serve", "invalid"],
        vec!["serve", "api", "worker"],
        vec!["serve", "--role", "worker"],
        vec!["serve", "API"],
    ] {
        let output = run_in_dir(&args, dir.path());
        assert!(!output.status.success(), "args {args:?} must be rejected");
        assert!(
            !String::from_utf8_lossy(&output.stderr).is_empty(),
            "rejection should explain itself"
        );
        assert!(
            !dir.path().join("data").exists(),
            "invalid arguments must not bootstrap server state"
        );
    }
}

#[test]
fn serve_fails_closed_on_an_invalid_configuration() {
    // `serve` really routes through bootstrap: an unparsable config must fail
    // fast instead of silently falling back to defaults.
    let dir = temp_cli_dir("serve-invalid-config");
    std::fs::create_dir_all(dir.path().join("data")).expect("data dir should create");
    std::fs::write(dir.path().join("data/config.toml"), "not valid TOML")
        .expect("invalid config should write");

    let output = run_in_dir(&["serve"], dir.path());
    assert!(
        !output.status.success(),
        "serve must fail on an unparsable configuration"
    );
}

#[test]
fn hash_password_reads_stdin_offline_and_rejects_invalid_input_without_echoing_it() {
    let dir = temp_cli_dir("hash-password");
    // An invalid server configuration proves this command never starts server
    // resources or logging, and never generates or repairs configuration files.
    std::fs::create_dir(dir.path().join("data")).expect("data dir should create");
    let invalid_config = "not valid TOML";
    std::fs::write(dir.path().join("data/config.toml"), invalid_config)
        .expect("config should write");

    let secrets: Vec<(Vec<u8>, String)> = vec![
        (b"scrape:secret\r\n".to_vec(), "scrape:secret".to_string()),
        (b"scrape:secret\n".to_vec(), "scrape:secret".to_string()),
        (b"scrape:secret\r".to_vec(), "scrape:secret".to_string()),
        (b"scrape:secret".to_vec(), "scrape:secret".to_string()),
        (
            b"  scrape:secret  \r\n".to_vec(),
            "  scrape:secret  ".to_string(),
        ),
        (
            format!("{}\r\n", "x".repeat(4096)).into_bytes(),
            "x".repeat(4096),
        ),
        ([b"x".to_vec(), b"\n".to_vec()].concat(), "x".to_string()),
    ];
    for (input, expected_password) in secrets {
        let output = run_hash_password_in_dir(&input, dir.path());
        assert!(
            output.status.success(),
            "hash-password should accept a valid piped password (len {}): {}",
            input.len(),
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("hash stdout should be utf-8");
        assert_eq!(
            stdout.lines().count(),
            1,
            "stdout must be a single PHC line"
        );
        assert!(stdout.starts_with("$argon2id$"));
        let phc = stdout.trim();
        assert!(
            aster_forge_crypto::verify_password(&expected_password, phc)
                .expect("verifier generation should produce a checkable PHC string"),
            "printed PHC must verify against the piped password"
        );
        assert!(
            output.stderr.is_empty(),
            "successful hash-password must stay silent on stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !stdout.contains("scrape:secret"),
            "stdout must never echo the piped secret"
        );
    }

    for input in [
        Vec::new(),
        b"\n".to_vec(),
        b"credential-marker\nsecond-line".to_vec(),
        b"credential-marker\t".to_vec(),
        b"credential-marker\0".to_vec(),
        vec![b'x'; 4097],
        vec![0xff],
        vec![b'x'; 4098],
        [vec![b'x'; 4095], vec![b'\t', b'\n']].concat(),
    ] {
        let output = run_hash_password_in_dir(&input, dir.path());
        assert!(
            !output.status.success(),
            "invalid piped input (len {}) must be rejected",
            input.len()
        );
        assert!(
            output.stdout.is_empty(),
            "rejected input must not produce a verifier"
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("credential-marker"),
            "error output must never echo the rejected secret"
        );
    }

    assert_eq!(
        std::fs::read_to_string(dir.path().join("data/config.toml"))
            .expect("config should stay intact"),
        invalid_config,
        "hash-password must not touch configuration files"
    );
    let entries = std::fs::read_dir(dir.path()).expect("cli dir should list");
    assert_eq!(
        entries.count(),
        1,
        "hash-password must not create files next to the config"
    );
}

#[test]
fn hash_password_truncation_boundary_stays_rejecting() {
    // take(4099) exists so a long input cannot be silently accepted after
    // truncation at a line ending: 4096 payload bytes plus a CRLF sit exactly
    // below the cap, 4097 payload bytes plus CRLF overflow and must fail.
    let dir = temp_cli_dir("hash-password-boundary");
    std::fs::create_dir_all(dir.path().join("data")).expect("data dir should create");

    let fits = format!("{}\r\n", "x".repeat(4096));
    assert!(
        run_hash_password_in_dir(fits.as_bytes(), dir.path())
            .status
            .success()
    );

    let overflows = format!("{}\r\n", "x".repeat(4097));
    let output = run_hash_password_in_dir(overflows.as_bytes(), dir.path());
    assert!(
        !output.status.success(),
        "4097-byte CRLF-terminated input must be rejected, not truncated"
    );
}
