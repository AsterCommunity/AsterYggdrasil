# Architecture Overview

AsterYggdrasil is built on the [AsterForge](https://github.com/AsterCommunity/AsterForge) runtime and follows the AsterDrive product layout: a thin application shell owning API and business services, plus product crates for models, migrations, and rendering.

## Workspace Crates

| Crate | Owns | Depends on |
| --- | --- | --- |
| `aster_yggdrasil` | Application shell: API routes, services, runtime assembly, static config loader, admin middleware | All product crates and all Forge crates |
| `aster_yggdrasil_model` | SeaORM entities and domain types (`UserRole`, `OperatorScope`, `UserBanScope`, task payloads). Forge trait impls that belong to entities (`TaskRecord`, `ClaimableTaskRecord`) live here | Forge crates, SeaORM. Never depends on the application crate |
| `aster_yggdrasil_migration` | Append-only database migrations and the standalone migration binary | Forge DB migration helpers |
| `aster_yggdrasil_render` | Minecraft skin/cape preview rendering (2D and 3D engines) | `image` only |

Dependency direction is one-way: product crates never depend on the application shell. Service code imports entities through `aster_yggdrasil_model::` full paths — there is no re-export shim.

## Runtime Model

The process runs on `aster_forge_runtime::AsterRuntime::builder()`. Components declare dependencies and shutdown order explicitly (`src/runtime/assembly.rs`):

```
background_tasks -> mail_outbox -> audit_manager -> audit_logs -> database
```

- `src/runtime/assembly.rs` assembles the component graph and runs it.
- `src/runtime/shutdown.rs` is the shutdown tail: after `run()` returns, the metrics provider drains; the Forge#74 audit-recovery hook will attach here when it lands upstream.
- `src/runtime/bootstrap.rs` installs panic hooks, initializes the JWT crypto provider, loads static config, and prepares runtime state.
- `src/runtime/tasks.rs` registers background task consumers and scheduled runtime tasks.
- `src/runtime/test_support.rs` provides shared test fixtures (never hand-assemble `AppStateParts` in tests).

## Config: Three Tracks

Both AsterDrive and AsterYggdrasil use the same three-track model, with Forge owning the shared pieces:

| Track | Owner | Lives in |
| --- | --- | --- |
| Static TOML snapshot | Product (`src/config/`, OnceLock + loader) | `config.toml`, schema in `src/config/schema.rs` |
| Hot-reloadable runtime config | `aster_forge_config::SyncRuntimeConfig` behind `RuntimeConfig` | `system_config` DB table |
| System-config registry | Product definitions (`src/config/definitions.rs`) validated by Forge | seeded by `ensure_defaults` |

Forge owns the schema types for shared sections directly (`aster_forge_cache::CacheConfig`, `aster_forge_config::ConfigSyncConfig`, `aster_forge_logging::LoggingConfig`); product sections (`server`, `auth`, `texture_storage`, `rate_limit`, `network_trust`, `deployment`) are product schemas — a deliberate boundary.

`RuntimeConfig` also hosts process runtimes: `AuthRuntimes` (Forge `JwtSigner`/`JwtVerifier` plus a bounded `PasswordHashRuntime` and a dummy-hash credential) and the config-sync runtime. TTL keys hot-reload; the JWT secret is boot-time only.

## Response Envelope

Serialization is delegated to Forge's validated envelope through a transparent adapter (`src/api/response.rs`): `AsterErrorCode` implements `aster_forge_api::response::ApiResponseCode`, and the wrapper injects the product code into `ForgeApiResponse<T, AsterErrorCode>`. Constructors reject mismatched states at compile time (a success code cannot carry error metadata; a failure code cannot carry data).

Wire shape:

```json
{ "code": "success", "msg": "", "data": { ... } }
{ "code": "auth.credentials_failed", "msg": "...", "error": { "retryable": false } }
```

`error` is omitted when the failure carries no retryability metadata; the per-error redundant `code` field was removed when the envelope moved to Forge. The top-level `code` is the stable public contract consumed by the frontend `errors.json` mapping. Wire-byte contract tests in `src/api/response.rs` lock these shapes.

## Auth Model

See [auth-model.md](auth-model.md) for the identity chain, roles, operator area grants, and player capability bans.

## Related

- [service ownership](service-ownership.md)
- [config model](config-model.md)
- [auth model](auth-model.md)
