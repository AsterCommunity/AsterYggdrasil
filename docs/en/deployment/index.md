---
description: AsterYggdrasil deployment overview covering launch checks, public URLs, reverse proxy, persistence, backups, and validation.
---

# Deployment Overview

::: tip What this covers
This page helps you decide what to check before going live. For concrete Docker setup, read [Docker Deployment](/en/deployment/docker).
:::

## Quick Routing

| What you want to do | Where to go |
| --- | --- |
| Run it locally first | [Getting Started](/en/guide/getting-started) |
| Deploy with Docker | [Docker Deployment](/en/deployment/docker) |
| Configure public URLs | [Config and Keys](/en/guide/configuration) |
| Diagnose launcher, skin, or join issues | [Troubleshooting](/en/guide/troubleshooting) |
| Understand texture persistence | [Texture Storage](/en/guide/storage) |

## Check Before Launch

Production deployment is not finished when the container starts. AsterYggdrasil must also be reachable by launchers, Minecraft clients, and servers, so public URLs must work from outside the host.

Before launch, verify:

- The site is reachable through an HTTPS domain.
- `/api/yggdrasil` returns metadata.
- The homepage `/` keeps the `X-Authlib-Injector-API-Location` response header.
- `public_site_url` or `yggdrasil_public_base_url` can produce client-reachable texture URLs.
- `skinDomains` covers the host used by texture URLs.
- The database, `config.toml`, and local texture storage are persisted.

## Recommended Path

1. Complete [Getting Started](/en/guide/getting-started) locally for accounts, profiles, and textures.
2. Prepare a domain name and HTTPS.
3. Mount `/data` using [Docker Deployment](/en/deployment/docker).
4. Configure `public_site_url`; use `yggdrasil_public_base_url` only when you need an advanced path or host override.
5. Log in with a real launcher and join a test server once.
6. Back up the database, `config.toml`, and texture directory.

## Public URLs Matter Most

Texture URLs are embedded in the Yggdrasil `textures` property. Clients must receive absolute URLs, for example:

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

If that address works only from the server host, launchers and Minecraft clients cannot load skins. In that case, check public URL and reverse proxy configuration first.

## Multiple Instance Boundary

These docs cover one primary node and optional follower instances. Periodic maintenance, mail outbox dispatch, audit cleanup, and texture consistency checks should run on only one primary node.

```toml
[server]
start_mode = "primary"
```

Use follower mode for other instances. Do not run global cleanup tasks from multiple instances at the same time.

## What to Back Up

Back up at least:

- Database.
- `config.toml` or equivalent secret/config records.
- Local texture storage directory, usually similar to `data/textures`.

Restore the database and texture directory as a set. Restoring only the database can produce missing objects; restoring only textures can produce orphan objects.
