//! Configuration loading helpers.

use super::schema::Config;
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::utils::paths::{
    DEFAULT_CONFIG_PATH, resolve_config_relative_path, resolve_config_relative_sqlite_url,
};
use config::{Config as RawConfig, Environment, File, FileFormat};
use secrecy::ExposeSecret;
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, value};

pub fn load() -> Result<Config> {
    let base_dir = std::env::current_dir()
        .map_aster_err_ctx("failed to resolve current dir", AsterError::config_error)?;
    let env_database_url = std::env::var("ASTER__DATABASE__URL").ok();
    load_from_dir(&base_dir, env_database_url.as_deref(), true)
}

/// Resolved configuration file location, mirroring the AsterDrive CLI contract:
/// `--config-file` names the file directly, `--data-dir` points at the
/// instance data directory, and neither falls back to the current directory.
#[derive(Debug, Clone)]
pub struct ConfigLocation {
    path: PathBuf,
}

impl ConfigLocation {
    pub fn resolve(config_file: Option<&Path>, data_dir: Option<&Path>) -> Result<Self> {
        let cwd = std::env::current_dir()
            .map_aster_err_ctx("failed to resolve current dir", AsterError::config_error)?;
        let path = match (config_file, data_dir) {
            (Some(_), Some(_)) => {
                return Err(AsterError::validation_error(
                    "use either --config-file or --data-dir",
                ));
            }
            (Some(path), None) => path.to_path_buf(),
            (None, Some(dir)) => dir.join(DEFAULT_CONFIG_PATH),
            (None, None) => PathBuf::from(DEFAULT_CONFIG_PATH),
        };
        Ok(Self {
            path: if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            },
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Instance root: the directory that relative config values (temp dir,
    /// sqlite url, storage root) resolve against. A config living directly in
    /// a `data` directory treats that directory's parent as the instance root,
    /// so `data/config.toml` and `--config-file data/config.toml` behave the
    /// same.
    pub(crate) fn base_dir(&self) -> &Path {
        let parent = self.path.parent().unwrap_or(Path::new("/"));
        if parent.file_name().is_some_and(|name| name == "data") {
            parent.parent().unwrap_or(parent)
        } else {
            parent
        }
    }
}

/// Loads configuration from a CLI-resolved location, generating the default
/// file on first startup exactly like the implicit path does.
pub fn load_at(location: &ConfigLocation) -> Result<Config> {
    let env_database_url = std::env::var("ASTER__DATABASE__URL").ok();
    load_from(
        location.path(),
        location.base_dir(),
        env_database_url.as_deref(),
        true,
    )
}

pub fn ensure_default_config_for_current_dir(default: &Config) -> Result<PathBuf> {
    let base_dir = std::env::current_dir()
        .map_aster_err_ctx("failed to resolve current dir", AsterError::config_error)?;
    let config_path = base_dir.join(DEFAULT_CONFIG_PATH);
    ensure_default_config_exists(&config_path, default)?;
    Ok(config_path)
}

fn load_from_dir(
    base_dir: &Path,
    env_database_url: Option<&str>,
    include_env: bool,
) -> Result<Config> {
    load_from(
        &base_dir.join(DEFAULT_CONFIG_PATH),
        base_dir,
        env_database_url,
        include_env,
    )
}

fn load_from(
    config_path: &Path,
    base_dir: &Path,
    env_database_url: Option<&str>,
    include_env: bool,
) -> Result<Config> {
    ensure_default_config_exists(config_path, &Config::default())?;
    let stable_defaults_config = ensure_stable_default_config_keys(config_path, None)?;

    let mut builder = RawConfig::builder();
    builder = match stable_defaults_config {
        Some(config_content) => {
            builder.add_source(File::from_str(&config_content, FileFormat::Toml))
        }
        // The format must be explicit: extension-based inference silently
        // drops a `--config-file` whose extension is not `.toml` (the
        // `required(false)` read error swallows it), so the second startup
        // would fall back to defaults after the first one wrote the file.
        None => builder.add_source(
            File::from(config_path)
                .format(FileFormat::Toml)
                .required(false),
        ),
    };

    if include_env {
        builder = builder.add_source(
            Environment::with_prefix("ASTER")
                .separator("__")
                .try_parsing(true),
        );
    } else if let Some(database_url) = env_database_url {
        builder = builder
            .set_override("database.url", database_url)
            .map_aster_err(AsterError::config_error)?;
    }

    let raw = builder.build().map_aster_err(AsterError::config_error)?;

    // Forge runs generated section validation and then the root
    // cross-section validation; keep that sequence out of this file.
    let cfg = aster_forge_config::load_config(|| {
        let mut cfg = raw
            .try_deserialize::<Config>()
            .map_aster_err(AsterError::config_error)?;
        resolve_loaded_paths(base_dir, config_path, &mut cfg)?;
        Ok::<_, AsterError>(cfg)
    })
    .map_aster_err(AsterError::config_error)?;

    eprintln!(
        "[INFO] Configuration loaded from: {}",
        config_path.display()
    );
    Ok(cfg)
}

fn ensure_default_config_exists(config_path: &Path, default: &Config) -> Result<()> {
    if config_path.exists() {
        return Ok(());
    }

    create_default_config(config_path, default)
}

fn create_default_config(config_path: &Path, default: &Config) -> Result<()> {
    let toml_str = toml::to_string_pretty(default).map_aster_err(AsterError::config_error)?;

    let content = format!(
        "# AsterYggdrasil configuration file\n\
         # Generated on first startup; edit as needed.\n\
         # Relative paths are resolved against the directory containing this file (default: ./data).\n\
         # This repository is a reusable template; project-specific services can extend these sections.\n\n\
         {toml_str}"
    );

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).map_aster_err_ctx(
            &format!("failed to create config dir '{}'", parent.display()),
            AsterError::config_error,
        )?;
    }

    std::fs::write(config_path, &content).map_aster_err_ctx(
        &format!("failed to write {}", config_path.display()),
        AsterError::config_error,
    )?;

    eprintln!(
        "[INFO] Default configuration written to: {}",
        config_path.display()
    );
    eprintln!("[INFO] Please review and modify it as needed.");
    Ok(())
}

fn ensure_stable_default_config_keys(
    config_path: &Path,
    current_content: Option<&str>,
) -> Result<Option<String>> {
    let content = match current_content {
        Some(content) => content.to_string(),
        None => std::fs::read_to_string(config_path).map_err(|error| {
            AsterError::config_error(format!("failed to read {}: {error}", config_path.display()))
        })?,
    };

    let mut doc = content.parse::<DocumentMut>().map_err(|error| {
        AsterError::config_error(format!(
            "failed to parse {}: {error}",
            config_path.display()
        ))
    })?;

    let mut changed = false;
    let auth_item = doc
        .as_table_mut()
        .entry("auth")
        .or_insert(Item::Table(Table::new()));
    let Some(auth_table) = auth_item.as_table_mut() else {
        return Err(AsterError::config_error("auth must be a table"));
    };
    if !auth_table.contains_key("mfa_secret_key") {
        auth_table.insert(
            "mfa_secret_key",
            value(
                crate::config::AuthConfig::default()
                    .mfa_secret_key
                    .expose_secret()
                    .to_string(),
            ),
        );
        changed = true;
    }

    if !changed {
        return Ok(None);
    }

    let updated = doc.to_string();
    if let Err(error) = std::fs::write(config_path, &updated) {
        eprintln!(
            "[ERROR] Failed to write generated stable configuration keys to {}: {error}. Fix config file permissions before starting.",
            config_path.display()
        );
        return Err(AsterError::config_error(format!(
            "failed to persist generated stable configuration keys to {}: {error}",
            config_path.display()
        )));
    } else {
        eprintln!(
            "[INFO] Added generated stable configuration keys to: {}",
            config_path.display()
        );
    }
    Ok(Some(updated))
}

fn resolve_loaded_paths(base_dir: &Path, config_path: &Path, cfg: &mut Config) -> Result<()> {
    let config_dir = config_path.parent().unwrap_or(base_dir);

    cfg.server.temp_dir = absolutize_against_instance_root(
        base_dir,
        resolve_config_relative_path(base_dir, config_dir, &cfg.server.temp_dir)?,
    );
    cfg.texture_storage.local_root = absolutize_against_instance_root(
        base_dir,
        resolve_config_relative_path(base_dir, config_dir, &cfg.texture_storage.local_root)?,
    );
    if let aster_forge_db::DatabaseUrl::Url(url) = &cfg.database.url {
        let resolved = resolve_config_relative_sqlite_url(base_dir, config_dir, url)?;
        cfg.database.url = absolutize_sqlite_url(base_dir, resolved).into();
    }
    Ok(())
}

/// Binds a resolved config path to the instance root instead of leaving it as
/// a cwd-relative string: consumers turn these values into filesystem paths
/// directly, so an explicit `--config-file` must not depend on the launch
/// directory. Absolute values (including empty sentinels) pass through.
fn absolutize_against_instance_root(base_dir: &Path, value: String) -> String {
    let path = Path::new(&value);
    if value.is_empty() || path.is_absolute() {
        return value;
    }
    base_dir.join(path).to_string_lossy().into_owned()
}

/// Absolute variant for file-backed sqlite URLs: the path component anchors to
/// the instance root, the query string is preserved verbatim.
fn absolutize_sqlite_url(base_dir: &Path, url: String) -> String {
    let Some(path_and_query) = url.strip_prefix("sqlite://") else {
        return url;
    };
    let (raw_path, query) = path_and_query
        .split_once('?')
        .map_or((path_and_query, None), |(p, q)| (p, Some(q)));
    let absolute = absolutize_against_instance_root(base_dir, raw_path.to_string());
    match query {
        Some(query) => format!("sqlite://{absolute}?{query}"),
        None => format!("sqlite://{absolute}"),
    }
}

#[cfg(test)]
mod tests {
    use super::load_from_dir;
    use crate::utils::paths::{
        DEFAULT_CONFIG_PATH, DEFAULT_SQLITE_DATABASE_PATH, DEFAULT_TEMP_DIR,
    };
    use std::path::{Path, PathBuf};

    fn make_temp_dir(test_name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "asteryggdrasil-config-loader-{test_name}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, content: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    /// Expected loaded value for a config path that resolves under the
    /// instance root: absolute, because loading must bind paths to the
    /// instance root rather than the process working directory.
    fn rooted(base: &Path, relative: &str) -> String {
        base.join(relative).to_string_lossy().into_owned()
    }

    fn rooted_sqlite(base: &Path, relative: &str) -> String {
        format!("sqlite://{}", base.join(relative).to_string_lossy())
    }

    #[test]
    fn config_location_resolves_data_dir_config_file_and_conflicts() {
        use super::ConfigLocation;
        use crate::errors::AsterError;
        use std::path::Path;

        assert!(
            ConfigLocation::resolve(Some(Path::new("a.toml")), Some(Path::new("/srv"))).is_err(),
            "--config-file and --data-dir are mutually exclusive"
        );

        let location = ConfigLocation::resolve(None, Some(Path::new("/srv/ygg"))).unwrap();
        assert_eq!(location.path(), Path::new("/srv/ygg/data/config.toml"));
        assert_eq!(location.base_dir(), Path::new("/srv/ygg"));

        let explicit = ConfigLocation::resolve(Some(Path::new("/srv/ygg/config.toml")), None)
            .expect("explicit config file should resolve");
        assert_eq!(explicit.path(), Path::new("/srv/ygg/config.toml"));
        assert_eq!(explicit.base_dir(), Path::new("/srv/ygg"));

        let data_file = ConfigLocation::resolve(Some(Path::new("/srv/ygg/data/config.toml")), None)
            .expect("data-prefixed explicit file should resolve");
        assert_eq!(
            data_file.base_dir(),
            Path::new("/srv/ygg"),
            "a config living directly in data/ resolves relative values against the instance root"
        );

        let err = ConfigLocation::resolve(Some(Path::new("a.toml")), Some(Path::new("/srv")))
            .expect_err("conflicting inputs should fail");
        assert!(matches!(err, AsterError::ValidationError(_)));
    }

    #[test]
    fn load_at_generates_default_config_under_explicit_data_dir() {
        use super::ConfigLocation;

        let dir = make_temp_dir("explicit-data-dir");
        let location = ConfigLocation::resolve(None, Some(&dir)).unwrap();
        let cfg = super::load_at(&location).unwrap();

        assert!(dir.join(DEFAULT_CONFIG_PATH).exists());
        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &dir,
                "data/asteryggdrasil.db?mode=rwc"
            ))
        );
        assert_eq!(cfg.server.temp_dir, rooted(&dir, DEFAULT_TEMP_DIR));

        // A minimal explicit config also loads; relative values bind to the
        // explicit file's parent, independent of the working directory.
        let instance = make_temp_dir("explicit-config-file");
        write(
            &instance.join("custom.toml"),
            br#"[server]
temp_dir = ".tmp"
"#,
        );
        let explicit = ConfigLocation::resolve(Some(&instance.join("custom.toml")), None).unwrap();
        let cfg = super::load_at(&explicit).unwrap();
        assert_eq!(
            cfg.server.temp_dir,
            rooted(&instance, ".tmp"),
            "relative temp dir binds to the explicit config's parent, not the cwd"
        );

        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(instance);
    }

    #[test]
    fn load_at_reads_the_explicit_file_twice_regardless_of_extension() {
        use super::ConfigLocation;

        let instance = make_temp_dir("explicit-non-toml-extension");
        write(
            &instance.join("instance.conf"),
            br#"[database]
url = "sqlite://custom-instance.db?mode=rwc"

[server]
temp_dir = ".tmp"
"#,
        );
        let location =
            ConfigLocation::resolve(Some(&instance.join("instance.conf")), None).unwrap();

        // First load: the file exists, settings apply.
        let first = super::load_at(&location).unwrap();
        assert_eq!(
            first.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &instance,
                "custom-instance.db?mode=rwc"
            ))
        );

        // Second load (next process start): must read the same file again, not
        // silently fall back to defaults because the extension is unknown.
        let second = super::load_at(&location).unwrap();
        assert_eq!(
            second.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &instance,
                "custom-instance.db?mode=rwc"
            )),
            "a second load of a non-.toml config file must not fall back to defaults"
        );
        assert_eq!(
            second.server.temp_dir,
            rooted(&instance, ".tmp"),
            "relative values anchor to the explicit file's directory"
        );

        let _ = std::fs::remove_dir_all(instance);
    }

    #[test]
    fn load_creates_default_config_under_data_dir() {
        let dir = make_temp_dir("create-default");

        let cfg = load_from_dir(&dir, None, false).unwrap();
        let generated = std::fs::read_to_string(dir.join(DEFAULT_CONFIG_PATH)).unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &dir,
                "data/asteryggdrasil.db?mode=rwc"
            ))
        );
        assert_eq!(cfg.server.temp_dir, rooted(&dir, DEFAULT_TEMP_DIR));
        assert_eq!(cfg.texture_storage.backend, "local");
        assert_eq!(cfg.texture_storage.local_root, rooted(&dir, "data/storage"));
        assert!(cfg.network_trust.trusted_proxies.is_empty());
        assert!(dir.join(DEFAULT_CONFIG_PATH).exists());
        assert!(generated.contains("# AsterYggdrasil configuration file"));
        assert!(generated.contains("[server]"));
        assert!(generated.contains(r#"url = "sqlite://asteryggdrasil.db?mode=rwc""#));
        assert!(generated.contains(r#"temp_dir = ".tmp""#));
        assert!(generated.contains("[object_storage]"));
        assert!(generated.contains(r#"backend = "local""#));
        assert!(generated.contains(r#"local_root = "storage""#));
        assert!(generated.contains("[object_storage.s3]"));
        assert!(generated.contains("[network_trust]"));
        assert!(generated.contains(r#"trusted_proxies = []"#));
        let default_concurrency = aster_forge_auth::DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY;
        assert!(generated.contains(&format!(
            "password_hash_max_concurrency = {default_concurrency}"
        )));
        assert_eq!(cfg.auth.password_hash_max_concurrency, default_concurrency);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_ignores_root_config_file_and_creates_data_config() {
        let dir = make_temp_dir("legacy-config");
        write(
            &dir.join("config.toml"),
            br#"[database]
url = "sqlite://custom.db?mode=rwc"
"#,
        );

        let cfg = load_from_dir(&dir, None, false).unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &dir,
                "data/asteryggdrasil.db?mode=rwc"
            ))
        );
        assert!(dir.join("config.toml").exists());
        assert!(dir.join(DEFAULT_CONFIG_PATH).exists());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_ignores_root_sqlite_database_file_for_default_layout() {
        let dir = make_temp_dir("legacy-db");
        write(&dir.join("asteryggdrasil.db"), b"legacy");

        let cfg = load_from_dir(&dir, None, false).unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &dir,
                "data/asteryggdrasil.db?mode=rwc"
            ))
        );
        assert!(dir.join("asteryggdrasil.db").exists());
        assert!(!dir.join(DEFAULT_SQLITE_DATABASE_PATH).exists());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_keeps_existing_data_prefixed_paths_without_double_data() {
        let dir = make_temp_dir("legacy-data-prefixed-values");
        write(
            &dir.join(DEFAULT_CONFIG_PATH),
            br#"[database]
url = "sqlite://data/asteryggdrasil.db?mode=rwc"

[server]
temp_dir = "data/.tmp"

[object_storage]
local_root = "data/storage"
"#,
        );

        let cfg = load_from_dir(&dir, None, false).unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &dir,
                "data/asteryggdrasil.db?mode=rwc"
            ))
        );
        assert_eq!(cfg.server.temp_dir, rooted(&dir, DEFAULT_TEMP_DIR));
        assert_eq!(cfg.texture_storage.local_root, rooted(&dir, "data/storage"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_resolves_relative_object_storage_root_under_config_dir() {
        let dir = make_temp_dir("object-storage-root-relative");
        write(
            &dir.join(DEFAULT_CONFIG_PATH),
            br#"[object_storage]
backend = "local"
local_root = "custom-textures"
"#,
        );

        let cfg = load_from_dir(&dir, None, false).unwrap();

        assert_eq!(cfg.texture_storage.backend, "local");
        assert_eq!(
            cfg.texture_storage.local_root,
            rooted(&dir, "data/custom-textures")
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_accepts_legacy_texture_storage_section() {
        let dir = make_temp_dir("legacy-texture-storage-section");
        write(
            &dir.join(DEFAULT_CONFIG_PATH),
            br#"[texture_storage]
backend = "local"
local_root = "legacy-textures"
"#,
        );

        let cfg = load_from_dir(&dir, None, false).unwrap();

        assert_eq!(cfg.texture_storage.backend, "local");
        assert_eq!(
            cfg.texture_storage.local_root,
            rooted(&dir, "data/legacy-textures")
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_resolves_relative_database_override_under_data_dir() {
        let dir = make_temp_dir("env-db-url-relative");
        write(&dir.join("asteryggdrasil.db"), b"legacy");

        let cfg = load_from_dir(&dir, Some("sqlite://custom.db?mode=rwc"), false).unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(&dir, "data/custom.db?mode=rwc"))
        );
        assert!(dir.join(DEFAULT_CONFIG_PATH).exists());
        assert!(dir.join("asteryggdrasil.db").exists());
        assert!(!dir.join(DEFAULT_SQLITE_DATABASE_PATH).exists());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_keeps_data_prefixed_database_override_without_double_data() {
        let dir = make_temp_dir("env-db-url-legacy-root-relative");

        let cfg = load_from_dir(&dir, Some("sqlite://data/custom.db?mode=rwc"), false).unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(&dir, "data/custom.db?mode=rwc"))
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_accepts_root_sqlite_database_for_relative_default_override() {
        let dir = make_temp_dir("env-db-url-relative-default");
        write(&dir.join("asteryggdrasil.db"), b"legacy");

        let cfg = load_from_dir(&dir, Some("sqlite://asteryggdrasil.db?mode=rwc"), false).unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &dir,
                "data/asteryggdrasil.db?mode=rwc"
            ))
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_accepts_root_sqlite_database_for_data_prefixed_default_override() {
        let dir = make_temp_dir("env-db-url-data-prefixed-default");
        write(&dir.join("asteryggdrasil.db"), b"legacy");

        let cfg = load_from_dir(
            &dir,
            Some("sqlite://data/asteryggdrasil.db?mode=rwc"),
            false,
        )
        .unwrap();

        assert_eq!(
            cfg.database.url,
            aster_forge_db::DatabaseUrl::Url(rooted_sqlite(
                &dir,
                "data/asteryggdrasil.db?mode=rwc"
            ))
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_rejects_relative_paths_outside_base_dir() {
        let dir = make_temp_dir("path-outside-base-dir");
        write(
            &dir.join(DEFAULT_CONFIG_PATH),
            br#"[server]
temp_dir = "../../outside/.tmp"
"#,
        );

        let err = load_from_dir(&dir, None, false).unwrap_err();
        assert!(err.to_string().contains("outside data base_dir"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_rejects_sqlite_url_outside_base_dir() {
        let dir = make_temp_dir("sqlite-url-outside-base-dir");
        write(
            &dir.join(DEFAULT_CONFIG_PATH),
            br#"[database]
url = "sqlite://../../outside/asteryggdrasil.db?mode=rwc"
"#,
        );

        let err = load_from_dir(&dir, None, false).unwrap_err();
        assert!(err.to_string().contains("outside data base_dir"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_rejects_zero_rate_limit_values() {
        let dir = make_temp_dir("invalid-rate-limit-zero");
        write(
            &dir.join(DEFAULT_CONFIG_PATH),
            br#"[rate_limit]
enabled = true

[rate_limit.auth]
seconds_per_request = 0
burst_size = 5
"#,
        );

        let err = load_from_dir(&dir, None, false).unwrap_err();
        assert!(err.to_string().contains("invalid value: integer `0`"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn load_rejects_zero_rate_limit_burst_size() {
        let dir = make_temp_dir("invalid-rate-limit-burst-zero");
        write(
            &dir.join(DEFAULT_CONFIG_PATH),
            br#"[rate_limit]
enabled = true

[rate_limit.auth]
seconds_per_request = 1
burst_size = 0
"#,
        );

        let err = load_from_dir(&dir, None, false).unwrap_err();
        assert!(err.to_string().contains("invalid value: integer `0`"));

        let _ = std::fs::remove_dir_all(dir);
    }
}
