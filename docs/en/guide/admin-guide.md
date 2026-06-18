---
description: AsterYggdrasil administrator guide covering users, profiles, textures, config, audit logs, and background tasks.
---

# Admin Guide

::: tip What this covers
This page follows administrator workflows. It does not replace detailed configuration docs; for config details, read [Config and Keys](/en/guide/configuration).
:::

## What Admins Manage

Administrators mainly maintain six kinds of state:

| Object | What matters |
| --- | --- |
| Users | Who can sign in, who is admin, and whether sessions must be revoked |
| Minecraft profiles | Player names, UUIDs, owners, renames, and deletion |
| Textures | Skin/cape upload, binding, public reads, and orphan cleanup |
| Yggdrasil config | Public URLs, profile-name login, upload switches, token policy |
| Signing keys | Metadata public key, textures property signatures, key rotation |
| Audit and tasks | Admin actions, protocol login behavior, cleanup tasks, retries |

## First Admin Account

On first run, create the initial account through setup:

```text
POST /api/v1/auth/setup
```

The first account becomes administrator. Administrators can manage users, runtime config, Minecraft profiles, audit logs, and background tasks.

## Users and Profiles

Users are site login identities. Minecraft profiles are in-game identities. One user can own multiple profiles.

Common admin APIs:

```text
GET    /api/v1/admin/users
GET    /api/v1/admin/users/{id}
PATCH  /api/v1/admin/users/{id}
POST   /api/v1/admin/users/{id}/sessions/revoke
GET    /api/v1/admin/users/{user_id}/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}
PUT    /api/v1/admin/minecraft-profiles/{uuid}/name
DELETE /api/v1/admin/minecraft-profiles/{uuid}
```

Rename profiles through the API. Direct database edits can desynchronize tokens, launcher caches, server allowlists, texture properties, and audit records.

## Textures

Textures have two layers:

- wardrobe: a user's personal texture library.
- profile texture: the skin/cape slot bound to a Minecraft profile.

Admins can inspect profile-bound textures, delete a slot, or delete texture references by hash:

```text
GET    /api/v1/admin/minecraft-profiles/{uuid}/textures
DELETE /api/v1/admin/minecraft-profiles/{uuid}/textures/{skin|cape}
DELETE /api/v1/admin/minecraft-textures/{hash}
```

Deletion goes through service-layer reference counting. Do not delete storage files directly, or the consistency check will report missing objects.

## Config

Runtime config is changed through the Admin Config API:

```text
GET    /api/v1/admin/config
GET    /api/v1/admin/config/schema
PUT    /api/v1/admin/config/{key}
DELETE /api/v1/admin/config/{key}
POST   /api/v1/admin/config/{key}/action
```

Before launch, check:

- `public_site_url`
- `yggdrasil_public_base_url`
- `yggdrasil_skin_domains`
- `yggdrasil_allow_skin_upload`
- `yggdrasil_allow_cape_upload`
- `yggdrasil_token_ttl_days`
- `yggdrasil_max_active_tokens`

Do not manually edit the signing private key. Rotate it through the action:

```text
rotate_yggdrasil_signature_key
```

## Audit and Tasks

Admin actions, Yggdrasil login behavior, texture upload/deletion, and profile create/delete/rename operations are audited.

```text
GET /api/v1/admin/audit-logs
GET /api/v1/admin/tasks
POST /api/v1/admin/tasks/cleanup
POST /api/v1/admin/tasks/{id}/retry
```

Watch especially:

- `yggdrasil-token-cleanup`
- `yggdrasil-texture-cleanup`
- `yggdrasil-storage-consistency-check`

If the consistency check fails, first verify whether the database or texture directory was edited manually or restored only partially.
