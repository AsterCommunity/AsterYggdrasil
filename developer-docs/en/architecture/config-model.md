# Config Model

AsterYggdrasil uses the same three-track config model as AsterDrive, with Forge owning the shared pieces.

| Track | Owner | Where |
| --- | --- | --- |
| Static TOML snapshot | Product loader (`src/config/loader.rs`, OnceLock in `src/config/mod.rs`) | `config.toml`, schema in `src/config/schema.rs` |
| Hot-reloadable runtime config | `aster_forge_config::SyncRuntimeConfig` behind `RuntimeConfig` | `system_config` DB table |
| System-config definitions | Product registry (`src/config/definitions.rs`) validated and normalized by Forge | seeded by `ensure_defaults` |

## Static TOML

Schema sections and their owners:

| Section | Schema owner |
| --- | --- |
| `cache` | `aster_forge_cache::CacheConfig` |
| `config_sync` | `aster_forge_config::ConfigSyncConfig` |
| `logging` | `aster_forge_logging::LoggingConfig` |
| `deployment`, `server`, `database`, `auth`, `network_trust`, `rate_limit`, `texture_storage` | Product schema |

Notes:

- `texture_storage` keeps the TOML/wire key `object_storage` via `#[serde(rename = "object_storage", alias = "texture_storage")]` so deployed configuration files keep working; the Rust domain name is texture-first.
- `auth` carries boot-time-only secrets (`jwt_secret`, `mfa_secret_key`); rotating them requires a restart.
- Database credentials accept either a plain URL or an inline table (`base_url`/`username`/`password`) to keep raw credentials out of debug output.

## RuntimeConfig

`RuntimeConfig` (`src/config/runtime_config.rs`) wraps `SyncRuntimeConfig<system_config::Model>` and adds:

- `install_auth_runtimes` — builds `AuthRuntimes` (Forge `JwtSigner`/`JwtVerifier` with the boot-time secret and snapshot TTLs, a bounded `PasswordHashRuntime`, and a dummy-hash credential) after the first reload.
- `apply`/`remove` — hot-reload a single key from the admin config API and Redis config-sync notifications.
- Audit-log allowlist settings — rebuilt on apply/remove of audit keys.

TTL keys (`auth_access_token_ttl_secs`, `auth_refresh_token_ttl_secs`, ...) hot-reload. The JWT secret does not.

## System-Config Registry

`src/config/definitions.rs` declares every hot-reloadable key with defaults, value types, normalization functions, and visibility. `ensure_defaults` seeds missing keys at startup. The admin config API reads/writes through this registry; normalization rejects invalid values before storage.

## Config Sync

Multi-instance deployments enable `config_sync` (`redis` backend). Admin mutations publish reload notifications through `aster_forge_config::ConfigSyncRuntime`; followers apply them into their own `RuntimeConfig` snapshot. The namespace is `aster_yggdrasil.config_reload`.
