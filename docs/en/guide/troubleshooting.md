---
description: AsterYggdrasil troubleshooting organized by startup, login, server join, textures, signatures, public URLs, and storage consistency.
---

# Troubleshooting

::: tip Start from the symptom
This page follows what users see, not source-code modules. Before editing the database, check public URLs, profiles, tokens, and texture paths.
:::

## Quick Table

| Symptom | Check first |
| --- | --- |
| Launcher cannot find auth server | Whether the address includes `/api/yggdrasil`, or whether the proxy stripped the ALI header |
| Login succeeds but joining fails | Whether the account has a profile, and whether the token points to a deleted or renamed profile |
| Skins or capes do not show | Whether the texture URL is publicly reachable and covered by `skinDomains` |
| Server signature verification fails | Whether the server cached old metadata, or signing keys were recently rotated |
| Texture returns 404 | Whether the hash exists, storage file exists, and public URL path is correct |
| Consistency check fails | Whether database and textures were restored as a set, or storage files were deleted directly |

## Launcher Cannot Find the Auth Server

First confirm the launcher uses a public address, not an internal one:

```text
https://skin.example.com/api/yggdrasil
```

If the launcher supports API Location Indication, this can also be:

```text
https://skin.example.com
```

Then request the site homepage and confirm this response header exists:

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

If it is missing, check whether the reverse proxy stripped custom response headers.

## Launcher Login Fails

Check in this order:

1. Use the site account username or email. Do not use a Minecraft profile name unless `yggdrasil_allow_profile_name_login` is enabled.
2. Confirm the same account and password can sign in to the site.
3. If registration activation or password reset depends on mail, confirm mail delivery works.
4. If there were many failures in a short time, wait for the rate-limit window.

Protocol login uses:

```text
POST /api/yggdrasil/authserver/authenticate
```

This endpoint returns Yggdrasil protocol errors, not the project API envelope.

## Login Succeeds but Joining Fails

The most common cause is that the account has no Minecraft profile. Account login and in-game identity are separate.

Ask the user to create a profile on the site, then log in from the launcher again.

If the profile was recently renamed:

- Old tokens are temporarily invalidated.
- The launcher must refresh or log in again.
- The profile UUID and texture bindings remain unchanged.

If the profile was deleted, related tokens are revoked. Create a new profile and log in again.

## Skins or Capes Do Not Show

Inspect the `textures` property from `profile/{uuid}` or `hasJoined`. After decoding, it should contain a URL like:

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

Then check:

- The URL is absolute.
- The browser or client machine can reach the URL.
- Metadata `skinDomains` includes `skin.example.com`.
- CDN or reverse proxy forwards `/api/yggdrasil/textures/{hash}` to the application.

Production deployments usually need:

```text
public_site_url
yggdrasil_public_base_url
yggdrasil_skin_domains
```

For normal deployments, configure `public_site_url` first. Use `yggdrasil_public_base_url` only when the API is exposed under a special path or separate host.

## Server Signature Verification Fails

Make launchers or servers fetch metadata again:

```text
GET /api/yggdrasil
```

Metadata contains `signaturePublickey`. After signing key rotation, an old cached public key cannot verify newly generated textures properties.

If only some clients fail, it is usually cache-related. If all clients fail, check whether an admin recently rotated signing keys and whether the reverse proxy caches metadata.

## Texture Upload Fails

Uploads accept PNG only. Common causes:

- MIME type is not `image/png`.
- Skin dimensions are not multiples of `64x32` or `64x64`.
- Cape dimensions are not multiples of `64x32` or `22x17`.
- File size exceeds `yggdrasil_max_texture_upload_bytes`.
- Pixel count exceeds `yggdrasil_max_texture_pixels`.
- Admin disabled skin or cape uploads.

Legacy `22x17` capes are padded to the standard canvas automatically.

## Texture 404 or Consistency Check Fails

Public reads use:

```text
GET /api/yggdrasil/textures/{hash}
```

If it returns 404:

- Confirm the hash comes from the current profile textures property.
- Confirm local texture storage is mounted.
- Confirm storage files were not deleted directly.
- Confirm the database and texture directory are from the same backup point.

If `yggdrasil-storage-consistency-check` reports failures, do not run cleanup first. Confirm whether a backup restore missed the texture directory or someone manually deleted storage files.
