//! Static deployment topology configuration.

use serde::{Deserialize, Serialize};

use super::Config;
use crate::errors::{AsterError, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentProfile {
    #[default]
    Single,
    Cluster,
}

impl DeploymentProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Cluster => "cluster",
        }
    }

    pub const fn requires_shared_runtime(self) -> bool {
        matches!(self, Self::Cluster)
    }

    pub const fn allows_instance_local_state(self) -> bool {
        !self.requires_shared_runtime()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeploymentConfig {
    #[serde(default)]
    pub profile: DeploymentProfile,
}

impl DeploymentConfig {
    pub const fn requires_shared_runtime(&self) -> bool {
        self.profile.requires_shared_runtime()
    }

    pub const fn allows_instance_local_state(&self) -> bool {
        self.profile.allows_instance_local_state()
    }
}

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
        config.object_storage.normalized_backend().as_str(),
        "s3" | "minio"
    ) {
        issues.push(
            "cluster profile requires object_storage.backend = \"s3\" or \"minio\"".to_string(),
        );
    }

    issues
}

pub fn validate_static(config: &Config) -> Result<()> {
    let issues = static_issues(config);
    if issues.is_empty() {
        return Ok(());
    }

    Err(AsterError::config_error(format!(
        "invalid deployment profile '{}': {}",
        config.deployment.profile.as_str(),
        issues.join("; ")
    )))
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
    use super::{DeploymentProfile, static_issues, validate_static};
    use crate::config::Config;

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
        validate_static(&config).expect("single profile should accept defaults");
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
        config.object_storage.backend = "s3".to_string();

        validate_static(&config).expect("cluster profile should accept shared dependencies");
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
