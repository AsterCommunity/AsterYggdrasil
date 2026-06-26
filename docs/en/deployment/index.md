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
| Understand texture and uploaded-avatar persistence | [Object Storage](/en/guide/storage) |

## Check Before Launch

Production deployment is not finished when the container starts. AsterYggdrasil must also be reachable by launchers, Minecraft clients, and servers, so public URLs must work from outside the host.

Before launch, verify:

- The site is reachable through an HTTPS domain.
- `/api/yggdrasil` returns metadata.
- The homepage `/` keeps the `X-Authlib-Injector-API-Location` response header.
- `public_site_url` or `yggdrasil_public_base_url` can produce client-reachable texture URLs.
- `skinDomains` covers the host used by texture URLs.
- The database, `config.toml`, and object storage backend are persisted or recoverable.

## Recommended Path

1. Complete [Getting Started](/en/guide/getting-started) locally for accounts, profiles, and textures.
2. Prepare a domain name and HTTPS.
3. Mount `/data` using [Docker Deployment](/en/deployment/docker).
4. Configure `public_site_url`; use `yggdrasil_public_base_url` only when you need an advanced path or host override.
5. Log in with a real launcher and join a test server once.
6. Back up the database, `config.toml`, and object storage backend.

## Public URLs Matter Most

Texture URLs are embedded in the Yggdrasil `textures` property. Clients must receive absolute URLs, for example:

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

If that address works only from the server host, launchers and Minecraft clients cannot load skins. In that case, check public URL and reverse proxy configuration first.

## Multiple Instance Boundary

Periodic maintenance, mail outbox dispatch, audit cleanup, and texture consistency checks have global side effects. These docs currently cover single-instance deployments.

For multi-instance production deployments, ensure externally that only one instance starts the full background task set. Future high availability should be carried by Forge runtime lease/lock support instead of a Yggdrasil-specific instance-role branch.

## What to Back Up

Back up at least:

- Database.
- `config.toml` or equivalent secret/config records.
- Object storage backend. The local backend usually uses a path similar to `data/storage`; S3/MinIO deployments must back up bucket objects and matching config.

Restore the database and object storage as a set. Restoring only the database can produce missing objects; restoring only objects can produce orphan objects.
