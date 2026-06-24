//! Yggdrasil texture property signing helpers.

use crate::config::yggdrasil::{
    DEFAULT_YGGDRASIL_API_ROOT, RuntimeYggdrasilPolicy, YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
    YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY,
};
use crate::db::repository::system_config_repo;
use crate::errors::{AsterError, Result};
use base64::Engine;
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::signature::{SignatureEncoding, Signer};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sea_orm::ConnectionTrait;
use sha1::Sha1;

const SIGNATURE_KEY_BITS: usize = 4096;

pub fn texture_base_url(policy: &RuntimeYggdrasilPolicy, texture_hash: &str) -> String {
    tracing::debug!(
        texture_hash,
        public_base_url_count = policy.public_base_urls.len(),
        "building yggdrasil texture base url"
    );
    texture_public_url(policy, texture_hash).unwrap_or_else(|| texture_path(texture_hash))
}

pub fn texture_object_url(
    policy: &RuntimeYggdrasilPolicy,
    texture_hash: &str,
    storage_key: &str,
) -> String {
    texture_object_public_url(policy, storage_key)
        .unwrap_or_else(|| texture_base_url(policy, texture_hash))
}

pub fn texture_object_public_url(
    policy: &RuntimeYggdrasilPolicy,
    storage_key: &str,
) -> Option<String> {
    let base_url = policy.texture_public_base_url.as_deref()?;
    // Only uploaded textures have storage keys in S3. Embedded default skins
    // must keep using the hash-based Yggdrasil API URL.
    Some(format!(
        "{base_url}/{}",
        storage_key.trim_start_matches('/')
    ))
}

pub fn texture_public_url(policy: &RuntimeYggdrasilPolicy, texture_hash: &str) -> Option<String> {
    if let Some(base_url) = policy.public_base_urls.first() {
        tracing::debug!(
            texture_hash,
            "using configured yggdrasil texture public url"
        );
        return Some(format!("{base_url}/textures/{texture_hash}"));
    }
    tracing::debug!(texture_hash, "no configured yggdrasil texture public url");
    None
}

pub fn required_texture_object_public_url(
    policy: &RuntimeYggdrasilPolicy,
    texture_hash: &str,
    storage_key: &str,
) -> Result<String> {
    texture_object_public_url(policy, storage_key)
        .or_else(|| texture_public_url(policy, texture_hash))
        .ok_or_else(|| {
            tracing::debug!(
                texture_hash,
                storage_key,
                "required yggdrasil texture object public url is missing"
            );
            AsterError::config_error(
                "public_site_url, yggdrasil_public_base_url, or yggdrasil_texture_public_base_url must be configured before serving Yggdrasil texture properties",
            )
        })
}

pub fn required_texture_public_url(
    policy: &RuntimeYggdrasilPolicy,
    texture_hash: &str,
) -> Result<String> {
    texture_public_url(policy, texture_hash).ok_or_else(|| {
        tracing::debug!(
            texture_hash,
            "required yggdrasil texture public url is missing"
        );
        AsterError::config_error(
            "public_site_url or yggdrasil_public_base_url must be configured before serving Yggdrasil texture properties",
        )
    })
}

fn texture_path(texture_hash: &str) -> String {
    format!("{DEFAULT_YGGDRASIL_API_ROOT}/textures/{texture_hash}")
}

pub fn signature_public_key(policy: &RuntimeYggdrasilPolicy) -> Result<String> {
    if let Some(private_key) = configured_private_key(policy)? {
        tracing::debug!("deriving yggdrasil signature public key from private key");
        return public_key_pem(&private_key);
    }
    tracing::debug!("using configured yggdrasil signature public key");
    Ok(policy.signature_public_key.trim().to_string())
}

pub fn sign_texture_property(
    policy: &RuntimeYggdrasilPolicy,
    value: &str,
) -> Result<Option<String>> {
    let Some(private_key) = configured_private_key(policy)? else {
        tracing::debug!(
            "skipping yggdrasil texture property signature because no private key is configured"
        );
        return Ok(None);
    };
    tracing::debug!(
        value_len = value.len(),
        "signing yggdrasil texture property"
    );
    let signing_key = SigningKey::<Sha1>::new(private_key);
    let signature = signing_key.sign(value.as_bytes());
    Ok(Some(
        base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()),
    ))
}

pub async fn ensure_signature_key<C: ConnectionTrait>(db: &C) -> Result<Option<String>> {
    tracing::debug!("ensuring persistent yggdrasil signature key");
    let existing = system_config_repo::find_by_key(db, YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY).await?;
    if let Some(config) = existing
        .as_ref()
        .filter(|config| !config.value.trim().is_empty())
    {
        sync_public_key_config(db, config.value.trim(), None).await?;
        tracing::debug!("persistent yggdrasil signature key already exists");
        return Ok(None);
    }

    let private_key = generate_private_key_pem(SIGNATURE_KEY_BITS)?;
    let saved = system_config_repo::upsert_with_options(
        db,
        YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
        &private_key,
        None,
        None,
    )
    .await?;
    sync_public_key_config(db, &private_key, None).await?;
    tracing::info!("generated persistent Yggdrasil signature key");
    Ok(Some(saved.value))
}

pub fn generate_private_key_pem(bits: usize) -> Result<String> {
    tracing::debug!(bits, "generating yggdrasil signature private key");
    let mut rng = rand::rng();
    let private_key = RsaPrivateKey::new(&mut rng, bits).map_err(|error| {
        AsterError::internal_error(format!(
            "failed to generate yggdrasil signature key: {error}"
        ))
    })?;
    private_key
        .to_pkcs8_pem(LineEnding::LF)
        .map(|pem| pem.to_string())
        .map_err(|error| {
            AsterError::internal_error(format!(
                "failed to encode yggdrasil signature private key: {error}"
            ))
        })
}

pub fn public_key_pem_from_private_key_pem(private_key_pem: &str) -> Result<String> {
    tracing::debug!("deriving yggdrasil public key pem from private key pem");
    let private_key = parse_private_key_pem(private_key_pem.trim())?;
    public_key_pem(&private_key)
}

pub async fn sync_public_key_config<C: ConnectionTrait>(
    db: &C,
    private_key_pem: &str,
    updated_by: Option<i64>,
) -> Result<String> {
    tracing::debug!(updated_by, "syncing yggdrasil signature public key config");
    let public_key = public_key_pem_from_private_key_pem(private_key_pem)?;
    let existing = system_config_repo::find_by_key(db, YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY).await?;
    if existing
        .as_ref()
        .is_some_and(|config| config.value.trim() == public_key.trim())
    {
        tracing::debug!("yggdrasil signature public key config already matches private key");
        return Ok(public_key);
    }
    system_config_repo::upsert_with_options(
        db,
        YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY,
        &public_key,
        None,
        updated_by,
    )
    .await?;
    tracing::debug!("yggdrasil signature public key config synced");
    Ok(public_key)
}

fn configured_private_key(policy: &RuntimeYggdrasilPolicy) -> Result<Option<RsaPrivateKey>> {
    let pem = policy.signature_private_key.trim();
    if pem.is_empty() {
        return Ok(None);
    }
    tracing::debug!("parsing configured yggdrasil signature private key");
    parse_private_key_pem(pem).map(Some)
}

fn parse_private_key_pem(pem: &str) -> Result<RsaPrivateKey> {
    RsaPrivateKey::from_pkcs8_pem(pem)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem))
        .map_err(|error| {
            AsterError::config_error(format!(
                "invalid yggdrasil signature private key PEM: {error}"
            ))
        })
}

fn public_key_pem(private_key: &RsaPrivateKey) -> Result<String> {
    tracing::debug!("encoding yggdrasil signature public key pem");
    let public_key = RsaPublicKey::from(private_key);
    public_key
        .to_public_key_pem(LineEnding::LF)
        .map(|pem| pem.to_string())
        .map_err(|error| {
            AsterError::config_error(format!(
                "failed to derive yggdrasil signature public key: {error}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::definitions::{
        YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY, YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY,
    };
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    fn policy_with_keys(public_key: &str, private_key: &str) -> RuntimeYggdrasilPolicy {
        RuntimeYggdrasilPolicy {
            server_name: "AsterYggdrasil".to_string(),
            allow_profile_name_login: true,
            allow_skin_upload: true,
            allow_cape_upload: true,
            enable_profile_key: true,
            enable_mojang_anti_features: true,
            token_ttl_days: 15,
            max_active_tokens: 10,
            max_texture_upload_bytes:
                crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_UPLOAD_BYTES,
            max_texture_pixels: crate::config::yggdrasil::DEFAULT_YGGDRASIL_MAX_TEXTURE_PIXELS,
            skin_domains: Vec::new(),
            public_base_urls: Vec::new(),
            texture_public_base_url: None,
            signature_public_key: public_key.to_string(),
            signature_private_key: private_key.to_string(),
        }
    }

    #[test]
    fn derives_public_key_from_configured_private_key() {
        let private_key = generate_private_key_pem(2048).unwrap();
        let policy = policy_with_keys("fallback", &private_key);

        let public_key = signature_public_key(&policy).unwrap();

        assert!(public_key.contains("BEGIN PUBLIC KEY"));
        assert_ne!(public_key, "fallback");
    }

    #[test]
    fn signs_texture_property_when_private_key_exists() {
        let private_key = generate_private_key_pem(2048).unwrap();
        let policy = policy_with_keys("", &private_key);

        let signature = sign_texture_property(&policy, "payload")
            .unwrap()
            .expect("signature should be present");

        assert!(!signature.is_empty());
    }

    #[test]
    fn falls_back_to_configured_public_key_without_private_key() {
        let policy = policy_with_keys("  public-key  ", "");

        assert_eq!(signature_public_key(&policy).unwrap(), "public-key");
        assert!(sign_texture_property(&policy, "payload").unwrap().is_none());
    }

    #[test]
    fn invalid_private_key_returns_config_error() {
        let policy = policy_with_keys("", "not a pem");

        let error = sign_texture_property(&policy, "payload").unwrap_err();

        assert!(matches!(error, AsterError::ConfigError(_)));
        assert!(
            error
                .message()
                .contains("invalid yggdrasil signature private key PEM")
        );
    }

    #[test]
    fn texture_url_uses_first_public_base_url_or_relative_path() {
        let mut policy = policy_with_keys("", "");
        assert_eq!(
            texture_base_url(&policy, "abc"),
            "/api/yggdrasil/textures/abc"
        );
        assert_eq!(
            texture_object_url(&policy, "abc", "ab/abc.png"),
            "/api/yggdrasil/textures/abc"
        );
        let error = required_texture_public_url(&policy, "abc").unwrap_err();
        assert!(matches!(error, AsterError::ConfigError(_)));
        assert!(
            error
                .message()
                .contains("public_site_url or yggdrasil_public_base_url")
        );

        policy.public_base_urls = vec![
            "https://skin.example.test/yggdrasil".to_string(),
            "https://fallback.example.test".to_string(),
        ];

        assert_eq!(
            texture_base_url(&policy, "abc"),
            "https://skin.example.test/yggdrasil/textures/abc"
        );
        assert_eq!(
            texture_object_url(&policy, "abc", "ab/abc.png"),
            "https://skin.example.test/yggdrasil/textures/abc"
        );
        assert_eq!(
            required_texture_public_url(&policy, "abc").unwrap(),
            "https://skin.example.test/yggdrasil/textures/abc"
        );
    }

    #[test]
    fn texture_object_url_prefers_public_storage_base_url_when_configured() {
        let mut policy = policy_with_keys("", "");
        policy.public_base_urls = vec!["https://skin.example.test/yggdrasil".to_string()];
        policy.texture_public_base_url = Some("https://cdn.example.test/textures".to_string());

        assert_eq!(
            texture_object_public_url(&policy, "ab/abc.png").as_deref(),
            Some("https://cdn.example.test/textures/ab/abc.png")
        );
        assert_eq!(
            texture_object_url(&policy, "abc", "ab/abc.png"),
            "https://cdn.example.test/textures/ab/abc.png"
        );
        assert_eq!(
            required_texture_object_public_url(&policy, "abc", "ab/abc.png").unwrap(),
            "https://cdn.example.test/textures/ab/abc.png"
        );
        assert_eq!(
            required_texture_public_url(&policy, "default-skin").unwrap(),
            "https://skin.example.test/yggdrasil/textures/default-skin"
        );
    }

    #[tokio::test]
    async fn ensure_signature_key_generates_when_missing_and_keeps_existing_value() {
        let db = crate::db::connect_with_metrics(
            &crate::config::DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        crate::services::system_config_service::ensure_defaults(&db)
            .await
            .unwrap();

        let generated = ensure_signature_key(&db)
            .await
            .unwrap()
            .expect("missing key should be generated");
        assert!(generated.contains("BEGIN PRIVATE KEY"));
        let second = ensure_signature_key(&db).await.unwrap();
        assert!(second.is_none());

        let stored = crate::entities::system_config::Entity::find()
            .filter(
                crate::entities::system_config::Column::Key.eq(YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY),
            )
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.value, generated);
        let public = crate::entities::system_config::Entity::find()
            .filter(
                crate::entities::system_config::Column::Key.eq(YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY),
            )
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert!(public.value.contains("BEGIN PUBLIC KEY"));
    }

    #[tokio::test]
    async fn ensure_signature_key_replaces_blank_default_value() {
        let db = crate::db::connect_with_metrics(
            &crate::config::DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        crate::services::system_config_service::ensure_defaults(&db)
            .await
            .unwrap();
        crate::db::repository::system_config_repo::upsert_with_options(
            &db,
            YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
            "   ",
            None,
            None,
        )
        .await
        .unwrap();

        let generated = ensure_signature_key(&db)
            .await
            .unwrap()
            .expect("blank key should be replaced");

        assert!(generated.contains("BEGIN PRIVATE KEY"));
        let public = crate::entities::system_config::Entity::find()
            .filter(
                crate::entities::system_config::Column::Key.eq(YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY),
            )
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert!(public.value.contains("BEGIN PUBLIC KEY"));
    }

    #[tokio::test]
    async fn ensure_signature_key_repairs_missing_public_key_for_existing_private_key() {
        let db = crate::db::connect_with_metrics(
            &crate::config::DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                pool_size: 1,
                retry_count: 0,
            },
            aster_forge_metrics::NoopMetrics::arc(),
        )
        .await
        .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        crate::services::system_config_service::ensure_defaults(&db)
            .await
            .unwrap();
        let private_key = generate_private_key_pem(2048).unwrap();
        crate::db::repository::system_config_repo::upsert_with_options(
            &db,
            YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
            &private_key,
            None,
            None,
        )
        .await
        .unwrap();

        let generated = ensure_signature_key(&db).await.unwrap();

        assert!(generated.is_none());
        let public = crate::entities::system_config::Entity::find()
            .filter(
                crate::entities::system_config::Column::Key.eq(YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY),
            )
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            public.value,
            public_key_pem_from_private_key_pem(&private_key).unwrap()
        );
    }
}
