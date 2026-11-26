//! Static deployment topology configuration.

use serde::{Deserialize, Serialize};

use super::Config;

/// Topology the server is deployed under; drives which dependencies must be
/// shared across instances.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentProfile {
    /// One process with local dependencies: SQLite, in-memory cache, and local
    /// file storage are all acceptable.
    #[default]
    Single,
    /// Multiple instances sharing state; requires PostgreSQL/MySQL, Redis
    /// cache and config sync, and S3/MinIO object storage.
    Cluster,
}

impl DeploymentProfile {
    /// Lowercase name as written in the config file.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Cluster => "cluster",
        }
    }

    /// Whether the profile demands dependencies shared across instances.
    pub const fn requires_shared_runtime(self) -> bool {
        matches!(self, Self::Cluster)
    }

    /// Whether per-instance local state (memory cache, SQLite, local files) is
    /// acceptable for this profile.
    pub const fn allows_instance_local_state(self) -> bool {
        !self.requires_shared_runtime()
    }
}

/// `[deployment]` section of the static config file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeploymentConfig {
    /// Topology profile; defaults to [`DeploymentProfile::Single`].
    #[serde(default)]
    pub profile: DeploymentProfile,
}

impl DeploymentConfig {
    /// Whether the configured profile demands shared dependencies.
    pub const fn requires_shared_runtime(&self) -> bool {
        self.profile.requires_shared_runtime()
    }

    /// Whether the configured profile permits instance-local state.
    pub const fn allows_instance_local_state(&self) -> bool {
        self.profile.allows_instance_local_state()
    }
}

/// Collects human-readable coherence problems for the configured profile:
/// cluster deployments must point database, cache, config sync, and texture
/// storage at shared backends. Returns an empty vector for valid single-node
/// setups.
pub fn static_issues(config: &Config) -> Vec<String> {
    if !config.deployment.requires_shared_runtime() {
        return Vec::new();
    }

    let mut issues = Vec::new();
    let database_scheme = database_base_url(&config.database.url)
        .split_once(':')
        .map(|(scheme, _)| scheme.trim().to_ascii_lowercase());
    if !matches!(
        database_scheme.as_deref(),
        Some("postgres" | "postgresql" | "mysql")
    ) {
        issues.push("cluster profile requires a shared PostgreSQL or MySQL database".to_string());
    }

    if config.cache.normalized_backend() != "redis" {
        issues.push("cluster profile requires cache.backend = \"redis\"".to_string());
    } else if cache_endpoint_is_blank(&config.cache.endpoint) {
        issues.push(
            "cluster profile requires cache.endpoint when cache.backend is redis".to_string(),
        );
    }

    if !config
        .config_sync
        .backend
        .trim()
        .eq_ignore_ascii_case("redis")
    {
        issues.push("cluster profile requires config_sync.backend = \"redis\"".to_string());
    } else if config_sync_endpoint_is_blank(&config.config_sync.endpoint) {
        issues.push(
            "cluster profile requires config_sync.endpoint when config_sync.backend is redis"
                .to_string(),
        );
    }

    if !matches!(
        config.texture_storage.normalized_backend().as_str(),
        "s3" | "minio"
    ) {
        issues.push(
            "cluster profile requires object_storage.backend = \"s3\" or \"minio\"".to_string(),
        );
    }

    issues
}

impl aster_forge_config::ValidatableConfig for DeploymentConfig {
    /// Profile rules are cross-section (database/cache/config_sync coherence)
    /// and live in the root `ValidatableConfig` implementation.
    fn validate(&self) -> aster_forge_config::Result<()> {
        Ok(())
    }
}

fn database_base_url(url: &aster_forge_db::DatabaseUrl) -> &str {
    match url {
        aster_forge_db::DatabaseUrl::Url(url) => url,
        aster_forge_db::DatabaseUrl::Credentials { base_url, .. } => base_url,
    }
}

fn cache_endpoint_is_blank(endpoint: &aster_forge_cache::CacheEndpoint) -> bool {
    match endpoint {
        aster_forge_cache::CacheEndpoint::Url(url) => url.trim().is_empty(),
        aster_forge_cache::CacheEndpoint::Credentials { base_url, .. } => {
            base_url.trim().is_empty()
        }
    }
}

fn config_sync_endpoint_is_blank(endpoint: &aster_forge_config::ConfigSyncEndpoint) -> bool {
    match endpoint {
        aster_forge_config::ConfigSyncEndpoint::Url(url) => url.trim().is_empty(),
        aster_forge_config::ConfigSyncEndpoint::Credentials { base_url, .. } => {
            base_url.trim().is_empty()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DeploymentProfile, static_issues};
    use crate::config::Config;
    use aster_forge_config::ValidatableConfig as _;

    #[test]
    fn profile_capabilities_are_mapped_in_one_place() {
        assert!(!DeploymentProfile::Single.requires_shared_runtime());
        assert!(DeploymentProfile::Single.allows_instance_local_state());
        assert!(DeploymentProfile::Cluster.requires_shared_runtime());
        assert!(!DeploymentProfile::Cluster.allows_instance_local_state());
    }

    #[test]
    fn single_profile_keeps_default_dependencies() {
        let config = Config::default();
        assert!(static_issues(&config).is_empty());
        config
            .validate()
            .expect("single profile should accept defaults");
    }

    #[test]
    fn cluster_profile_requires_all_shared_dependencies() {
        let mut config = Config::default();
        config.deployment.profile = DeploymentProfile::Cluster;

        let issues = static_issues(&config);
        assert_eq!(issues.len(), 4);
        assert!(issues.iter().any(|issue| issue.contains("PostgreSQL")));
        assert!(issues.iter().any(|issue| issue.contains("cache.backend")));
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("config_sync.backend"))
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("object_storage.backend"))
        );
    }

    #[test]
    fn cluster_profile_accepts_shared_dependencies() {
        let mut config = Config::default();
        config.deployment.profile = DeploymentProfile::Cluster;
        config.database.url = "postgres://aster:secret@db/asteryggdrasil".into();
        config.cache.backend = "redis".to_string();
        config.cache.endpoint = "redis://cache:6379/0".into();
        config.config_sync.backend = "redis".to_string();
        config.config_sync.endpoint = "redis://cache:6379/0".into();
        config.texture_storage.backend = "s3".to_string();

        config
            .validate()
            .expect("cluster profile should accept shared dependencies");
    }

    /// The cross-section pass runs at the root: a cluster profile with local
    /// dependencies must fail validation even when every section is fine.
    #[test]
    fn root_validation_rejects_cluster_profile_with_local_dependencies() {
        let mut config = Config::default();
        config.deployment.profile = DeploymentProfile::Cluster;
        let error = config.validate().unwrap_err();
        assert!(
            error.to_string().contains("invalid deployment profile"),
            "{error}"
        );
    }

    /// A section-level failure blocks the loaded configuration before the
    /// root cross-section pass runs (Forge pipeline order).
    #[test]
    fn section_validation_failure_blocks_the_loaded_config() {
        let config = Config {
            worker: crate::config::worker::WorkerConfig {
                drain_timeout_secs: 86_400,
                cancellation_timeout_secs: 1,
            },
            ..Config::default()
        };
        let error = aster_forge_config::load_config(|| Ok::<_, std::convert::Infallible>(config))
            .unwrap_err();
        assert!(error.to_string().contains("24 hours"), "{error}");
    }

    #[test]
    fn structured_database_credentials_are_supported_and_redacted() {
        let config: Config = toml::from_str(
            r#"
[database]
url = { base_url = "postgres://db:5432/asteryggdrasil", username = "aster", password = "secret" }
"#,
        )
        .expect("structured database credentials should deserialize");

        assert_eq!(
            format!("{:?}", config.database.url),
            "DatabaseUrl::Credentials(<redacted>)"
        );
    }
}
