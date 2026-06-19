---
description: Session forwarding guide for site owners and Minecraft server owners, covering AY, Yggdrasil-compatible sites, Mojang, priority, weights, texture forwarding, and CustomSkinLoader.
---

# Yggdrasil Forwarding

::: tip What this page covers
This page is for site owners and Minecraft server administrators. It explains how to point a server at AsterYggdrasil while allowing players from AY, external skin sites, or Mojang to pass session checks.
:::

Yggdrasil forwarding only handles **sessionserver join verification**. It does not import external skin-site accounts into AY, and it does not proxy external authserver login.

Common use cases:

- The Minecraft server points only to AY.
- Local AY players can join.
- Yggdrasil-compatible sites (such as LittleSkin), Mojang, or other upstream players can join through AY `hasJoined` forwarding.
- Site owners can tune upstream order with priority and weight.
- When clients trust AY metadata, AY can proxy upstream texture URLs and re-sign texture properties.

## Quick Reference

| Goal | Where to look |
| --- | --- |
| Point the server at AY | Admin panel -> Yggdrasil forwarding |
| Tune AY, Yggdrasil-compatible site, and Mojang order | Update upstream priority and weight |
| Let AY clients display compatible-site textures | Enable texture forwarding on the matching compatible-site upstream |
| Let compatible-site clients keep original textures | Disable texture forwarding on the matching compatible-site upstream |
| Add client-side mixed skin-site fallback | Use CustomSkinLoader |

## Server Address

The Minecraft server should still point to AY:

```text
https://skin.example.com/api/yggdrasil
```

For local testing:

```text
http://localhost:3300/api/yggdrasil
```

authlib-injector javaagent example:

```text
-javaagent:authlib-injector.jar=https://skin.example.com/api/yggdrasil
```

Keep the server in online mode:

```properties
online-mode=true
enforce-secure-profile=true
```

The Minecraft server will ask AY:

```text
GET /api/yggdrasil/sessionserver/session/minecraft/hasJoined
```

AY then queries local and upstream sources according to your forwarding configuration.

## Forwarding Servers

A forwarding server is a `hasJoined` lookup source.

| Source | Purpose | Recommendation |
| --- | --- | --- |
| AsterYggdrasil | Local AY join records | Keep enabled, usually with high priority |
| Yggdrasil-compatible site (such as LittleSkin) | External skin-site sessionserver | Enable when your community needs it |
| Mojang | Official sessionserver | Treat as testing for now |

AY local is also represented as a forwarding server, so local AY and remote upstreams can be ordered in the same table.

## Priority and Weight

Lookup order is determined by priority first, then weight.

- Lower priority values are checked earlier.
- Within the same priority group, higher weight means a higher chance to be checked earlier.
- Once an upstream returns a matching profile, AY returns it and stops checking later upstreams.
- Upstream failures are recorded, then AY continues with later upstreams.

A conservative starting point:

| Upstream | enabled | priority | weight |
| --- | ---: | ---: | ---: |
| AsterYggdrasil | yes | 100 | 1 |
| Yggdrasil-compatible site | yes | 150 | 1 |
| Mojang | no or testing | 200 | 1 |

If a remote upstream should override local AY profiles, give it a lower priority. Check name-collision behavior before using that in production.

## Session Behavior

Joining a server has two parts:

1. The client sends `join` to the Yggdrasil service it logged in with.
2. The Minecraft server sends `hasJoined` to AY.

For a Yggdrasil-compatible site player joining an AY-backed server:

```text
Compatible-site client -> compatible-site join
Minecraft server -> AY hasJoined
AY -> compatible-site hasJoined
```

If the compatible-site upstream matches the join, AY returns that player profile to the Minecraft server.

## Texture Forwarding

Texture forwarding affects only the upstream `textures` property.

When disabled, AY returns the upstream `textures.value` and `signature` unchanged. Clients continue to use the upstream texture URL.

When enabled, AY:

1. Decodes upstream `textures.value`.
2. Rewrites texture URLs to AY proxy URLs.
3. Re-signs the `textures` property with AY's Yggdrasil private key.
4. Lets the client request the AY proxy URL.
5. Fetches the original upstream texture URL server-side and returns the PNG.

Proxy URLs look like:

```text
https://skin.example.com/api/yggdrasil/sessionserver/session/minecraft/forwardedTextures/{upstream_id}/{texture_hash}/{ticket}
```

`ticket` is a signed AY forwarding credential that contains the original texture URL. It does not depend on a short TTL cache, so long online sessions and later clients can still load the skin.

::: warning Texture forwarding requires clients to trust AY metadata
After AY rewrites a texture URL, AY must re-sign the texture property. The client must use AY metadata so it trusts AY's `signaturePublickey` and `skinDomains`.
:::

## Expected Client Behavior

Assume:

- The Minecraft server points to AY.
- Compatible-site session forwarding is enabled.
- Compatible-site texture forwarding is enabled.
- AY `public_site_url` or `yggdrasil_public_base_url` is correct, and metadata `skinDomains` includes the AY public host.

| Client login source | Can join AY server | Sees AY local skins | Sees compatible-site textures | Notes |
| --- | ---: | ---: | ---: | --- |
| AY client | yes | yes | yes | Recommended path. The client trusts AY keys and texture host. |
| Compatible-site client | yes | usually no | may fail | The client trusts that compatible site metadata, not AY re-signed textures. |
| Mojang client | depends on login and upstream match | usually no | usually no | Do not rely on Mojang clients for mixed skin-site display. |

If compatible-site clients should reliably show original textures from that site, disable texture forwarding on the matching upstream. AY will return upstream textures unchanged, so compatible-site clients can verify them with that site metadata.

| Compatible-site texture forwarding | AY client sees compatible-site textures | Compatible-site client sees original-site textures |
| ---: | ---: | ---: |
| enabled | usually yes | may fail |
| disabled | depends on whether AY metadata allows the original texture host | usually yes |

If you want one consistent experience, ask players to log in through AY. If players keep using different skin-site clients, texture display may differ between clients.

## CustomSkinLoader Fallback

[CustomSkinLoader](https://github.com/xfl03/MCCustomSkinLoader) is a client-side mod that can load skins from sources such as Mojang, LittleSkin, Ely.by, CustomSkinAPI, UniSkinAPI, and legacy APIs. It is useful when a mixed community wants clients to display textures from multiple skin sites.

It has a different role from AY forwarding:

| Capability | AY Yggdrasil forwarding | CustomSkinLoader |
| --- | ---: | ---: |
| Decides whether a player can join | yes | no |
| Configured centrally by the server owner | yes | no, clients install the mod |
| Displays textures from multiple skin sites | works in some scenarios | better fit |
| Depends on authlib-injector metadata signatures | yes | not in the same way |

Recommended split:

- Let AY forwarding handle server join verification.
- Enable AY texture forwarding when AY clients should display compatible-site textures.
- For mixed-client communities, recommend CustomSkinLoader and configure LittleSkin, Mojang, or other skin sources on the client side.
- Do not treat CustomSkinLoader as an authorization system. It only affects texture display.

::: details Why CustomSkinLoader helps
authlib-injector texture properties use signatures and `skinDomains`. A client normally trusts one Yggdrasil metadata root. AY can re-sign external textures, but a compatible-site client may not trust AY's public key. The original site signature cannot cover an AY-rewritten URL.

CustomSkinLoader adds client-side texture source lookup, which helps bypass the single-metadata-root display limitation. It does not replace sessionserver verification.
:::

## Troubleshooting

| Symptom | Check |
| --- | --- |
| Compatible-site player cannot join | Compatible-site upstream enabled, base URL correct, server points to AY, compatible-site client completed join |
| AY client cannot see compatible-site texture | Compatible-site texture forwarding enabled, AY metadata includes AY public host, `hasJoined` URL includes `{ticket}` |
| Texture URL returns 404 | The client may have an old two-segment URL; restart AY and rejoin so it receives `{upstream_id}/{texture_hash}/{ticket}` |
| Signature verification fails | Client may not be using AY metadata, or has cached an old AY public key |
| URL is not in whitelist | Check whether `public_site_url` / `yggdrasil_public_base_url` host appears in metadata `skinDomains` |
| Different clients see different skins | This is normal with multiple Yggdrasil trust roots; use AY login consistently or add CustomSkinLoader |

## Before Production

- `public_site_url` or `yggdrasil_public_base_url` is an absolute URL reachable by clients.
- Metadata `skinDomains` includes the AY public host.
- The Yggdrasil signing private key exists and clients can fetch `signaturePublickey`.
- Compatible-site upstream base URLs point to their Yggdrasil API roots.
- Mojang is still treated as testing; do not make it the only usable path.
- After enabling texture forwarding, test with an AY client and confirm it requests `/forwardedTextures/{upstream_id}/{texture_hash}/{ticket}`.
