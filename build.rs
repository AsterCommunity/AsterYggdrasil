//! Build script: inject build identity (time, revision, target, profile,
//! features) and provide fallback frontend assets.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const BUILD_TIME_ENV: &str = "ASTER_BUILD_TIME";
const BUILD_REVISION_ENV: &str = "ASTER_BUILD_REVISION";
const BUILD_PROFILE_ENV: &str = "ASTER_BUILD_PROFILE";
const BUILD_TARGET_ENV: &str = "ASTER_BUILD_TARGET";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=frontend-panel/dist");
    println!("cargo:rerun-if-env-changed={BUILD_TIME_ENV}");
    println!("cargo:rerun-if-env-changed={BUILD_REVISION_ENV}");

    configure_build_time()?;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|error| io::Error::other(format!("missing CARGO_MANIFEST_DIR: {error}")))?;
    configure_build_revision(Path::new(&manifest_dir))?;
    configure_build_features();
    configure_build_context(
        &env::var("PROFILE")
            .map_err(|error| io::Error::other(format!("missing PROFILE: {error}")))?,
        &env::var("TARGET")
            .map_err(|error| io::Error::other(format!("missing TARGET: {error}")))?,
    )?;

    let dist_path = Path::new(&manifest_dir).join("frontend-panel/dist");

    if !dist_path.exists() {
        create_fallback_files(&dist_path)?;
    }

    Ok(())
}

/// Injects the build timestamp. An explicit `ASTER_BUILD_TIME` override (used
/// by release automation to keep builds reproducible) is validated and passed
/// through; otherwise the current UTC time is captured.
fn configure_build_time() -> io::Result<()> {
    let value = match env::var(BUILD_TIME_ENV) {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => {
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
            println!("cargo:rustc-env={BUILD_TIME_ENV}={now}");
            return Ok(());
        }
        Err(env::VarError::NotUnicode(_)) => {
            return Err(io::Error::other(format!(
                "{BUILD_TIME_ENV} must contain valid Unicode"
            )));
        }
    };

    let value = value.trim();
    if value.is_empty() {
        return Err(io::Error::other(format!(
            "{BUILD_TIME_ENV} must not be empty when set"
        )));
    }
    if value.contains('\r') || value.contains('\n') {
        return Err(io::Error::other(format!(
            "{BUILD_TIME_ENV} must be a single-line value"
        )));
    }
    println!("cargo:rustc-env={BUILD_TIME_ENV}={value}");

    Ok(())
}

/// Captures the git revision and dirty state of the source tree. An explicit
/// `ASTER_BUILD_REVISION` override (set by release automation when building
/// from an export or CI artifact) must be `unknown` or a 7-64 character
/// hexadecimal revision.
fn configure_build_revision(manifest_dir: &Path) -> io::Result<()> {
    let revision_override = env::var(BUILD_REVISION_ENV)
        .map(Some)
        .or_else(|error| match error {
            env::VarError::NotPresent => Ok(None),
            _ => Err(io::Error::other(
                format!("{BUILD_REVISION_ENV} must contain valid Unicode"),
            )),
        })?;
    let identity = source_identity(manifest_dir, revision_override.as_deref())?;
    println!("cargo:rustc-env={BUILD_REVISION_ENV}={}", identity.revision);
    println!(
        "cargo:rustc-env=ASTER_BUILD_DIRTY={}",
        identity
            .dirty
            .map_or("unknown", |dirty| if dirty { "true" } else { "false" })
    );
    for path in identity.watch_paths {
        if let Some(path) = nearest_existing_watch_path(&path) {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
    Ok(())
}

fn nearest_existing_watch_path(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|candidate| candidate.exists())
        .map(Path::to_path_buf)
}

/// Collects enabled cargo features (excluding the synthetic `DEFAULT`/`FULL`
/// markers) as a sorted, comma-separated list for the version command.
fn configure_build_features() {
    let mut features: Vec<String> = env::vars_os()
        .filter_map(|(key, _)| {
            let key = key.to_str()?;
            let feature = key.strip_prefix("CARGO_FEATURE_")?;
            if matches!(feature, "DEFAULT" | "FULL") {
                return None;
            }
            Some(feature.to_ascii_lowercase().replace('_', "-"))
        })
        .collect();
    features.sort();
    println!(
        "cargo:rustc-env=ASTER_BUILD_FEATURES={}",
        features.join(",")
    );
}

/// Injects the cargo profile and target triple; both are required, single-line
/// values.
fn configure_build_context(profile: &str, target: &str) -> io::Result<()> {
    for (name, value) in [
        (BUILD_PROFILE_ENV, profile),
        (BUILD_TARGET_ENV, target),
    ] {
        if value.is_empty() || value.contains(['\r', '\n']) {
            return Err(io::Error::other(format!(
                "{name} must be a non-empty single-line value, got: {value:?}"
            )));
        }
        println!("cargo:rustc-env={name}={value}");
    }

    Ok(())
}

struct SourceIdentity {
    revision: String,
    dirty: Option<bool>,
    watch_paths: Vec<PathBuf>,
}

fn source_identity(
    manifest_dir: &Path,
    revision_override: Option<&str>,
) -> io::Result<SourceIdentity> {
    let override_value = revision_override.map(str::trim);
    if let Some(value) = override_value
        && value != "unknown"
        && (!(7..=64).contains(&value.len()) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(io::Error::other(format!(
            "{BUILD_REVISION_ENV} must be 'unknown' or a 7-64 character hexadecimal Git revision"
        )));
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
        .map(|value| value.trim().to_owned());
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
    }
    Ok(SourceIdentity {
        revision,
        dirty,
        watch_paths,
    })
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

fn create_fallback_files(dist_path: &Path) -> io::Result<()> {
    fs::create_dir_all(dist_path)?;

    let fallback_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/svg+xml" href="%ASTERYGGDRASIL_FAVICON_URL%" />
    <link rel="apple-touch-icon" href="%ASTERYGGDRASIL_FAVICON_URL%" />
    <meta name="description" content="%ASTERYGGDRASIL_DESCRIPTION%" />
    <meta http-equiv="Content-Security-Policy" content="%ASTERYGGDRASIL_CSP%" />
    <meta name="asteryggdrasil-version" content="%ASTERYGGDRASIL_VERSION%" />
    <meta name="asteryggdrasil-wordmark-dark" content="%ASTERYGGDRASIL_WORDMARK_DARK_URL%" />
    <meta name="asteryggdrasil-wordmark-light" content="%ASTERYGGDRASIL_WORDMARK_LIGHT_URL%" />
    <meta name="asteryggdrasil-csrf-cookie-name" content="%ASTERYGGDRASIL_CSRF_COOKIE_NAME%" />
    <meta name="asteryggdrasil-csrf-header-name" content="%ASTERYGGDRASIL_CSRF_HEADER_NAME%" />
    <title>%ASTERYGGDRASIL_TITLE% - Frontend Not Built</title>
    <style>
        :root { color-scheme: dark; }
        body {
            margin: 0;
            min-height: 100vh;
            display: grid;
            place-items: center;
            background: #111827;
            color: #f8fafc;
            font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        }
        main {
            width: min(560px, calc(100vw - 48px));
            border: 1px solid #334155;
            border-radius: 8px;
            padding: 28px;
            background: #0f172a;
        }
        h1 { margin: 0 0 10px; font-size: 28px; letter-spacing: 0; }
        p { color: #cbd5e1; line-height: 1.6; }
        code {
            background: #1e293b;
            border: 1px solid #334155;
            border-radius: 6px;
            padding: 2px 6px;
        }
    </style>
</head>
<body>
    <main>
        <h1>%ASTERYGGDRASIL_TITLE%</h1>
        <p>The embedded frontend has not been built yet.</p>
        <p>Run <code>cd frontend-panel && bun install && bun run build</code>, then restart the server.</p>
        <p>API health remains available at <code>/health</code> and <code>/health/ready</code>.</p>
    </main>
</body>
</html>"#;

    fs::write(dist_path.join("index.html"), fallback_html)?;
    fs::create_dir_all(dist_path.join("assets"))?;
    Ok(())
}
