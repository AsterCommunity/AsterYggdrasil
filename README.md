# AsterYggdrasil

Self-hosted Minecraft skin site and Yggdrasil/authlib-injector authentication server.

> **Fast development version**
>
> The current version is `0.1.0-alpha.1` and is still moving quickly. The backend already has verifiable foundations for accounts, profiles, Yggdrasil protocol endpoints, textures, runtime config, audit logs, and maintenance tasks, but the frontend experience, deployment docs, and some operational capabilities will continue to change. Do not treat this alpha as a long-term stable API; read the docs and plan backups before production use.

- 中文 README: [README.zh.md](README.zh.md)
- Docs home: [docs/index.md](docs/index.md)
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
- Skin/cape upload, PNG re-encoding, legacy cape compatibility, hash-based public reads, and local texture storage.
- Runtime config, Yggdrasil signing key rotation, audit logs, and periodic maintenance tasks.

It is not a file drive, private cloud, game server panel, or generic SaaS template. The product domain is Minecraft/Yggdrasil: accounts, player profiles, skins, capes, launcher login, server join verification, signing keys, texture storage, and admin operations.

## Current Fit

AsterYggdrasil is a good fit when:

- You operate Minecraft servers in the authlib-injector or offline-login ecosystem.
- You want to control player accounts, Minecraft profiles, texture files, the database, signing keys, and backups.
- You need Yggdrasil/authlib-injector-compatible protocol endpoints.
- You want to start with SQLite and local texture storage, then expand the database later if needed.
- You want a single-binary deployment model instead of maintaining a PHP runtime, web server modules, and extension dependencies.
- You want a Rust, Actix Web, SeaORM, React, and Vite codebase for further development.

The current version is not the right fit when:

- You need a polished commercial-grade skin site frontend ready for large long-term user traffic.
- You need S3/MinIO texture storage. The config shape is reserved, but the backend is not implemented yet.
- You need multi-primary high availability, automatic failover, a complete ban system, or enterprise compliance guarantees.
- You need game server management, file storage, WebDAV, WOPI, team sharing, or cloud-drive features.
- You need a public replacement for Mojang official online-mode auth for arbitrary clients.

## Implemented Capabilities

### Accounts and Site API

- `POST /api/v1/auth/setup`
- `POST /api/v1/auth/register`
- `POST /api/v1/auth/login`
- `POST /api/v1/auth/refresh`
- `POST /api/v1/auth/logout`
- `GET /api/v1/auth/me`
- Session management, passkeys, avatars, and external-auth provider foundations.
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
DELETE /api/v1/admin/minecraft-profiles/{uuid}
```

Profile names cannot be changed after creation. To change a name, delete the old profile, create a new one, and log in from the launcher again.

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

### Config, Audit, and Tasks

- `system_config` stores runtime config.
- `POST /api/v1/admin/config/yggdrasil/action` rotates the Yggdrasil signing key.
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
- Back up the database, `data/config.toml`, and the local texture storage directory.
- Treat the Yggdrasil signing private key as sensitive config. Rotate it through the config action instead of editing database rows directly.
- In multi-instance deployments, only one instance should use `start_mode = "primary"` for periodic maintenance.
- The production texture storage backend today is local. `s3` / `minio` backends are reserved but not implemented yet.

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
src/texture_storage/         Minecraft texture storage abstraction
src/types/                   Shared enums and DB wrapper types
src/utils/                   crypto, ID, path, number, email, RAII helpers
migration/                   SeaORM migration crate
api-docs-macros/             OpenAPI helper macros
frontend-panel/              React + Vite admin frontend
developer-docs/              Developer notes
docs/                        User/deployment docs site
tests/                       Integration tests and OpenAPI export tests
tmp/authlib-injector/wiki/   authlib-injector/Yggdrasil reference entrypoint
```

## License

MIT. See [LICENSE](LICENSE).
