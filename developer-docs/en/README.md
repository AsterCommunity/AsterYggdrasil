# AsterYggdrasil Developer Guide

AsterYggdrasil is a self-hosted Minecraft skin site and Yggdrasil authentication server. It is no longer a generic Rust + React template. The current codebase already contains account auth, external auth, passkeys, user profiles, Minecraft profiles, wardrobe textures, authlib-injector-compatible protocol routes, runtime config, background tasks, audit, and an administration frontend.

These docs describe the current implementation and extension contracts. New work should model Minecraft/Yggdrasil concepts directly and must not reintroduce old file-drive, team-sharing, or generic starter assumptions.

## Core Principles

- Read the existing service, repository, DTO, OpenAPI, frontend service, and page patterns before changing code.
- Use product-domain names such as `yggdrasil`, `minecraft_profile`, `texture`, `skin`, `cape`, `wardrobe`, and `authlib_injector`.
- Project and admin APIs use the common response envelope; Yggdrasil protocol endpoints must keep their native protocol response shapes.
- Extend `AsterErrorCode` for new public error reasons. Do not add another client-visible subcode system.
- Add audit records for administrator actions, security-sensitive changes, Minecraft profile/texture changes, Yggdrasil token/session behavior, and runtime lifecycle events.
- Add OpenAPI schemas and generated frontend types whenever an API contract changes.
- Keep migrations append-only once a project has shipped.
- `frontend-panel/` is now product UI. Do not extend it as if it were still a template/demo information architecture.

## Current Product Domains

The backend currently includes:

- Local account setup, registration, login, refresh/logout, session management, and user profile updates.
- Passkey/WebAuthn login and credential management.
- External auth providers, OAuth/OIDC login, account linking, and email verification flows.
- Minecraft profile creation, listing, deletion, and texture binding.
- Wardrobe texture library for skin/cape upload, validation, storage, binding, and deletion.
- Yggdrasil/authlib-injector protocol routes for metadata, authenticate, refresh, validate, invalidate, signout, join, hasJoined, profile, and textures.
- Admin APIs for config, users, Minecraft profiles, audit logs, external auth providers, background tasks, and system info.
- Runtime config, mail outbox, audit, background tasks, metrics, CORS/CSRF, security headers, and rate limiting.

The frontend currently includes:

- `/` public connection page with authlib-injector setup information.
- `/init` first administrator setup.
- `/login` and `/register` account entry with external auth and passkey login.
- `/reset-password` and `/invite/:token` password-reset and invitation registration entry points.
- `/force-password-change` forced password-change entry.
- `/account` account workbench.
- `/account/profiles` Minecraft profile and launcher/texture workflow.
- `/account/wardrobe` current-user texture library.
- `/account/audit` current-user audit log.
- `/account/settings` personal settings, sessions, and passkeys; `/settings/security` remains as a compatibility route for the old security settings entry.
- `/admin/*` config, users, user invitations, Minecraft profiles, external auth, audit, task, and about pages.
- `/tos` and `/privacy` legal pages.

## Backend Extension Path

Use this shape for new backend functionality:

```text
src/entities/                  SeaORM model
migration/                     schema migration
src/db/repository/             database access
src/services/                  business behavior
src/api/dto/                   request/response DTOs
src/api/routes/                HTTP handlers and route registration
src/api/openapi.rs             OpenAPI paths and schemas
tests/                         integration coverage
frontend-panel/src/services/   frontend service wrapper
frontend-panel/src/pages/      UI page when needed
```

Texture blobs go through `src/texture_storage/`; do not bring back the old file-drive model. Yggdrasil behavior belongs under `src/services/yggdrasil_service/`, texture behavior belongs under `src/services/texture_service/`, and handlers should only authenticate, extract HTTP input, call services, and return responses.

## Runtime Startup

Startup is split under `src/runtime/startup/`:

- `common.rs` prepares runtime directories, metrics, database handles, migrations, runtime config, cache, and audit manager.
- `primary.rs` builds primary runtime state.
- `follower.rs` builds follower runtime state.
- `mod.rs` selects by `config.server.start_mode` and records `server_start`.

`server.start_mode = "primary"` runs dispatcher and maintenance loops. `server.start_mode = "follower"` keeps common service state but skips primary-only background tasks, avoiding duplicate mail delivery or duplicate maintenance side effects across nodes.

## Graceful Shutdown

Shutdown is coordinated from `src/main.rs` and `src/runtime/shutdown.rs`:

1. Wait for SIGINT/SIGTERM.
2. Cancel the shared shutdown token.
3. Stop Actix gracefully.
4. Record `server_shutdown`.
5. Stop background tasks with a grace period.
6. Flush audit logs.
7. Close database handles.

When adding long-running workers, they must observe the shutdown token and leave persisted state resumable.

## Background Tasks

The task system lives in `src/services/task_service/` and `src/runtime/tasks.rs`.

The persisted `BackgroundTaskKind` currently remains `system_runtime`. Concrete system work is distinguished by `SystemRuntimeTaskKind`, currently:

- `background-task-dispatch`
- `system-health-check`
- `auth-session-cleanup`
- `external-auth-flow-cleanup`
- `mail-outbox-dispatch`
- `audit-cleanup`
- `task-cleanup`
- `yggdrasil-token-cleanup`
- `yggdrasil-storage-consistency-check`
- `yggdrasil-texture-cleanup`

Admin APIs can list, retry, and clean up tasks; there is no ordinary user task API right now. If a domain task kind is added, define its payload/result types, registry entry, retry classification, initial steps, presentation, visibility rules, and tests together.

Key contracts:

- Claiming is token-fenced with `processing_token`.
- Workers renew leases through heartbeat updates.
- Stale workers must not overwrite a newer lease.
- Graceful shutdown releases processing work back to `retry` without spending retry budget or writing business failure details.
- Task presentation uses stable message codes so the frontend does not parse task payloads or result blobs.
- Add task integration tests for claim, retry, cleanup, and shutdown behavior when changing dispatch semantics.

Mail outbox delivery is also a system runtime task. See [Mail Runtime Extension](./mail-runtime.md) for the concrete extension rules.

Yggdrasil/authlib-injector protocol endpoints, authentication, error shapes, the Minecraft services compatibility layer, and test expectations are covered in [Yggdrasil API Implementation](./yggdrasil-api.md).

## Audit Service

Audit code lives in `src/services/audit_service/`; stable enums live in `src/types/audit.rs`.

Use audit for:

- server start and shutdown
- setup, register, login, logout, refresh token, and session revoke
- passkey register, rename, delete, and login
- admin config changes and config actions
- admin user changes and session revocation
- admin external auth provider create, update, delete, and test
- external auth login, link, and unlink
- admin task retry and cleanup
- mail send and mail delivery failure
- Minecraft profile create and delete
- Minecraft texture upload, bind, and delete
- Yggdrasil authenticate, refresh, invalidate, signout, and join server

Audit entries should include structured details and presentation metadata. Frontend code should display `presentation` first and use raw `details` only as a fallback/debug surface.

Mail audit details, presentation, and tests are covered in [Mail Runtime Extension](./mail-runtime.md).

## API And Errors

Project APIs use the common envelope in `src/api/response.rs`:

```json
{ "code": "success", "msg": "", "data": {} }
```

Client-facing failures expose stable `AsterErrorCode` values. Existing domains cover auth, external auth, mail, config, audit logs, tasks, Minecraft profiles, Minecraft textures, wardrobe, passkeys, avatars, and frontend config.

Yggdrasil/authlib-injector protocol endpoints are the exception: they return protocol-compatible status codes, fields, and error bodies without the project envelope. Keep protocol error mapping around `src/services/yggdrasil_service/error.rs` and `src/api/routes/yggdrasil.rs`; do not pollute the global error system with protocol-only shapes.

When changing API contracts:

1. Update DTOs and route annotations.
2. Register paths and schemas in `src/api/openapi.rs`.
3. Run OpenAPI generation.
4. Regenerate frontend API types.
5. Update frontend service/page code.

Commands:

```bash
cargo test --features openapi --test generate_openapi
cd frontend-panel
bun run generate-api
```

## Frontend Extension Path

The frontend lives in `frontend-panel/`. It is not a marketing site or a template demo.

Use:

- `src/services/` for API wrappers.
- `src/types/api.generated.ts` for generated API types and `src/types/api.ts` for re-exports and aliases.
- `src/lib/presentation.ts` for stable audit/task display formatting.
- `src/pages/account/` for signed-in account pages.
- `src/pages/admin/` for administrator pages.
- `src/components/yggdrasil/` for launcher, Minecraft preview, copy field, and other Yggdrasil/Minecraft components.
- `src/components/account/` for account-domain page composition.
- `src/components/admin/`, `src/components/common/`, and `src/components/layout/` for admin and shared UI composition.

Admin screens should stay dense, predictable, and operational. Profile and wardrobe pages should follow real Minecraft workflows, not template feature cards or file-drive management patterns.

## Testing

Useful commands:

```bash
cargo fmt
cargo check
cargo test
cargo test --features openapi --test generate_openapi

cd frontend-panel
bun run check
bun run test
bun run build
```

Targeted commands used often:

```bash
cargo test --test test_yggdrasil
cargo test --test test_admin_tasks
cargo test --test test_audit
cargo test --test test_auth
cargo test --test test_external_auth
cargo test --test test_database_backends
cargo test mail_template
cargo test texture_service
cargo test task_service::presentation
cargo test shutdown_release_returns_processing_task_to_retry_without_failure_update
```

When changing migrations, repositories, or SQL, at least run the SQLite coverage; for cross-database semantics, run `ASTER_TEST_DATABASE_BACKEND=postgres|mysql cargo test --test test_database_backends`. When changing frontend services or key pages, run `bun run test`; for page workflows, run the relevant Playwright tests.

## Product Boundary Checklist

Before adding a module to AsterYggdrasil, ask:

- Is this needed by the Minecraft skin site, Yggdrasil auth, account security, runtime operations, or admin panel?
- Does the name express the product domain instead of old template, file-drive, or team-sharing concepts?
- Do protocol endpoints preserve authlib-injector/Yggdrasil-compatible response formats?
- Do project APIs keep the common envelope and stable `AsterErrorCode` values?
- Are sensitive values kept out of plaintext storage, logs, audit details, and error messages?
- Do tests cover the relevant service/repository/API risk?
- Does the frontend receive stable presentation data or DTOs instead of parsing backend internals?

If the answer is no, stop and confirm the requirement before adding unrelated product surface. Untested code is irresponsible; stale docs are the same problem wearing a different coat.
