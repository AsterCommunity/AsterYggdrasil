# Service Module Ownership

Each service module owns one product domain. Handlers only authenticate, extract parameters, call the service, and return a response; repositories live in `src/db/repository/`; DTOs live in `src/api/dto/`. Service files are facade-first: the shell (`auth_service.rs` or `auth_service/mod.rs`) declares submodules and re-exports the public surface so `crate::services::auth_service::login` paths stay stable across internal refactors.

## Service Modules

| Module | Domain | Submodules |
| --- | --- | --- |
| `services/auth_service/` | Site accounts: registration, setup, login/logout, token rotation and verification, sessions, contact verification, password change, token scope rules | `shared` `session` `login` `registration` `tokens` `contact_verification` `password_change` `token_scope` |
| `services/passkey_service.rs` | Passkey/WebAuthn registration and authentication | single file |
| `services/user_invitation_service.rs` | Invitation creation, revocation, acceptance | single file |
| `services/profile_service/` | Site user profiles and avatars | `avatar*` `info` `profile` `shared` |
| `services/yggdrasil_service/` | Yggdrasil protocol: authenticate/refresh/invalidate/signout, join/hasJoined, profile lookup, metadata, texture property signing | `auth` `session` `token` `login` `profile` `properties` `metadata` `cache` `error` `minecraft_services` |
| `services/texture_service/` | Wardrobe texture lifecycle: upload, processing, binding, deletion, previews, maintenance | `shared` `library` `reports` `moderation` `wardrobe` `deletion` `processing` `query` `preview` `cache` `types` `error` |
| `services/mail/` | Mail outbox dispatch, templates, sending, mail audit | `outbox` `outbox_runtime` `audit` `sender` `template` |
| `services/ban_service.rs` | Player capability bans: list/create/update/revoke, enforcement queries | single file |
| `services/config_service/` | Runtime config service layer above the static config track: get/set/delete, config actions, public config, schema | `actions` `public` `runtime` `schema` `system` |
| `services/admin_user_service.rs` | Admin user CRUD, temporary passwords, scope normalization | single file |
| `services/admin_overview_service.rs` | Admin overview aggregation and system health summary | single file |
| `services/external_auth_service/` | External auth providers, OAuth/OIDC flows, identity linking, password links | `login` `resolution` `providers` `links` `verification` `password_link` `normalize` |
| `services/captcha_service.rs` | Captcha generation and verification | single file |
| `services/health_service.rs` | Readiness and diagnostics health scopes | single file |
| `services/yggdrasil_session_forward_service.rs` | Session-forward server registry and upstream checks | single file |
| `services/yggdrasil_signature.rs` | Yggdrasil signing key generation and rotation | single file |
| `services/texture_preview.rs` | Preview route glue above the render crate | single file |

## State Traits

Service functions take a single capability trait instead of flat bound lists:

```rust
pub async fn login<S>(state: &impl SharedRuntimeState, ...) -> Result<AuthTokenBundle>
```

Capability traits compose via supertraits in `src/runtime/state_traits.rs`:

- Base: `DatabaseRuntimeState`, `AppConfigRuntimeState`, `RuntimeConfigRuntimeState`, `CacheRuntimeState`, `TextureStorageRuntimeState` (texture store), `MetricsRuntimeState`, `AuditRuntimeState`
- `SharedRuntimeState` = all of the above
- `MailRuntimeState: SharedRuntimeState` adds `mail_sender()`
- `TaskRuntimeState: SharedRuntimeState` adds `wake_background_task_dispatcher()`
- `YggdrasilSessionForwardRuntimeState: SharedRuntimeState` adds the forwarded HTTP client

Only genuinely narrow helpers keep two-capability bounds. The concrete `AppState` implements everything; tests use `runtime::test_support::setup_state`.

## Forge Delegation

What the services do **not** own (delegated to Forge crates):

- JWT signing/verification and password hashing — `aster_forge_auth` (`AuthRuntimes` in `RuntimeConfig`)
- Audit buffering and drain — `aster_forge_audit::AuditWriter` (`AuditRuntimeState`)
- Token/session claims model — Forge `TokenClaims` wrapped by product `AuthenticatedClaims`
- Mail outbox state machine — `aster_forge_db::MailOutboxDbStore` + `aster_forge_mail`
- Rate limiting and trusted-proxy IP resolution — `aster_forge_middleware`
- Metrics provider and infra recorder — `aster_forge_metrics`
- Background task claim/lease/retry rules — `aster_forge_tasks`
