---
description: AsterYggdrasil guide overview, organized by first run, player usage, launcher integration, administrator maintenance, and project reference.
---

# Guides

These docs are organized by what you are trying to do, not by protocol or source-code module names.

If you are new, start with [Getting Started](./getting-started). If the service is already running, jump to the section that matches your role.

## First Run

If you just want to get the service running, read:

- [Getting Started](./getting-started): start the backend, create the first admin, and verify metadata, profiles, and texture paths
- [Deployment Overview](/en/deployment/): launch checks for public URLs, reverse proxy, persistence, backups, and validation
- [Docker Deployment](/en/deployment/docker): production deployment, HTTPS, persistence, and reverse proxy setup
- [Config and Keys](./configuration): public URLs, skinDomains, upload policy, and signing keys

## Player Usage

Once the service opens, normal users should start here:

- [User Guide](./user-guide): accounts, Minecraft profiles, skins/capes, launcher login, and common issues
- [Minecraft Profiles](./profiles): profile names, UUIDs, renames, deletion, and admin inspection
- [Textures](./yggdrasil-textures): wardrobe, profile binding, direct upload, PNG validation, and public reads
- [FAQ](./faq): short answers to common questions

## Launcher and Server Integration

For authlib-injector, launchers, or Minecraft servers, read:

- [Launcher Setup](./launcher-setup): what address, account identifier, and javaagent parameter to use
- [Launcher Login](./launcher-login): authenticate, refresh, selectedProfile, token lifecycle, and join/hasJoined
- [Yggdrasil Forwarding](./yggdrasil-forwarding): point the server at AY and forward Yggdrasil-compatible site, Mojang, or other upstream session checks by priority
- [Yggdrasil API](./yggdrasil-api): ALI, metadata, authserver, sessionserver, profile lookup, texture API, and protocol errors
- [Textures](./yggdrasil-textures): texture property URLs, skinDomains, signatures, and caching concerns
- [Troubleshooting](./troubleshooting): launcher login, server join, skin display, signature verification, and texture 404s

## Administrator Maintenance

Administrators need to keep three layers separate: startup config, runtime config, and externally visible protocol behavior.

- [Admin Guide](./admin-guide): users, profiles, textures, config, audit logs, and tasks
- [Capability Bans](./user-bans): restrict user access to Yggdrasil, profiles, texture upload, and public texture library interactions by scope
- [Config and Keys](./configuration): `config.toml`, `system_config`, public URLs, and signing key rotation
- [Object Storage](./storage): local/S3/MinIO backends, storage paths, public URLs, and consistency checks
- [Audit and Tasks](./audit-tasks): audit coverage, runtime tasks, multiple-instance boundaries, and maintenance guidance
- [Deployment Overview](/en/deployment/) and [Docker Deployment](/en/deployment/docker): persistent data, reverse proxy, trusted proxies, multiple instances, and backups

## Project Reference

For project scope, fit, and current limits, read [About AsterYggdrasil](./about).

Before editing the docs, read [Docs Contributing](./docs-contributing). These docs are for real deployers, players, and server owners, not a prose version of the source tree.
