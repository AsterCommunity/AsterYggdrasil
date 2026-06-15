use super::*;
use crate::db::repository::{
    minecraft_profile_repo, minecraft_profile_texture_repo, minecraft_texture_repo, user_repo,
};
use crate::runtime::AppState;
use crate::types::UserRole;
use sha2::Digest;
use std::io::Cursor;
use std::sync::Arc;

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

async fn test_state(texture_root: String) -> AppState {
    let db_cfg = crate::config::DatabaseConfig {
        url: "sqlite::memory:".to_string(),
        pool_size: 1,
        retry_count: 0,
    };
    let db = crate::db::connect_with_metrics(&db_cfg, crate::metrics_core::NoopMetrics::arc())
        .await
        .expect("texture cleanup test database should connect");
    migration::Migrator::up(&db, None)
        .await
        .expect("texture cleanup test migrations should run");
    crate::services::system_config_service::ensure_defaults(&db)
        .await
        .expect("texture cleanup test defaults should seed");

    let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
    runtime_config
        .reload(&db)
        .await
        .expect("texture cleanup runtime config should reload");
    let config = Arc::new(crate::config::Config {
        database: db_cfg,
        texture_storage: crate::config::TextureStorageConfig {
            backend: "local".to_string(),
            local_root: texture_root,
            ..Default::default()
        },
        cache: crate::config::CacheConfig {
            enabled: false,
            ..Default::default()
        },
        ..Default::default()
    });
    let cache = crate::cache::create_cache(&config.cache).await;
    let texture_storage = crate::texture_storage::create_texture_storage(&config.texture_storage)
        .expect("texture cleanup storage should initialize");
    let yggdrasil_rate_limiter = crate::runtime::AppState::new_yggdrasil_rate_limiter(&config);

    AppState {
        db_handles: crate::db::DbHandles::single(db),
        config,
        runtime_config,
        cache,
        texture_storage,
        mail_sender: crate::services::mail_service::memory_sender(),
        metrics: crate::metrics_core::NoopMetrics::arc(),
        yggdrasil_rate_limiter,
        background_task_dispatch_wakeup:
            crate::runtime::AppState::new_background_task_dispatch_wakeup(),
    }
}

async fn create_profile_texture_asset(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
    texture_type: MinecraftTextureType,
    hash: &str,
    storage_key: &str,
    is_wardrobe_item: bool,
) {
    let texture = minecraft_texture_repo::create(
        state.writer_db(),
        minecraft_texture_repo::CreateMinecraftTexture {
            user_id,
            texture_type,
            hash,
            storage_key,
            mime_type: "image/png",
            file_size: 1,
            width: 64,
            height: 64,
            texture_model: MinecraftTextureModel::Default,
            visibility: MinecraftTextureVisibility::Private,
            is_wardrobe_item,
        },
    )
    .await
    .unwrap();
    minecraft_profile_texture_repo::upsert_for_profile(
        state.writer_db(),
        minecraft_profile_texture_repo::UpsertMinecraftProfileTexture {
            profile_id,
            texture_id: texture.id,
            texture_type,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn cleanup_orphan_texture_blobs_deletes_unreferenced_storage_keys_only() {
    let root = std::env::temp_dir().join(format!(
        "asteryggdrasil-orphan-textures-{}",
        uuid::Uuid::new_v4()
    ));
    let state = test_state(root.to_string_lossy().to_string()).await;
    let referenced_key = "aa/referenced.png";
    let orphan_key = "bb/orphan.png";
    let referenced_path = root.join(referenced_key);
    let orphan_path = root.join(orphan_key);
    tokio::fs::create_dir_all(referenced_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::create_dir_all(orphan_path.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&referenced_path, png(64, 64))
        .await
        .unwrap();
    tokio::fs::write(&orphan_path, png(64, 64)).await.unwrap();

    let user = user_repo::create(
        state.writer_db(),
        "texture-cleanup-user",
        "texture-cleanup@example.com",
        "password-hash",
        UserRole::User,
    )
    .await
    .unwrap();
    let profile = minecraft_profile_repo::create(
        state.writer_db(),
        user.id,
        "1234567890abcdef1234567890abcdef",
        "CleanupSkin",
        MinecraftTextureModel::Default,
        "skin,cape",
    )
    .await
    .unwrap();
    create_profile_texture_asset(
        &state,
        user.id,
        profile.id,
        MinecraftTextureType::Skin,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        referenced_key,
        false,
    )
    .await;

    let result = cleanup_orphan_texture_blobs(&state).await.unwrap();

    assert_eq!(
        result,
        OrphanTextureCleanupResult {
            scanned: 2,
            deleted: 1,
            skipped: 1,
        }
    );
    assert!(tokio::fs::try_exists(referenced_path).await.unwrap());
    assert!(!tokio::fs::try_exists(orphan_path).await.unwrap());
    if let Err(error) = tokio::fs::remove_dir_all(root).await {
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}

#[tokio::test]
async fn check_texture_storage_consistency_reports_missing_and_hash_mismatch() {
    let root = std::env::temp_dir().join(format!(
        "asteryggdrasil-texture-consistency-{}",
        uuid::Uuid::new_v4()
    ));
    let state = test_state(root.to_string_lossy().to_string()).await;
    let valid_bytes = png(64, 64);
    let valid_hash = hex::encode(sha2::Sha256::digest(&valid_bytes));
    let valid_key = "aa/valid.png";
    let mismatch_key = "bb/mismatch.png";
    let missing_key = "cc/missing.png";
    tokio::fs::create_dir_all(root.join("aa")).await.unwrap();
    tokio::fs::create_dir_all(root.join("bb")).await.unwrap();
    tokio::fs::write(root.join(valid_key), &valid_bytes)
        .await
        .unwrap();
    tokio::fs::write(root.join(mismatch_key), png(64, 64))
        .await
        .unwrap();

    let user = user_repo::create(
        state.writer_db(),
        "texture-consistency-user",
        "texture-consistency@example.com",
        "password-hash",
        UserRole::User,
    )
    .await
    .unwrap();
    let profile = minecraft_profile_repo::create(
        state.writer_db(),
        user.id,
        "abcdefabcdefabcdefabcdefabcdefab",
        "ConsistencySkin",
        MinecraftTextureModel::Default,
        "skin,cape",
    )
    .await
    .unwrap();
    for (texture_type, hash, storage_key) in [
        (MinecraftTextureType::Skin, valid_hash.as_str(), valid_key),
        (
            MinecraftTextureType::Cape,
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            mismatch_key,
        ),
    ] {
        create_profile_texture_asset(
            &state,
            user.id,
            profile.id,
            texture_type,
            hash,
            storage_key,
            false,
        )
        .await;
    }
    let second_profile = minecraft_profile_repo::create(
        state.writer_db(),
        user.id,
        "fedcbafedcbafedcbafedcbafedcbafe",
        "MissingTexture",
        MinecraftTextureModel::Default,
        "skin,cape",
    )
    .await
    .unwrap();
    create_profile_texture_asset(
        &state,
        user.id,
        second_profile.id,
        MinecraftTextureType::Skin,
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        missing_key,
        false,
    )
    .await;

    let report = check_texture_storage_consistency(&state).await.unwrap();

    assert_eq!(report.checked, 3);
    assert_eq!(report.missing, 1);
    assert_eq!(report.hash_mismatched, 1);
    assert_eq!(report.issues.len(), 2);
    assert!(report.issues.iter().any(|issue| issue.kind
        == TextureStorageConsistencyIssueKind::MissingObject
        && issue.storage_key == missing_key));
    assert!(report.issues.iter().any(|issue| issue.kind
        == TextureStorageConsistencyIssueKind::HashMismatch
        && issue.storage_key == mismatch_key));
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn register_bound_textures_in_wardrobe_converts_and_deduplicates_existing_bindings() {
    let root = std::env::temp_dir().join(format!(
        "asteryggdrasil-wardrobe-register-{}",
        uuid::Uuid::new_v4()
    ));
    let state = test_state(root.to_string_lossy().to_string()).await;
    let user = user_repo::create(
        state.writer_db(),
        "wardrobe-register-user",
        "wardrobe-register@example.com",
        "password-hash",
        UserRole::User,
    )
    .await
    .unwrap();
    let profile_a = minecraft_profile_repo::create(
        state.writer_db(),
        user.id,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "RegisterOne",
        MinecraftTextureModel::Default,
        "skin,cape",
    )
    .await
    .unwrap();
    let profile_b = minecraft_profile_repo::create(
        state.writer_db(),
        user.id,
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "RegisterTwo",
        MinecraftTextureModel::Default,
        "skin,cape",
    )
    .await
    .unwrap();
    let shared_hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    create_profile_texture_asset(
        &state,
        user.id,
        profile_a.id,
        MinecraftTextureType::Skin,
        shared_hash,
        "aa/shared.png",
        false,
    )
    .await;
    create_profile_texture_asset(
        &state,
        user.id,
        profile_b.id,
        MinecraftTextureType::Skin,
        shared_hash,
        "bb/shared.png",
        false,
    )
    .await;

    let result = register_bound_textures_in_wardrobe(&state).await.unwrap();
    assert_eq!(result.scanned_bindings, 2);
    assert_eq!(result.converted_textures, 1);
    assert_eq!(result.rebound_bindings, 1);
    assert_eq!(result.removed_duplicate_textures, 1);

    let wardrobe_count = minecraft_texture_repo::list_by_hash(state.writer_db(), shared_hash)
        .await
        .unwrap()
        .into_iter()
        .filter(|texture| texture.is_wardrobe_item)
        .count();
    assert_eq!(wardrobe_count, 1);
    let bindings = minecraft_profile_texture_repo::list_by_hash(state.reader_db(), shared_hash)
        .await
        .unwrap();
    assert_eq!(bindings.len(), 2);
    assert!(bindings.iter().all(|item| item.texture.is_wardrobe_item));
    if let Err(error) = tokio::fs::remove_dir_all(root).await {
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}
