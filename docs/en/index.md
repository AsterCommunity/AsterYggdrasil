---
layout: home
description: AsterYggdrasil is a self-hosted Minecraft skin site and Yggdrasil/authlib-injector authentication server.

hero:
  name: AsterYggdrasil
  text: Minecraft skin site and auth server
  tagline: Host player accounts, Minecraft profiles, skin/cape textures, and authlib-injector/Yggdrasil integration on your own service.
  actions:
    - theme: brand
      text: Getting Started
      link: /en/guide/getting-started
    - theme: alt
      text: User Guide
      link: /en/guide/user-guide
    - theme: alt
      text: About
      link: /en/guide/about

features:
  - title: Yggdrasil API
    details: "`/api/yggdrasil` serves metadata, authserver, sessionserver, profile lookup, and public texture reads."
    link: /en/guide/yggdrasil-api
  - title: Launcher Login
    details: Reuses site accounts and returns accessToken/clientToken, availableProfiles, and selectedProfile for launchers.
    link: /en/guide/launcher-login
  - title: Minecraft Profiles
    details: Profiles are modeled independently; names are immutable, and deletion clears bindings and revokes related tokens.
    link: /en/guide/profiles
  - title: Texture System
    details: Supports wardrobe textures, profile binding, skin/cape upload, PNG re-encoding, legacy cape handling, and hash URLs.
    link: /en/guide/yggdrasil-textures
  - title: Config and Keys
    details: "Yggdrasil policy, public URLs, skinDomains, and signing keys use runtime config; private keys rotate through actions."
    link: /en/guide/configuration
  - title: Maintenance
    details: Runtime tasks handle token cleanup, orphan texture cleanup, storage consistency checks, audit cleanup, and task artifact cleanup.
    link: /en/guide/audit-tasks
---

AsterYggdrasil is a self-hosted Minecraft skin site and Yggdrasil/authlib-injector authentication server. It is not just a template README with new names: the current backend already includes accounts, profiles, Yggdrasil protocol endpoints, texture processing, runtime config, audit logs, and maintenance tasks.

If this is your first deployment, start with [Getting Started](./guide/getting-started.md). If you are a player or server owner, read the [User Guide](./guide/user-guide.md). If you want the project context and boundaries, read [About AsterYggdrasil](./guide/about.md).

## Main Entrypoint

The protocol root is:

```text
/api/yggdrasil
```

The site homepage returns `X-Authlib-Injector-API-Location: /api/yggdrasil/`, so launchers that support authlib-injector ALI can discover the API from the site URL.

Common public endpoints:

```text
GET  /api/yggdrasil
POST /api/yggdrasil/authserver/authenticate
POST /api/yggdrasil/authserver/refresh
POST /api/yggdrasil/sessionserver/session/minecraft/join
GET  /api/yggdrasil/sessionserver/session/minecraft/hasJoined
GET  /api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}
GET  /api/yggdrasil/textures/{hash}
```

Site and admin APIs live under `/api/v1`, including account APIs, profile management, wardrobe textures, config, audit logs, and background tasks.

## Recommended Reading

1. [About](./guide/about.md): understand who this project is for and where the current limits are.
2. [Getting Started](./guide/getting-started.md): start locally, create the first admin, and verify metadata and texture paths.
3. [User Guide](./guide/user-guide.md): accounts, profiles, textures, and launcher login in normal use.
4. [Yggdrasil API](./guide/yggdrasil-api.md): ALI, metadata, authserver, sessionserver, and protocol errors.
5. [Minecraft Profiles](./guide/profiles.md): profile creation, deletion, immutability, and admin APIs.
6. [Textures](./guide/yggdrasil-textures.md): wardrobe, binding, skin/cape upload, hashes, public reads, and skinDomains.
7. [Config and Keys](./guide/configuration.md): public URLs, runtime config, and signing key rotation.
8. [Audit and Tasks](./guide/audit-tasks.md): admin-visible audit logs, runtime tasks, and maintenance policy.
9. [Docker Deployment](./deployment/docker.md): reverse proxy, HTTPS, persistence, and backups.

## Current Boundaries

- Minecraft profile names cannot be changed after creation. Rename flows should delete and recreate the profile.
- Profile disabling is intentionally left for a future ban system that defines login, join, hasJoined, and texture access semantics together.
- The current production texture storage backend is local. S3/minio config shape is reserved, but the backend still needs implementation.
- The admin frontend is still evolving; docs describe stable backend behavior and deployable semantics first.
