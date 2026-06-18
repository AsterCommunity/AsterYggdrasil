---
layout: home
description: AsterYggdrasil documentation home, organized by getting started, player usage, launcher integration, administrator configuration, and deployment.

hero:
  name: AsterYggdrasil
  text: Documentation
  tagline: Self-hosted Minecraft skin site and Yggdrasil/authlib-injector authentication server, documented from local setup to real launcher integration.
  actions:
    - theme: brand
      text: Getting Started
      link: /en/guide/getting-started
    - theme: alt
      text: Guides
      link: /en/guide/
    - theme: alt
      text: Docker Deployment
      link: /en/deployment/

features:
  - title: First Run
    details: Start the service, create the first admin, then verify metadata, public URLs, and texture reads.
    link: /en/guide/getting-started
  - title: Player Usage
    details: Follow the real flow through accounts, Minecraft profiles, wardrobe textures, skins/capes, and launcher login.
    link: /en/guide/user-guide
  - title: Launcher and Server Integration
    details: Starts with what to enter in launchers, then covers ALI, authserver, sessionserver, texture URLs, and signature verification.
    link: /en/guide/launcher-setup
  - title: Admin Configuration
    details: Separate users, profiles, textures, static config, runtime config, audit logs, and background tasks.
    link: /en/guide/admin-guide
  - title: Texture System
    details: Covers wardrobe, profile binding, direct Yggdrasil upload, PNG re-encoding, hash URLs, and local storage.
    link: /en/guide/yggdrasil-textures
---

## First, Understand It

AsterYggdrasil is a self-hosted Minecraft skin site and Yggdrasil/authlib-injector authentication server. It lets you host site accounts, Minecraft profiles, skin/cape textures, launcher login, and server join verification on your own service.

The current codebase already includes account auth, external auth, Minecraft profiles, wardrobe textures, Yggdrasil protocol endpoints, texture processing, runtime config, audit logs, and maintenance tasks. The docs describe those implemented capabilities instead of presenting future roadmap items as available features.

## Choose Your Path

### I just want to run it

Start with [Getting Started](/en/guide/getting-started). It walks through starting the backend, creating the first admin, checking `/api/yggdrasil` metadata, creating a profile, and verifying texture upload and public reads.

For production, continue to [Deployment Overview](/en/deployment/) and [Docker Deployment](/en/deployment/docker). Public URLs, reverse proxy headers, cached signing keys, and texture directory backups are the common failure points.

### I am a player

Open [Guides](/en/guide/) first. Normal users should read the [User Guide](/en/guide/user-guide), which follows account login, Minecraft profile creation, skin/cape management, launcher login, and common issues.

For profile names, UUIDs, renames, or deletion, go to [Minecraft Profiles](/en/guide/profiles). For skins and capes, go to [Textures](/en/guide/yggdrasil-textures).

### I need launcher or server integration

Read [Launcher Setup](/en/guide/launcher-setup), then [Launcher Login](/en/guide/launcher-login) and [Yggdrasil API](/en/guide/yggdrasil-api). Launchers normally need this protocol root:

```text
https://your-domain.example/api/yggdrasil
```

Launchers that support API Location Indication may use the site root instead. The homepage returns:

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

### I administer an instance

Start with [Admin Guide](/en/guide/admin-guide) and [Config and Keys](/en/guide/configuration). Confirm `public_site_url`, `yggdrasil_public_base_url`, `yggdrasil_skin_domains`, and signing key rotation. Then read [Audit and Tasks](/en/guide/audit-tasks) for token cleanup, texture consistency checks, and admin-visible audit logs.

For launcher login, server join, skin display, or signature verification issues, go directly to [Troubleshooting](/en/guide/troubleshooting). Short questions are covered in the [FAQ](/en/guide/faq).

Texture persistence is covered in [Texture Storage](/en/guide/storage). The current production storage backend is local. The S3/MinIO config shape is reserved, but the backend is not implemented yet.

### I want to edit the docs

Read [Docs Contributing](/en/guide/docs-contributing) first. These docs are for real users, not a directory index for source modules. Before adding a page, ask: what task is the reader trying to complete?

## Current Boundaries

- Minecraft profile names support controlled renames through user or administrator APIs. Do not edit names directly in the database.
- Deleting a profile handles texture bindings, reference counts, related Yggdrasil tokens, and audit records.
- Yggdrasil protocol endpoints return protocol-shaped responses; site and admin APIs use `{ "code": "success", "msg": "", "data": ... }`.
- The current production texture storage backend is local. S3/MinIO is only a reserved config shape.
- The admin frontend is still evolving; docs prioritize stable backend behavior and deployable semantics.
