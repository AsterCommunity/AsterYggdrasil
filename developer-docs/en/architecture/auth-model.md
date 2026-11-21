# Auth Model

## Identity Chain

Every request resolves through:

1. **JWT access token** (Forge `aster_forge_auth`): claims carry `sub` (stringified `users.id`), `sid` (session id), `session_version` (generation fence), `token_type` (access/refresh), `restriction` (password-change gate), and `jti` (refresh only).
2. **`AuthenticatedClaims`** (`src/services/auth_service/tokens.rs`): the product view over Forge claims — numeric `user_id`, `session_version`, `jti`, `password_change`, `token_type`. Roles and operator scopes are **not** in the token; they are read from the database per request, so scope and role changes take effect immediately.
3. **Database lookup**: `users.id` by `sub`; `session_version` compared against `users.session_version`; `restriction` must match `users.must_change_password` exactly.

Refresh rotation checks the refresh `jti` against `auth_sessions.current_refresh_jti`, records the previous jti (replay detection), and bumps `last_seen`. `session_version` bumps invalidate every outstanding token (password change, admin disable, admin scope changes).

## Roles

`UserRole` (SeaORM active enum on `users.role`): `Admin`, `Operator`, `User`.

- `Admin` — full backend access; protected from modification by other admins.
- `Operator` — admin access restricted to granted areas.
- `User` — player; only `/api/v1/account/*` and the Yggdrasil protocol surface.

## Operator Area Grants

`OperatorScope`-style grants are stored as namespaced identifiers in `user_operator_scopes` (one row per grant):

```
com.astercosm.yggdrasil.permission.admin.{overview|users|profiles|texture_library|audit|tasks|settings|external_auth}
```

Enforcement is `RequireAdminOrScope` on each admin route group (`src/api/middleware/admin.rs`):

```rust
user.role.is_admin()
    || (user.role == UserRole::Operator && user.operator_scopes.contains(&scope))
```

Planned migration (issue #13): the grants move from bare area names to the full namespaced ids above, carried by a `PermissionRegistry` (descriptor-first, modeled on the AsterACME `ProviderRegistry`) that validates ids at construction and lets the admin UI render unknown entries from descriptors. Role/scope changes bump `session_version`, so outstanding tokens die immediately.

## Player Capability Bans

`UserBanScope` values are player capability identifiers stored in `user_ban` rows with optional expiry:

```
com.astercosm.yggdrasil.capability.yggdrasil.{access|join}
com.astercosm.yggdrasil.capability.profile.manage
com.astercosm.yggdrasil.capability.texture.upload
com.astercosm.yggdrasil.capability.texture_library.interact
```

`ban_service` enforces deny rules at the capability use sites (Yggdrasil endpoints, texture upload, profile management, texture library). Bans apply to players; admin panel access is governed by the grants above — the two vocabularies never intersect (`permission.admin.*` vs `capability.*`).

## Password Handling

Hashing and verification run through `aster_forge_auth::PasswordHashRuntime` (bounded argon2 worker pool) hosted in `RuntimeConfig::AuthRuntimes`. Unknown identifiers at login burn the runtime's dummy hash so response timing does not reveal account existence. `must_change_password` maps to Forge `Restriction::MustChangePassword` on the token and is enforced again per request in `current_user`.

## Protocol Credentials

Yggdrasil tokens are a separate credential family (`yggdrasil_token` table, sha256-hashed access tokens, `selected_profile` binding). They are independent from site JWT sessions and are revoked by profile deletion, admin user deletion, bans (`yggdrasil_access`), and `invalidate`/`signout`.
