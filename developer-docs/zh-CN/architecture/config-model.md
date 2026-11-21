# 配置模型

AsterYggdrasil 使用与 AsterDrive 相同的三轨配置模型，Forge 拥有共享部分。

| 轨道 | 归属 | 位置 |
| --- | --- | --- |
| 静态 TOML 快照 | 产品 loader（`src/config/loader.rs`，`src/config/mod.rs` 的 OnceLock） | `config.toml`，schema 在 `src/config/schema.rs` |
| 可热更运行时配置 | `aster_forge_config::SyncRuntimeConfig`，由 `RuntimeConfig` 包装 | `system_config` DB 表 |
| system-config 定义 | 产品注册表（`src/config/definitions.rs`），由 Forge 校验归一化 | `ensure_defaults` 播种 |

## 静态 TOML

各 schema 段及归属：

| 段 | schema 归属 |
| --- | --- |
| `cache` | `aster_forge_cache::CacheConfig` |
| `config_sync` | `aster_forge_config::ConfigSyncConfig` |
| `logging` | `aster_forge_logging::LoggingConfig` |
| `deployment`、`server`、`database`、`auth`、`network_trust`、`rate_limit`、`texture_storage` | 产品 schema |

注意：

- `texture_storage` 通过 `#[serde(rename = "object_storage", alias = "texture_storage")]` 保留 TOML/wire 键 `object_storage`，已部署的配置文件继续可用；Rust 域名以 texture 优先。
- `auth` 承载仅启动期的密钥（`jwt_secret`、`mfa_secret_key`）；轮换需要重启。
- 数据库凭据接受纯 URL 或 inline table（`base_url`/`username`/`password`），避免原始凭据进入 debug 输出。

## RuntimeConfig

`RuntimeConfig`（`src/config/runtime_config.rs`）包装 `SyncRuntimeConfig<system_config::Model>`，并增加：

- `install_auth_runtimes` —— 首次 reload 后构建 `AuthRuntimes`（Forge `JwtSigner`/`JwtVerifier`，使用启动期密钥与快照 TTL；加上有上限的 `PasswordHashRuntime` 和 dummy-hash 凭据）。
- `apply`/`remove` —— 从管理端配置 API 和 Redis config-sync 通知热更单个 key。
- 审计日志 allowlist 设置 —— audit key 的 apply/remove 时重建。

TTL key（`auth_access_token_ttl_secs`、`auth_refresh_token_ttl_secs` 等）热更。JWT secret 不热更。

## System-Config 注册表

`src/config/definitions.rs` 声明每个可热更 key 的默认值、值类型、归一化函数和可见性。`ensure_defaults` 在启动时播种缺失 key。管理端配置 API 通过该注册表读写；归一化在入库前拒绝非法值。

## Config Sync

多实例部署启用 `config_sync`（`redis` backend）。管理端变更通过 `aster_forge_config::ConfigSyncRuntime` 发布 reload 通知；follower 应用到自己的 `RuntimeConfig` 快照。命名空间为 `aster_yggdrasil.config_reload`。
