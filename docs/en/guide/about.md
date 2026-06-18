---
description: Why AsterYggdrasil exists, what it can do today, who it is for, and which limits the current version should not hide.
---

# About AsterYggdrasil

::: tip This is not the integration guide
If you only want to run the service first, start with [Getting Started](./getting-started).

If you want to know whether this project is the right fit for your Minecraft server or skin site, keep reading.
:::

## What It Solves

AsterYggdrasil is a self-hosted Minecraft skin site and Yggdrasil/authlib-injector authentication server.

In practical terms, it lets you host account login, Minecraft profiles, skin/cape textures, launcher authentication, and server join verification on your own service instead of relying on a third-party skin site or a pile of temporary scripts.

The current code already includes:

- Site account setup, registration, login, refresh, logout, and admin bootstrap.
- The `/api/yggdrasil` protocol root with metadata, authserver, sessionserver, profile lookup, and public texture reads.
- Separate Minecraft profile records, so one site account can own multiple profiles.
- Skin/cape upload, PNG re-encoding, legacy cape compatibility, hash-based public reads, and local texture storage.
- Runtime config, signing key rotation, audit logs, and periodic maintenance tasks.

These are implemented backend capabilities, not roadmap promises. The docs should stay within that boundary and avoid presenting unfinished work as available functionality.

## Who It Fits

AsterYggdrasil is worth trying if you run a Minecraft server in the authlib-injector or offline-login ecosystem and want players to manage their own accounts, profile names, skins, and capes.

It also fits deployments where you want to control the database, texture files, signing keys, and backups instead of handing that state to a hosted skin site.

If you are building a custom launcher, skin site, or server panel, it can also be a Rust backend base for Yggdrasil protocol work.

It is also a good fit if you want a single-binary deployment model instead of maintaining a PHP runtime, web server modules, and a stack of extensions.

## Who Should Wait

If you need a polished, finished commercial-grade skin-site frontend today, the current version is not there yet. The backend is the stable part; the admin frontend is still evolving.

If you need multi-node object storage, S3/MinIO texture storage, multi-primary high availability, or a mature ban system, wait or plan to build that work. The production texture storage backend today is local. The S3 config shape is reserved, but the backend is not implemented yet.

If your target is Mojang official online mode, AsterYggdrasil is not designed for that scenario. It targets self-hosted Yggdrasil/authlib-injector integration.

## Current Boundaries

Minecraft profile names support controlled renames through the user or administrator APIs. A controlled rename keeps the UUID, texture bindings, and audit trail, then temporarily invalidates bound Yggdrasil tokens so launchers can refresh into the new name. Do not edit names directly in the database.

Texture uploads accept PNG only. The server re-encodes uploads into sanitized PNG files and hashes the processed bytes. Raw uploads are not kept.

Public texture URLs must be absolute URLs reachable by clients. In production, configure `public_site_url` or `yggdrasil_public_base_url`; otherwise profile texture responses can fail because the service cannot build a public URL.

Do not hand-edit the signing private key in the database. Use the admin config action to rotate it so the service generates and stores a matching RSA private/public key pair.

## Where To Start

- Run it first: [Getting Started](./getting-started).
- Learn the user flow: [User Guide](./user-guide).
- Integrate launchers and servers: [Yggdrasil API](./yggdrasil-api) and [Launcher Login](./launcher-login).
- Deploy it: [Docker Deployment](/en/deployment/docker).
