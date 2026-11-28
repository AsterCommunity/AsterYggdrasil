# User Capability Ban Implementation

Capability bans are user-level, scope-level business restrictions. They control whether a user can use Yggdrasil, Minecraft profiles, texture upload, and public texture library interaction capabilities. They do not replace account status, operator scopes, or the Minecraft profile lifecycle.

User-facing documentation lives in `docs/en/guide/user-bans.md`. This document covers code boundaries and extension contracts.

## Domain Model

Core types live in `src/types/user.rs`:

- `UserBanScope`
- `UserBanStatus`
- `UserBanEventType`
- `UserBanScopes`

Current `UserBanScope` values:

| Scope | Code meaning |
| --- | --- |
| `yggdrasil_access` | Yggdrasil authenticate and token use |
| `yggdrasil_join` | join/hasJoined and Minecraft services multiplayer state |
| `minecraft_profile_manage` | Profile create, delete, rename, and profile texture binding rewrites |
| `texture_upload` | Wardrobe upload, direct Yggdrasil upload, texture metadata updates, profile texture bind/delete |
| `texture_library_interact` | User-side public texture library submit, withdraw, report, and similar interaction actions |

Do not pass scopes as raw `String` values between service and repository layers. Add new capabilities by extending `UserBanScope`, then update service checks, frontend labels, OpenAPI, tests, and this document.

## Storage

Migration: `migration/src/m20260620_000002_user_bans.rs`.

Entities:

- `src/entities/user_ban.rs`
- `src/entities/user_ban_event.rs`

Tables:

- `user_bans`
- `user_ban_events`

`user_bans.scopes` and event `previous_scopes` / `next_scopes` columns are TEXT containing a JSON array, for example:

```json
["texture_upload","minecraft_profile_manage"]
```

Code must read and write these columns through the `UserBanScopes` wrapper. `UserBanScopes::new(Vec<UserBanScope>)` sorts, deduplicates, and rejects empty arrays. The database stores JSON-array text, but the business layer must not allow empty scope sets or bypass the wrapper with raw strings.

## Service Boundary

Business logic lives in `src/services/ban_service.rs`.

Important inputs and outputs:

- `CreateUserBanInput.scopes: Vec<UserBanScope>`
- `UpdateUserBanInput.scopes: Option<Vec<UserBanScope>>`
- `ListUserBansInput.scope: Option<UserBanScope>`
- `UserBanInfo.scopes: Vec<UserBanScope>`
- `UserBanEventInfo.previous_scopes / next_scopes`

List filtering still uses one `scope` query parameter, meaning "return records whose `scopes` array contains this scope". Create, update, and response bodies use the plural `scopes` array. Do not restore the old singular `scope` request field.

A ban is effective when:

- `status == active`
- `revoked_at IS NULL`
- `starts_at <= now`
- `expires_at IS NULL OR expires_at > now`

The same user cannot have two currently effective bans covering the same scope. `reject_duplicate_effective_scopes` checks each requested scope and excludes the current ban id during updates.

## API

Admin API routes live in `src/api/routes/admin/user_bans.rs`:

```text
GET    /api/v1/admin/user-bans
GET    /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/users/{user_id}/bans
PATCH  /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/user-bans/{ban_id}/revoke
GET    /api/v1/admin/user-bans/{ban_id}/events
```

Current-user lookup lives in `src/api/routes/account.rs`:

```text
GET /api/v1/account/bans
```

DTOs live in:

- `src/api/dto/admin.rs`
- `src/api/dto/account.rs`

Create and update DTOs use `#[serde(deny_unknown_fields)]`. The old singular `scope` request field should be rejected. `scopes: []` should return a validation error.

Admin responses include internal fields such as `admin_note`, `created_by_user_id`, `revoked_by_user_id`, and `revoke_note`. Current-user responses go through `AccountUserBanInfo` and remove those internal fields.

## Enforcement Points

For new project API capabilities, prefer:

```rust
ban_service::ensure_user_not_banned(state, user_id, UserBanScope::...)
```

When a protocol-compatible error body is required, use `is_user_banned` and map to the protocol error.

Current important checks:

- Yggdrasil authenticate and token validate/refresh: `yggdrasil_access`
- Yggdrasil join, hasJoined, Minecraft services privileges/player attributes: `yggdrasil_join`
- Minecraft profile create/delete/rename: `minecraft_profile_manage`
- Wardrobe upload and Yggdrasil texture upload: `texture_upload`
- Profile texture bind/delete: `texture_upload` + `minecraft_profile_manage`
- Public texture library user interactions: `texture_library_interact`

Upload handlers must check `texture_upload` before reading multipart files. Keep the second service-layer check as defense in depth for non-HTTP call paths.

## Error Mapping

Project APIs blocked by a ban return:

```text
403 user_ban.forbidden
```

Stable error codes live in `src/api/error_code.rs`:

- `user_ban.not_found`
- `user_ban.already_active`
- `user_ban.not_active`
- `user_ban.duration_invalid`
- `user_ban.reason_invalid`
- `user_ban.forbidden`

Texture service has `TextureErrorKind::UserBanForbidden`. Project API routes must map that back to `AsterErrorCode::UserBanForbidden`, not to `minecraft_texture.upload_disabled`.

Yggdrasil protocol endpoints must keep protocol response shapes and must not use the project envelope. Ban errors usually map to invalid token/credentials or `ForbiddenOperationException` at the protocol surface, handled around `src/services/yggdrasil_service/*` and `src/services/texture_service/error.rs`.

## Audit And Events

Admin operations write audit entries:

- `AdminCreateUserBan`
- `AdminUpdateUserBan`
- `AdminRevokeUserBan`

The audit entity type is `UserBan`. Details include the `scopes` array, target user, status, reason, notes, and time range. Frontend audit presentation should display `scopes`; do not read only an old `scope` field.

`user_ban_events` stores create, update, and revoke transitions. Scope transitions use `previous_scopes` and `next_scopes`.

## Frontend Contract

API types come from the generated OpenAPI file. After backend DTO changes, run:

```bash
cargo nextest run --features openapi --test generate_openapi
cd frontend-panel && bun run generate-api
```

The admin user detail capability-ban section should create and update records with `scopes` arrays. The account overview defaults to showing only currently effective bans; revoked and expired records should not appear in the default user-facing panel.

Public texture library browsing must not be hidden or blocked by `texture_library_interact`.

## Tests

Related tests:

- `tests/test_user_bans.rs`
- `tests/test_yggdrasil.rs`
- `frontend-panel/src/components/admin/admin-users-page/UserDetailBanSection.test.tsx`
- `frontend-panel/src/pages/account/AccountOverviewPage.test.tsx`
- `frontend-panel/src/services/apiServices.test.ts`

Changes should cover at least:

- Create, update, revoke, events, and audit.
- Non-empty `scopes`, sorting/deduplication, and rejection of the old `scope` field.
- One record with multiple scopes blocks every included scope.
- Duplicate effective bans are rejected when any scope overlaps.
- Current-user lookup does not leak internal admin fields.
- Texture upload bans fail before file reads and texture record creation.
- Project APIs return `user_ban.forbidden`; Yggdrasil protocol APIs keep protocol error bodies.

Recommended verification:

```bash
cargo nextest run --test test_user_bans
cargo nextest run --test test_yggdrasil user_ban_texture_upload_scope_blocks_wardrobe_upload_before_temp_or_record_write
cd frontend-panel && bun run test -- src/components/admin/admin-users-page/UserDetailBanSection.test.tsx src/pages/account/AccountOverviewPage.test.tsx src/services/apiServices.test.ts
cd frontend-panel && bun run check
```
