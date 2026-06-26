# Docker Deployment

This page covers production deployment concerns for AsterYggdrasil as a Minecraft skin site and Yggdrasil authentication server.

If you have not mapped the launch checklist yet, start with [Deployment Overview](/en/deployment/). This page focuses on what to mount and configure for Docker.

## Persistent Data

Runtime state inside the container should be mounted at `/data`. Persist at least:

- `config.toml`
- SQLite database, or external database connection config.
- Local object storage directory.
- Runtime temp and log directories if enabled.

Example static config:

```toml
[server]
host = "0.0.0.0"
port = 3000
temp_dir = ".tmp"

[database]
url = "sqlite://asteryggdrasil.db?mode=rwc"

[object_storage]
backend = "local"
local_root = "storage"

[cache]
backend = "memory"
```

If `config.toml` lives at `/data/config.toml`, `local_root = "storage"` resolves to `/data/storage`. Textures and uploaded user avatars are written through this object storage directory.

If you use S3 or MinIO, objects are not written to `/data/storage`, but the database and `config.toml` still must be persisted. Example:

```toml
[object_storage]
backend = "s3"

[object_storage.s3]
endpoint = "https://s3.example.com"
region = "auto"
bucket = "asteryggdrasil"
base_path = "production"
access_key_id = "..."
secret_access_key = "..."
force_path_style = false
```

Textures and uploaded avatars use the same object storage backend. S3/MinIO uploads are performed by server-side streaming and do not require browser presigned uploads.

## Reverse Proxy

Production deployments usually expose HTTPS through Nginx, Caddy, or Traefik. Make sure the external path matches runtime config:

```text
https://skin.example.com/api/yggdrasil
```

matching runtime config:

```json
yggdrasil_public_base_url = ["https://skin.example.com/api/yggdrasil"]
yggdrasil_skin_domains = ["skin.example.com"]
```

authlib-injector verifies that texture URL hosts are covered by `skinDomains`. If public base URL and skinDomains do not match, launchers or servers may reject textures.

If the S3 bucket or a frontend CDN is publicly readable, you can additionally configure runtime `yggdrasil_texture_public_base_url` so uploaded texture URLs point directly at object storage/CDN. In that mode, the bucket/CDN must allow anonymous `GET`/`HEAD` reads from the site origin; default skins still use the Yggdrasil API.

## ALI

The site homepage returns:

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

Do not strip this response header in the reverse proxy. It lets users enter the site root in launchers and have the launcher discover the Yggdrasil API automatically.

## trusted proxies

When running behind a reverse proxy, configure trusted proxies so the service does not trust forged forwarded headers from clients.

```toml
[network_trust]
trusted_proxies = ["127.0.0.1"]
```

Use the real source address or CIDR used between the proxy and the application.

## Multiple Instances

Periodic maintenance, mail outbox dispatch, audit cleanup, and texture consistency checks have global side effects. The current Docker guide covers single-instance deployments.

For multi-instance production deployments, ensure externally that only one instance starts the full background task set. Future high availability should be carried by Forge runtime lease/lock support.

## Signing Key

Startup ensures the Yggdrasil signing private key and public key exist. In production, rotate keys through the admin config action instead of editing private key values directly:

```text
POST /api/v1/admin/config/yggdrasil/action
```

After rotation, clients and servers may need to fetch metadata again.

## Backup

Back up at least:

- Database.
- `/data/storage`.
- `data/config.toml` or equivalent secret/config records.

Database and object storage must be backed up as a set. Restoring only one side can produce missing-object or orphan-object reports from the storage consistency check.
