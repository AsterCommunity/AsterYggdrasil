# AsterYggdrasil

Self-hosted Minecraft skin site and Yggdrasil/authlib-injector authentication server.

> **Fast development version**
>
> The current target version is `0.1.0-beta.1` and development is still moving quickly. The project now has implemented flows for accounts, scoped operators, captcha-protected public auth flows, Minecraft profiles, Yggdrasil protocol endpoints, wardrobe textures, the public texture library, runtime config, audit logs, and maintenance tasks. Do not treat this beta as a long-term stable API; read the docs and plan backups before production use.

- 中文 README: [README.zh.md](README.zh.md)
- Documentation site: [yggdrasil.astercosm.com](https://yggdrasil.astercosm.com/)
- Docs source: [docs/index.md](docs/index.md)
- Getting started: [docs/guide/getting-started.md](docs/guide/getting-started.md)
- User guide: [docs/guide/user-guide.md](docs/guide/user-guide.md)
- Docker deployment: [docs/deployment/docker.md](docs/deployment/docker.md)
- Example config: [config.example.toml](config.example.toml)
- Developer docs: [developer-docs/README.md](developer-docs/README.md)

## What is AsterYggdrasil?

AsterYggdrasil puts the identity and texture flow needed by private Minecraft deployments into one self-hosted service:

- Site account registration, login, refresh, logout, and first-admin setup.
- Separate Minecraft profile records, with multiple profiles per site account.
- Yggdrasil/authlib-injector protocol root at `/api/yggdrasil`.
- Launcher login plus token refresh/validate/invalidate/signout.
- Minecraft join / hasJoined / profile lookup.
- Skin/cape upload, PNG re-encoding, legacy cape compatibility, hash-based public reads, and local/S3/MinIO object storage.
- Wardrobe texture management plus public texture library submission, review, publishing, tags, copying, reporting, and unpublishing.
- Admin and scoped operator workflows for users, profiles, texture library moderation, config, audit, tasks, and external authentication.
- Runtime config, Yggdrasil signing key rotation, audit logs, and periodic maintenance tasks.

AsterYggdrasil focuses on the Minecraft/Yggdrasil domain: accounts, player profiles, skins, capes, launcher login, server join verification, signing keys, texture storage, and admin operations.

## Current Fit

AsterYggdrasil is a good fit when:

- You operate Minecraft servers in the authlib-injector or offline-login ecosystem.
- You want to control player accounts, Minecraft profiles, texture files, the database, signing keys, and backups.
- You need Yggdrasil/authlib-injector-compatible protocol endpoints.
- You want to start with SQLite and local object storage, then expand the database or storage backend later if needed.
- You want a single-binary deployment model instead of maintaining a PHP runtime, web server modules, and extension dependencies.
- You want a Rust, Actix Web, SeaORM, React, and Vite codebase for further development.

The current version is not the right fit when:

- You need a finished commercial-grade operations panel ready for large long-term public traffic without your own validation.
- You need client-side presigned uploads directly to S3/MinIO. Uploads are server-side streaming only.
- You need multi-primary high availability, automatic failover, a complete ban system, or enterprise compliance guarantees.
- You need full Minecraft game server lifecycle management such as world hosting, plugin management, console access, or scheduled server operations.
- You need a public replacement for Mojang official online-mode auth for arbitrary clients.

## Implemented Capabilities

### Accounts and Site API

- `POST /api/v1/auth/setup`
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/refresh`
- `POST /api/v1/auth/logout`
- `GET /api/v1/auth/me`
- Session management, passkeys, avatars, external-auth providers, scoped operators, and visual captcha policy.
- Project APIs use the standard envelope and stable `AsterErrorCode` values.

### Yggdrasil Protocol API

Protocol root:

```text
/api/yggdrasil
```

Common endpoints:

```text
GET  /api/yggdrasil
POST /api/yggdrasil/authserver/authenticate
POST /api/yggdrasil/authserver/refresh
POST /api/yggdrasil/authserver/validate
POST /api/yggdrasil/authserver/invalidate
POST /api/yggdrasil/authserver/signout

POST /api/yggdrasil/sessionserver/session/minecraft/join
GET  /api/yggdrasil/sessionserver/session/minecraft/hasJoined
GET  /api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}

POST /api/yggdrasil/api/profiles/minecraft
GET  /api/yggdrasil/textures/{hash}
```

Protocol endpoints return Yggdrasil/authlib-injector-compatible responses and do not use the `/api/v1` project envelope.

The site homepage `/` returns:

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

Launchers that support API Location Indication can discover the protocol root from the site root URL.

### Minecraft Profiles

Current-user APIs:

```text
GET    /api/v1/profiles/minecraft
POST   /api/v1/profiles/minecraft
GET    /api/v1/profiles/minecraft/{uuid}/textures
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}
```

Admin APIs:

```text
GET    /api/v1/admin/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}
GET    /api/v1/admin/users/{user_id}/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}/textures
DELETE /api/v1/admin/minecraft-profiles/{uuid}/textures/{skin|cape}
DELETE /api/v1/admin/minecraft-textures/{hash}
PUT    /api/v1/admin/minecraft-profiles/{uuid}/name
DELETE /api/v1/admin/minecraft-profiles/{uuid}
```

Profile names support controlled renames through the user or administrator API. A rename keeps the UUID, texture bindings, and audit chain, then temporarily invalidates Yggdrasil tokens bound to that profile so launchers can refresh into the new name.

### Textures

Site users can upload wardrobe textures first, then bind them to profiles:

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
DELETE /api/v1/wardrobe/textures/{texture_id}
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
```

Launchers and compatible tools can use the Yggdrasil texture API:

```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
DELETE /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
GET    /api/yggdrasil/textures/{hash}
```

Uploads must be PNG files. The server validates MIME type, dimensions, upload policy, and profile ownership, then re-encodes the image as a sanitized PNG and hashes the processed bytes.

Public texture library APIs let users publish and reuse wardrobe textures:

```text
GET    /api/v1/texture-library/tags
GET    /api/v1/texture-library/textures
GET    /api/v1/texture-library/textures/{texture_id}
POST   /api/v1/texture-library/textures/{texture_id}/copy
POST   /api/v1/texture-library/textures/{texture_id}/reports
POST   /api/v1/wardrobe/textures/{texture_id}/library-submission
DELETE /api/v1/wardrobe/textures/{texture_id}/library-submission
```

Admins and scoped texture-library operators can review submissions, manage tags, handle reports, and unpublish public textures:

```text
GET  /api/v1/admin/texture-library/textures
POST /api/v1/admin/texture-library/textures/{texture_id}/approve
POST /api/v1/admin/texture-library/textures/{texture_id}/reject
POST /api/v1/admin/texture-library/textures/{texture_id}/unpublish

GET  /api/v1/admin/texture-library/reports
POST /api/v1/admin/texture-library/reports/{report_id}/accept
POST /api/v1/admin/texture-library/reports/{report_id}/reject
```

### Config, Audit, and Tasks

- `system_config` stores runtime config.
- `texture_library_enabled` and `texture_library_review_required` control the public texture library.
- `auth_captcha_*` keys control visual captcha policy and preview.
- `POST /api/v1/admin/config/yggdrasil/action` rotates the Yggdrasil signing key.
- `POST /api/v1/admin/config/auth_captcha/action` previews captcha rendering.
- `GET /api/v1/admin/audit-logs` lists audit logs.
- `GET /api/v1/admin/tasks`, `POST /api/v1/admin/tasks/{id}/retry`, and `POST /api/v1/admin/tasks/cleanup` manage background tasks.
- Runtime tasks cover token cleanup, texture object cleanup, storage consistency checks, audit cleanup, and task artifact cleanup.

## Quick Start

### Run From Source

```bash
git clone https://github.com/AsterCommunity/AsterYggdrasil.git
cd AsterYggdrasil

cd frontend-panel
bun install
bun run build
cd ..

cargo run
```

Default address:

```text
http://127.0.0.1:3000
```

On first startup, the service generates `data/config.toml`, creates the default SQLite database, runs migrations, and initializes runtime config.

Health checks:

```text
GET /health
GET /health/ready
```

### Docker Trial

For a local HTTP trial:

```bash
mkdir -p ./data

docker run -d \
  --name asteryggdrasil \
  -p 3000:3000 \
  -e ASTER__SERVER__HOST=0.0.0.0 \
  -e ASTER__AUTH__BOOTSTRAP_INSECURE_COOKIES=true \
  -e 'ASTER__DATABASE__URL=sqlite:///data/asteryggdrasil.db?mode=rwc' \
  -v "$(pwd)/data:/data" \
  ghcr.io/astercommunity/asteryggdrasil:latest
```

`ASTER__AUTH__BOOTSTRAP_INSECURE_COOKIES=true` is only for local or internal HTTP testing. Production deployments should use HTTPS and keep secure cookies enabled.

See [docs/deployment/docker.md](docs/deployment/docker.md) for full deployment notes.

## Production Notes

- Do not expose `:3000` directly to the public Internet. Put it behind a reverse proxy for HTTPS, upload limits, and real client IP handling.
- Configure `public_site_url` or `yggdrasil_public_base_url` before real use; otherwise textures properties cannot include client-reachable absolute URLs.
- Back up the database, `data/config.toml`, and the object storage backend or local object storage directory.
- Treat the Yggdrasil signing private key as sensitive config. Rotate it through the config action instead of editing database rows directly.
- In multi-instance deployments, only one instance should use `start_mode = "primary"` for periodic maintenance.
- The production object storage backend can be local, S3, or MinIO. Textures and uploaded avatars use the same backend.
- For publicly readable S3/MinIO buckets or CDNs, `yggdrasil_texture_public_base_url` can make uploaded texture URLs point directly at object storage while default skins still use the Yggdrasil API.

## Common Development Commands

```bash
# Backend
cargo fmt
cargo check
cargo test
cargo test --features openapi --test generate_openapi
cargo test --features metrics
cargo run

# Frontend
cd frontend-panel
bun install
bun run dev
bun run build
bun run check
bun run test
bun run test:e2e

# Docs
cd docs
bun install
bun run docs:dev
bun run docs:build
```

## Project Structure

```text
src/                         Rust backend
src/api/                     Routes, DTOs, OpenAPI registration, middleware, response helpers
src/cache/                   Cache trait plus memory/noop/Redis implementations
src/config/                  Static config, runtime config definitions, config normalization
src/db/                      Database connections, retry helpers, transactions, repositories
src/entities/                SeaORM entities
src/runtime/                 AppState, startup, shutdown, logging, background task loops
src/services/                auth, external auth, config, mail, audit, task, health, Yggdrasil, texture
src/object_storage/          Object storage abstraction used by textures and uploaded avatars
src/types/                   Shared enums and DB wrapper types
src/utils/                   crypto, ID, path, number, email, RAII helpers
migration/                   SeaORM migration crate
api-docs-macros/             OpenAPI helper macros
frontend-panel/              React + Vite product frontend and admin UI
developer-docs/              Developer notes
docs/                        User/deployment docs site
tests/                       Integration tests and OpenAPI export tests
tmp/authlib-injector/wiki/   authlib-injector/Yggdrasil reference entrypoint
```

## License

MIT. See [LICENSE](LICENSE).
