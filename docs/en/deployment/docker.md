# Docker Deployment

This page covers production deployment concerns for AsterYggdrasil as a Minecraft skin site and Yggdrasil authentication server.

If you have not mapped the launch checklist yet, start with [Deployment Overview](/en/deployment/). This page focuses on what to mount and configure for Docker.

## Persistent Data

Runtime state inside the container should be mounted at `/data`. Persist at least:

- `config.toml`
- SQLite database, or external database connection config.
- Local texture storage directory.
- Runtime temp and log directories if enabled.

Example static config:

```toml
[server]
host = "0.0.0.0"
port = 3000
start_mode = "primary"
temp_dir = ".tmp"

[database]
url = "sqlite://asteryggdrasil.db?mode=rwc"

[texture_storage]
backend = "local"
local_root = "textures"

[cache]
enabled = true
backend = "memory"
```

If `config.toml` lives at `/data/config.toml`, `local_root = "textures"` resolves to `/data/textures`.

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

Periodic maintenance tasks should run on only one primary node:

```toml
[server]
start_mode = "primary"
```

Other instances should use follower mode to avoid duplicate global cleanup, mail outbox, and background task dispatch work.

## Signing Key

Startup ensures the Yggdrasil signing private key and public key exist. In production, rotate keys through the admin config action instead of editing private key values directly:

```text
POST /api/v1/admin/config/yggdrasil/action
```

After rotation, clients and servers may need to fetch metadata again.

## Backup

Back up at least:

- Database.
- `/data/textures`.
- `data/config.toml` or equivalent secret/config records.

Database and texture storage must be backed up as a set. Restoring only one side can produce missing-object or orphan-object reports from the storage consistency check.
