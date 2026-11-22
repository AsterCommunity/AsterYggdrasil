# Service Module Ownership

The service layer is organized by **domain plane** (mirroring AsterDrive's `auth/` / `user/` / `ops/` structure): module names carry no `_service` suffix, and `services/mod.rs` declares only the top-level domains. Each domain's `mod.rs` declares submodules and re-exports the public surface; handlers only authenticate, extract parameters, call the service, and return a response; repositories live in `src/db/repository/`; DTOs live in `src/api/dto/`.

## Domain Planes

```text
services/
├── auth/       Auth plane: local (site accounts), external (federated login), passkey, captcha
├── user/       User domain: profile (profiles/avatars), admin (admin user management), ban, invitation
├── ops/        Operations plane: admin (overview), audit, config, health, task
├── texture/    Texture domain
├── yggdrasil/  Protocol domain: protocol (protocol flows), session_forward, signature
└── mail/       Mail domain
```

## Service Modules

| Module | Domain | Submodules |
| --- | --- | --- |
| `services/auth/local/` | Site accounts: registration, setup, login/logout, token rotation and verification, sessions, contact verification, password change, token scope rules | `shared` `session` `login` `registration` `tokens` `contact_verification` `password_change` `token_scope` |
| `services/auth/external/` | External auth providers, OAuth/OIDC flows, identity linking, password links | `login` `resolution` `providers` `links` `verification` `password_link` `normalize` |
| `services/auth/passkey.rs` | Passkey/WebAuthn registration and authentication | single file |
| `services/auth/captcha.rs` | Captcha generation and verification | single file |
| `services/user/profile/` | Site user profiles and avatars | `avatar*` `info` `details` `shared` |
| `services/user/admin.rs` | Admin user CRUD, temporary passwords, scope normalization | single file |
| `services/user/ban.rs` | Player capability bans: list/create/update/revoke, enforcement queries | single file |
| `services/user/invitation.rs` | Invitation creation, revocation, acceptance | single file |
| `services/yggdrasil/protocol/` | Yggdrasil protocol: authenticate/refresh/invalidate/signout, join/hasJoined, profile lookup, metadata, texture property signing | `auth` `session` `token` `login` `profile` `properties` `metadata` `cache` `error` `minecraft_services` |
| `services/yggdrasil/session_forward.rs` | Session-forward server registry and upstream checks | single file |
| `services/yggdrasil/signature.rs` | Yggdrasil signing key generation and rotation | single file |
| `services/texture/` | Wardrobe texture lifecycle: upload, processing, binding, deletion, previews, maintenance | `shared` `library` `reports` `moderation` `wardrobe` `deletion` `processing` `query` `preview` `cache` `types` `error` |
| `services/mail/` | Mail outbox dispatch, templates, sending, mail audit | `outbox` `outbox_runtime` `audit` `sender` `template` |
| `services/ops/admin/overview.rs` | Admin overview aggregation and system health summary | single file |
| `services/ops/audit/` | Audit input, context, query, and presentation | `context` `details` `filters` `manager` `models` `presentation` `query` `runtime` |
| `services/ops/config/` | Runtime config service layer above the static config track: get/set/delete, config actions, public config, schema | `actions` `public` `runtime` `schema` `system` |
| `services/ops/health/` | Readiness and diagnostics health scopes | single file |
| `services/ops/task/` | Background task creation, dispatch, lease, retry, presentation | `dispatch` `spec` and others |

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
- Outbound HTTP — `aster_yggdrasil_outbound` (single construction point via `Profile` + `ClientConfig`)
