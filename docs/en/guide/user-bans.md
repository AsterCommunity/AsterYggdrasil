---
description: AsterYggdrasil user capability ban guide covering scoped restrictions for Yggdrasil, profiles, texture upload, and public texture library interactions.
---

# User Capability Bans

::: tip What this page covers
This page is for site administrators. Capability bans restrict what a user can do in specific product areas. They do not disable the whole account and they do not target a single Minecraft profile.
:::

A capability ban is enforced by **user + scope**. One ban record can contain multiple scopes, such as texture upload and profile management together. Public texture library browsing is not blocked by capability bans; even restricted users can still browse published textures.

## When To Use Them

Common cases:

- A user repeatedly uploads invalid or abusive textures: restrict texture upload only.
- A user abuses profile names or profile churn: restrict profile management only.
- A user should temporarily stay out of servers while keeping site access: restrict Yggdrasil join only.
- A user should not submit, withdraw, or report public textures: restrict public texture library interaction.

To block the entire site account, use user status or session revocation. Capability bans are for specific business capabilities.

## Scopes

| Scope | Blocks | Does not block |
| --- | --- | --- |
| `yggdrasil_access` | Yggdrasil/authlib-injector login and token use such as refresh/validate | Site web login |
| `yggdrasil_join` | `join`, `hasJoined` matches, Minecraft services multiplayer state | Site web login and texture browsing |
| `minecraft_profile_manage` | Creating, deleting, renaming Minecraft profiles, and operations that rewrite profile texture bindings | Viewing existing own profiles |
| `texture_upload` | Wardrobe upload, direct Yggdrasil texture upload, wardrobe texture metadata updates, profile texture bind/delete | Public texture library browsing |
| `texture_library_interact` | Submitting or withdrawing public texture library reviews, reporting, and user-side public library interaction actions | Public texture library browsing |

`texture_upload` is checked before receiving the uploaded file. A restricted user cannot write an image to disk or create a texture record before the request fails.

## Admin Entry Point

Open a user detail page in the admin panel and use the capability ban section:

1. Create a ban.
2. Select one or more scopes.
3. Fill the internal reason.
4. Optionally set a user-visible reason, start time, expiration time, and internal note.
5. Save.

An empty start time means the ban starts immediately. An empty expiration time means no scheduled expiration. Expiration must be later than the start time.

The same user cannot have two currently effective bans covering the same scope. If an effective ban already contains `texture_upload`, another effective ban containing `texture_upload` will be rejected. Update or revoke the existing record instead.

## Update And Revoke

Only currently effective bans can be updated or revoked. Expired and revoked records remain as history and should not be edited.

Updating can change scopes, reason, user-visible reason, internal note, and time range. Revoking can include a revoke note. Create, update, and revoke actions are audited, and the event list keeps previous and next scopes, status, and expiration values.

## What Users See

The account overview page shows the user's currently effective capability restrictions. Revoked and expired records are not shown in the default user-facing panel.

Users can see:

- Restricted scopes.
- Current status.
- Start and expiration time.
- User-visible reason; if it is empty, the ban reason is shown.

Users cannot see internal admin notes, operator ids, revoke notes, or other internal fields.

## API Reference

Admin APIs use the project response envelope.

```text
GET    /api/v1/admin/user-bans
GET    /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/users/{user_id}/bans
PATCH  /api/v1/admin/user-bans/{ban_id}
POST   /api/v1/admin/user-bans/{ban_id}/revoke
GET    /api/v1/admin/user-bans/{ban_id}/events
```

Create request example:

```json
{
  "scopes": ["texture_upload", "minecraft_profile_manage"],
  "reason": "repeated invalid uploads",
  "public_reason": "Texture upload is temporarily restricted.",
  "admin_note": "review again after appeal",
  "starts_at": null,
  "expires_at": "2026-07-01T00:00:00Z"
}
```

`scopes` must be a non-empty array. The API does not accept the old single `scope` request field.

List endpoints can use one `scope` query parameter to find records that contain that scope:

```text
GET /api/v1/admin/user-bans?user_id=123&scope=texture_upload&effective_only=true
```

Current user lookup:

```text
GET /api/v1/account/bans?effective_only=true
```

## Error Codes

Project API requests blocked by a capability ban return `403` with:

```json
{
  "code": "user_ban.forbidden",
  "error": {
    "retryable": false
  }
}
```

Common admin error codes:

| Code | Meaning |
| --- | --- |
| `user_ban.already_active` | The target user already has an effective ban covering one of these scopes |
| `user_ban.not_active` | The request tried to update or revoke an expired/revoked ban |
| `user_ban.duration_invalid` | Expiration is not later than start time |
| `user_ban.reason_invalid` | Reason, notes, or scope list is invalid |
| `user_ban.not_found` | The ban record does not exist |

Yggdrasil protocol endpoints still return protocol-compatible error bodies and do not use the project envelope. Launchers usually see Yggdrasil `ForbiddenOperationException` or invalid token/credentials style errors because that is the compatible protocol surface.

## Before Production

- Operators need the `users` scope to manage capability bans.
- Fill a user-visible reason when the user should know what to fix.
- Do not use capability bans as a replacement for account disabling.
- When testing `texture_upload`, confirm the request fails before uploaded files are written.
- Confirm the user account page does not show revoked records by default.
