# Minecraft Profiles

A Minecraft profile is the player's identity in the Yggdrasil protocol. Site accounts handle login; profiles handle the player name, UUID, and texture properties used by launchers and servers.

AsterYggdrasil models profiles separately instead of mixing them into the user table. One site account can own multiple Minecraft profiles, and administrators can inspect the account-to-profile relationship clearly.

## Create and List

Current-user APIs:

```text
GET    /api/v1/profiles/minecraft
POST   /api/v1/profiles/minecraft
GET    /api/v1/profiles/minecraft/{uuid}/textures
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}
```

Admin APIs:

```text
GET    /api/v1/admin/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}
GET    /api/v1/admin/users/{user_id}/minecraft-profiles
GET    /api/v1/admin/minecraft-profiles/{uuid}/textures
DELETE /api/v1/admin/minecraft-profiles/{uuid}/textures/{skin|cape}
DELETE /api/v1/admin/minecraft-textures/{hash}
DELETE /api/v1/admin/minecraft-profiles/{uuid}
```

The admin list supports filtering by name, uuid, and user-related conditions.

## Relation to Wardrobe

Users can upload skin/cape files to their wardrobe first:

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
DELETE /api/v1/wardrobe/textures/{texture_id}
```

Then they can bind one wardrobe texture to a profile's skin or cape slot.

This does not conflict with direct Yggdrasil upload. Direct upload writes the processed texture to the target profile; wardrobe behaves more like a personal texture library for reuse and management.

## Name Rules

Profile names cannot be changed after creation. Do not update them directly in the database; launcher caches, tokens, audit logs, server allowlists, and texture properties can all be affected.

To change a name:

1. Delete the old profile.
2. Create a new profile.
3. Log in again from the launcher.

There is no name history table in the current model.

## Deletion Semantics

Deleting a profile:

- Deletes the profile row.
- Deletes texture rows linked to the profile.
- Deletes unreferenced texture objects by reference count.
- Revokes Yggdrasil tokens whose selectedProfile points to the profile.
- Writes an audit log entry.

If multiple profiles reference the same texture hash, deleting one profile does not remove an object that is still referenced by another profile.

## Disable and Ban

The current version intentionally does not provide a simple `profile.disabled` field. Disabling profiles affects login, join, hasJoined, texture reads, admin display, and audit policy; it should be defined by a future unified ban system.

Do not add temporary disable fields or bypass the deletion flow before that system exists.
