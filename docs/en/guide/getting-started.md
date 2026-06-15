# Getting Started

This page does one thing: run AsterYggdrasil locally and verify that Yggdrasil metadata, accounts, profiles, and texture paths actually work.

If you are preparing a production deployment, run through this page first, then continue to [Docker Deployment](/en/deployment/docker). Starting directly in production is possible, but validating the local flow first saves time.

## Requirements

- Rust stable toolchain.
- SQLite. The default config uses local SQLite and needs no separate database service.
- Bun. Required only for the docs site or the admin frontend.

## 1. Start the Backend

```bash
cargo run
```

The first startup creates runtime files, the SQLite database, and the default static config:

```text
data/config.toml
```

Default address:

```text
http://127.0.0.1:3000
```

Health check endpoints:

```text
GET /health
GET /health/ready
```

## 2. Create the First Admin

AsterYggdrasil includes local authentication and admin APIs. On first run, create the admin through setup:

```text
POST /api/v1/auth/setup
```

Normal login, registration, and refresh use:

```text
POST /api/v1/auth/login
POST /api/v1/auth/register
POST /api/v1/auth/refresh
```

The first created account becomes the administrator. The admin configures public URLs, Yggdrasil policy, signing keys, audit logs, and background tasks.

Admin capabilities include:

- View and update runtime config.
- Execute the Yggdrasil signing key rotation action.
- View audit logs.
- View and retry background tasks.
- Manage users, Minecraft profiles, and textures.

## 3. Verify Yggdrasil Metadata

After startup, request:

```text
GET /api/yggdrasil
GET /api/yggdrasil/
```

The response is authlib-injector metadata and does not use the project API envelope. It should include:

- `meta.serverName`
- `meta.implementationName`
- `meta.implementationVersion`
- `meta.feature.non_email_login`
- `skinDomains`
- `signaturePublickey`

The site homepage `/` returns:

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

Launchers that support ALI can use the site URL and discover the real Yggdrasil API root automatically. When deploying behind a reverse proxy, do not strip this response header.

## 4. Create a Minecraft Profile

After logging into the site account, users can create Minecraft profiles. A profile is the player identity seen by launchers and servers:

```text
POST /api/v1/profiles/minecraft
GET  /api/v1/profiles/minecraft
```

Profile names cannot be changed after creation. To change a name, delete the profile and create a new one.

## 5. Upload and Bind Textures

Current users can upload textures to their wardrobe first, then bind them to a profile:

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/skin
POST   /api/v1/wardrobe/textures/cape
PUT    /api/v1/profiles/minecraft/{uuid}/textures/skin
PUT    /api/v1/profiles/minecraft/{uuid}/textures/cape
```

Launchers and compatible tools can also use the Yggdrasil texture endpoints to write directly to a profile:


```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/skin
PUT    /api/yggdrasil/api/user/profile/{uuid}/cape
DELETE /api/yggdrasil/api/user/profile/{uuid}/skin
DELETE /api/yggdrasil/api/user/profile/{uuid}/cape
```

Upload requires a Yggdrasil access token. The server validates token state, profile ownership, upload switches, MIME type, PNG dimensions, and then re-encodes the image as a safe PNG.

Public read:

```text
GET /api/yggdrasil/textures/{hash}
```

## 6. Configure Public URLs

You can skip this for local testing. Once real launchers, servers, or external users need to reach the service, configure public URLs.

For normal deployments, configure:

```text
public_site_url
```

If the Yggdrasil API is exposed under a different path or host, configure the advanced override:

```text
yggdrasil_public_base_url
```

Without a usable public URL, the service cannot build absolute texture URLs in the textures property, and skins will fail to load.

## 7. Local Docs Site

```bash
cd docs
bun install
bun run docs:dev
```

Build docs:

```bash
cd docs
bun run docs:build
```

## Next

- [Yggdrasil API](./yggdrasil-api.md)
- [Launcher Login](./launcher-login.md)
- [Minecraft Profiles](./profiles.md)
- [Textures](./yggdrasil-textures.md)
- [Config and Keys](./configuration.md)
