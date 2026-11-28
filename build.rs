//! Build script: inject build identity and provide fallback frontend assets.

use std::env;
use std::fs;
use std::io;
use std::path::Path;

#[path = "build_support/identity.rs"]
mod identity;

const REVISION_ENV: &str = "ASTER_BUILD_REVISION";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Keep Cargo's default package scan active. It detects new or modified
    // package files while excluding the package target directory, which is
    // required for build identity to notice untracked source changes without
    // rebuilding on its own artifacts.

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    println!("cargo:rustc-env=ASTER_BUILD_TIME={now}");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|error| io::Error::other(format!("missing CARGO_MANIFEST_DIR: {error}")))?;
    let manifest_dir = Path::new(&manifest_dir);
    configure_build_identity(manifest_dir)?;
    let dist_path = manifest_dir.join("frontend-panel/dist");

    if !dist_path.exists() {
        create_fallback_files(&dist_path)?;
    }

    Ok(())
}

fn configure_build_identity(manifest_dir: &Path) -> io::Result<()> {
    let revision_override = match env::var(REVISION_ENV) {
        Ok(value) => Some(value),
        Err(env::VarError::NotPresent) => None,
        Err(env::VarError::NotUnicode(_)) => {
            return Err(io::Error::other(format!(
                "{REVISION_ENV} must contain valid Unicode"
            )));
        }
    };
    let source = identity::source_identity(manifest_dir, revision_override.as_deref())?;
    println!("cargo:rustc-env={REVISION_ENV}={}", source.revision);
    println!(
        "cargo:rustc-env=ASTER_BUILD_DIRTY={}",
        identity::dirty_label(source.dirty)
    );
    let features =
        identity::feature_names(env::vars_os().filter_map(|(key, _)| key.into_string().ok()));
    println!(
        "cargo:rustc-env=ASTER_BUILD_FEATURES={}",
        features.join(",")
    );
    for (cargo_key, aster_key) in [
        ("TARGET", "ASTER_BUILD_TARGET"),
        ("PROFILE", "ASTER_BUILD_PROFILE"),
    ] {
        let value = env::var(cargo_key)
            .map_err(|error| io::Error::other(format!("missing {cargo_key}: {error}")))?;
        if value.is_empty() || value.contains(['\r', '\n']) {
            return Err(io::Error::other(format!(
                "{cargo_key} must be a non-empty single-line value"
            )));
        }
        println!("cargo:rustc-env={aster_key}={value}");
    }
    Ok(())
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
