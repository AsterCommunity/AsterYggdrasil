---
description: AsterYggdrasil FAQ covering accounts, profiles, launchers, textures, deployment, and current feature boundaries.
---

# FAQ

## Is AsterYggdrasil a replacement for Mojang online mode?

No. It targets self-hosted Yggdrasil/authlib-injector integration, where you host your own site accounts, Minecraft profiles, and textures. Mojang official online-mode servers are not the goal of this project.

## How are site accounts and Minecraft profiles related?

Site accounts handle website login and launcher authentication. Minecraft profiles are in-game identities and contain profile names, UUIDs, and texture properties.

One site account can own multiple Minecraft profiles.

## Can profile names be changed?

Yes. Use the user or administrator API. Controlled renames keep the UUID, texture bindings, and audit trail, then temporarily invalidate Yggdrasil tokens bound to that profile.

Do not edit names directly in the database.

## What happens when a profile is deleted?

Deletion handles:

- The profile row.
- Texture rows bound to the profile.
- Texture objects that are no longer referenced.
- Yggdrasil tokens pointing to the profile.
- Audit logs.

## Why must skin URLs be public absolute URLs?

Minecraft clients and servers do not run inside your application process. They can only load the URL embedded in the `textures` property, so that URL must be reachable from the client machine.

Local testing can skip this briefly. Production deployments must configure `public_site_url` or `yggdrasil_public_base_url`.

## What is `skinDomains` for?

authlib-injector checks whether the texture URL host is covered by metadata `skinDomains`. AsterYggdrasil automatically includes the effective texture URL host. Configure `yggdrasil_skin_domains` only for additional CDN or external texture domains.

## Can I use S3 or MinIO?

The current production texture storage backend is local. The S3/MinIO config shape is reserved, but the backend is not implemented yet.

## Can I delete texture files directly?

Not recommended. Texture objects and database rows are linked by hash and reference counts. Directly deleting storage files can cause public 404s and consistency check failures.

Delete profiles, unbind textures, or delete hashes through the API.

## Do users need to log in again after signing key rotation?

Usually no. Signatures are calculated when profile properties are generated and are not stored in tokens. Clients or servers may need to fetch `/api/yggdrasil` metadata again to get the new `signaturePublickey`.

## Launcher login works but no character appears. What now?

The account has no Minecraft profile. Sign in to the site, create a profile, then log in from the launcher again.

## Which page should I read first?

- First run: [Getting Started](/en/guide/getting-started)
- Player usage: [User Guide](/en/guide/user-guide)
- Launcher setup: [Launcher Setup](/en/guide/launcher-setup)
- Production deployment: [Deployment Overview](/en/deployment/)
- Problems: [Troubleshooting](/en/guide/troubleshooting)
