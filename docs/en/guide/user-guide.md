---
description: A user-facing guide for AsterYggdrasil accounts, Minecraft profiles, textures, launcher login, and common operational boundaries.
---

# User Guide

This page follows the real usage flow. You do not need to read the entire Yggdrasil protocol first; understand where accounts, profile names, textures, and launcher login fit, and the rest becomes much easier.

If you are the server owner or deployer, still read this once from the normal user perspective. Many integration issues come from missing profiles, missing public URLs, or launchers holding old tokens.

## Sign In

AsterYggdrasil uses site accounts as the login identity. On a fresh service, there are no users yet, so the first account must be created through setup.

Common cases:

- No users exist yet: the first account becomes the administrator.
- An account already exists: log in with username or email.
- Public registration is enabled: new users can register themselves.
- Registration activation or password reset is enabled: mail delivery must be configured and working.

A site account is not a Minecraft profile. One site account can own one or more Minecraft profiles, and the launcher sees those profiles after login.

## Create a Minecraft Profile

The Minecraft profile is the identity seen by launchers and servers. It contains the protocol `id` and `name`, and it is the profile that receives skin/cape texture properties.

Current-user profile APIs:

```text
GET    /api/v1/profiles/minecraft
POST   /api/v1/profiles/minecraft
PUT    /api/v1/profiles/minecraft/{uuid}/name
GET    /api/v1/profiles/minecraft/{uuid}/textures
DELETE /api/v1/profiles/minecraft/{uuid}
```

Creating a profile only requires a name. Profile names support controlled renames. A rename keeps the UUID, texture bindings, and audit trail, then temporarily invalidates bound Yggdrasil tokens. Refresh or log in again from the launcher to receive the new name.

::: warning Do not edit profile names directly in the database
Direct renames can desynchronize launcher caches, Yggdrasil tokens, server allowlists, texture properties, and audit records. The resulting state is difficult to diagnose.
:::

## Manage Skins and Capes

Textures have two workflows.

The wardrobe workflow uploads textures to the user's own texture library first, then binds one texture to a profile. This works well for web management and for reusing textures across profiles owned by the same account.

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
DELETE /api/v1/wardrobe/textures/{texture_id}
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
```

The direct Yggdrasil workflow writes a texture directly to a target profile. Launchers and compatible tools usually use this route.

```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
DELETE /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
```

Upload requirements:

- The file must be `image/png`.
- Skins support multiples of `64x32` or `64x64`.
- Capes support multiples of `64x32` or `22x17`.
- Legacy `22x17` capes are padded to the standard canvas before storage.
- The server keeps only the re-encoded PNG, not the raw upload.

Public texture reads use:

```text
GET /api/yggdrasil/textures/{hash}
```

The hash is calculated from the processed PNG bytes. Re-uploading the same processed image produces a stable URL hash.

## Log In From a Launcher

Launcher login uses the Yggdrasil authserver:

```text
POST /api/yggdrasil/authserver/authenticate
```

Users log in with their site account username or email and password. If the administrator enables `yggdrasil_allow_profile_name_login`, profile-name login can also be allowed.

Successful login returns:

- `accessToken`
- `clientToken`
- `availableProfiles`
- `selectedProfile`

If the account has no Minecraft profile, login can still succeed, but there is no `selectedProfile` that can join a server. Create a profile first, then log in again.

## Configure authlib-injector

The protocol root is:

```text
https://your-domain.example/api/yggdrasil
```

If the launcher supports API Location Indication, users may enter only the site root. AsterYggdrasil serves this response header from the homepage:

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

For direct `javaagent` usage, use the full protocol root:

```text
-javaagent:authlib-injector.jar=https://your-domain.example/api/yggdrasil
```

## Common Issues

### Login succeeds but joining fails

Check whether the account has a Minecraft profile. Without a profile, there is no usable `selectedProfile`.

Also check token freshness. Deleting a profile revokes Yggdrasil tokens that point to it, so renaming by delete-and-create requires a fresh launcher login.

### Skins do not show

First check whether the profile response contains a textures property. Then check whether the URL inside that property is an absolute public URL reachable by the client.

Production deployments usually need:

```text
public_site_url
```

or the advanced override:

```text
yggdrasil_public_base_url
```

If you use a CDN or an additional host, make sure metadata `skinDomains` covers the texture URL host.

### Server signature verification fails

Make the launcher or server fetch `/api/yggdrasil` metadata again. After signing key rotation, an old cached public key cannot verify newly generated textures properties.

### Can I rename a profile?

Yes, but use the site API or administrator API. A controlled rename keeps the profile UUID and texture bindings, then temporarily invalidates bound Yggdrasil tokens. Refresh or log in again from the launcher to receive the new name. Do not edit profile names directly in the database.
