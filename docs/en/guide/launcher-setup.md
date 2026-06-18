---
description: Launcher setup guide for players and server owners covering API addresses, ALI, javaagent usage, login information, and validation checks.
---

# Launcher Setup

::: tip What this covers
This page explains what address to put into launchers and servers. For protocol details, read [Launcher Login](/en/guide/launcher-login) and [Yggdrasil API](/en/guide/yggdrasil-api).
:::

## Get the Correct Address

Administrators should give players a public site URL, for example:

```text
https://skin.example.com
```

The Yggdrasil API root is usually:

```text
https://skin.example.com/api/yggdrasil
```

If the launcher supports API Location Indication, enter the site root. AsterYggdrasil serves this header from the homepage:

```text
X-Authlib-Injector-API-Location: /api/yggdrasil/
```

If the launcher does not support ALI, enter the full `/api/yggdrasil` address.

## Before Players Log In

Players need at least:

- A site account.
- The site account password.
- A Minecraft profile under that account.

If the account has no Minecraft profile, launcher login may succeed but there is no usable `selectedProfile` for joining servers. Create a profile on the site first.

## Quick Launcher Reference

| Launcher capability | What to enter |
| --- | --- |
| Supports ALI | `https://skin.example.com` |
| Does not support ALI | `https://skin.example.com/api/yggdrasil` |
| Needs account identifier | Site account username or email |
| Allows profile-name login | Only after admin enables `yggdrasil_allow_profile_name_login` |

Launcher UI labels vary. Common names include "auth server", "Yggdrasil API", "third-party login", or "authlib-injector server". They all point to the same protocol root.

## javaagent Form

If you use authlib-injector directly as a `javaagent`:

```text
-javaagent:authlib-injector.jar=https://skin.example.com/api/yggdrasil
```

Servers should stay in online mode:

```properties
online-mode=true
enforce-secure-profile=true
```

If you temporarily disable secure profile checks, review the impact first. Troubleshooting-only settings should not remain in production configuration.

## Check After Login

After successful login, the launcher should receive:

- `accessToken`
- `clientToken`
- `availableProfiles`
- `selectedProfile`

If login succeeds but joining fails, read [Troubleshooting](/en/guide/troubleshooting#login-succeeds-but-joining-fails).

If joining works but skins do not show, read [Troubleshooting](/en/guide/troubleshooting#skins-or-capes-do-not-show).
