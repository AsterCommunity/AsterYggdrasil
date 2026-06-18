# Yggdrasil API Implementation

This document describes the current Yggdrasil/authlib-injector implementation in this repository. User-facing setup docs live in `docs/en/guide/yggdrasil-api.md` and `docs/en/guide/yggdrasil-textures.md`; this file focuses on code boundaries, endpoint behavior, authentication, and testing contracts.

## Code Boundaries

| Layer | Location | Contract |
| --- | --- | --- |
| Routes | `src/api/routes/yggdrasil.rs`, `src/api/routes/yggdrasil/texture.rs`, `src/api/routes/yggdrasil/minecraft_services.rs` | Handlers extract HTTP input, perform protocol authentication, call services, and return protocol-shaped responses. |
| DTOs | `src/api/dto/yggdrasil.rs` | All protocol wire fields belong here. Keep authlib-injector/Mojang wire names stable; do not build protocol responses with ad hoc `json!` in handlers. |
| Services | `src/services/yggdrasil_service.rs`, `src/services/yggdrasil_service/*` | Token, profile, session, metadata, and minecraftservices behavior lives here. |
| Textures | `src/services/texture_service/`, `src/texture_storage/` | PNG validation, re-encoding, hashing, storage, and public reads go through texture services. |
| Config | `src/config/definitions.rs`, `src/config/yggdrasil.rs` | Runtime Yggdrasil settings are defined and normalized here. |
| OpenAPI | `src/api/openapi.rs` | Register every changed protocol path and schema. |
| Tests | `tests/test_yggdrasil.rs` | Covers protocol compatibility, error bodies, token lifecycle, textures, and minecraftservices endpoints. |

Yggdrasil protocol endpoints do not use the project API envelope. `/api/v1` still returns:

```json
{ "code": "success", "msg": "", "data": {} }
```

Everything under `/api/yggdrasil` must keep native protocol fields and error bodies.

## API Root

Default protocol root:

```text
/api/yggdrasil
```

The site root `/` exposes API Location Indication through `X-Authlib-Injector-API-Location: /api/yggdrasil/`. authlib-injector maps Mojang services to this root:

| Original service | AsterYggdrasil path |
| --- | --- |
| `authserver.mojang.com` and compatible auth services | `/api/yggdrasil/authserver/*` |
| `sessionserver.mojang.com` | `/api/yggdrasil/sessionserver/*` |
| `api.minecraftservices.com` | `/api/yggdrasil/minecraftservices/*` |

Forwarded `Authorization` headers are preserved, so minecraftservices endpoints can validate Bearer tokens in the Mojang style.

## Metadata

| Method | Path | Auth | Main handler |
| --- | --- | --- | --- |
| `GET` | `/api/yggdrasil` | None | `metadata` |
| `GET` | `/api/yggdrasil/` | None | `metadata` |

Response DTO: `YggdrasilMetaResp`.

Current fields:

- `meta.serverName`: from `yggdrasil_server_name`.
- `meta.implementationName`: always `AsterYggdrasil`.
- `meta.implementationVersion`: current crate version.
- `meta.links.homepage`: site root when `public_site_url` is configured.
- `meta.links.register`: `/register` when user registration is enabled.
- `meta.feature.non_email_login`: from `yggdrasil_allow_profile_name_login`.
- `meta.feature.enable_profile_key`: from `yggdrasil_enable_profile_key`.
- `meta.feature.enable_mojang_anti_features`: from `yggdrasil_enable_mojang_anti_features`.
- `meta.feature.username_check`: currently fixed to `true`, keeping authlib-injector username character checks enabled; Aster profile names are currently limited to 3-16 ASCII letters, digits, or underscores.
- `skinDomains`: default Mojang domains, configured domains, and active public texture URL hosts.
- `signaturePublickey`: RSA public key used to verify signed `textures` properties.

These advanced flags are currently not declared:

- `feature.legacy_skin_api`
- `feature.no_mojang_namespace`

Under authlib-injector semantics, omitted feature flags are normally equivalent to false, meaning the server does not claim support.

`feature.legacy_skin_api` remains undeclared. The old official `skins.minecraft.net` domain is no longer a reliable entrypoint; the modern official path is username -> UUID -> sessionserver profile -> `textures.minecraft.net/texture/{hash}`, so the server does not need to implement legacy `GET /skins/MinecraftSkins/{username}.png`.

Metadata uses `Cache-Control: no-cache, no-store, must-revalidate`. Clients should refetch metadata after signature key or public URL changes.

## Authserver

| Method | Path | Auth | Success | Failure |
| --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/authserver/authenticate` | username/password | `200` + `YggdrasilAuthenticateResp` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/refresh` | request body token | `200` + `YggdrasilRefreshResp` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/validate` | request body token | `204` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/invalidate` | request body token | `204` | `400`/`403` + `YggdrasilErrorBody` |
| `POST` | `/api/yggdrasil/authserver/signout` | username/password | `204` | `400`/`403` + `YggdrasilErrorBody` |

Implementation:

- route: `src/api/routes/yggdrasil.rs`
- service: `src/services/yggdrasil_service/auth.rs`, `login.rs`, `token.rs`

Important contracts:

- `authenticate` supports email/account identifier login. When `yggdrasil_allow_profile_name_login = true`, it also accepts profile names.
- `signout` reuses the login identifier resolver, so profile-name signout also works when profile-name login is enabled. This is a permissive extension over docs that literally describe `username` as an email address.
- Plain access tokens are only returned to clients; the database stores token hashes.
- `clientToken` is client-provided; the server generates one when it is missing.
- `refresh` must revoke the old token and issue the replacement in one transaction. If refresh fails, the old token should remain valid.
- Profile rename temporarily invalidates tokens bound to that selected profile, forcing launchers to refresh and pick up the new name.
- High-frequency auth endpoints have debug logs, but logs must only contain lengths, hashes, booleans, token ids, user ids, profile ids, and similar non-secret values.

## Sessionserver

| Method | Path | Auth | Success | Failure/miss |
| --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/sessionserver/session/minecraft/join` | request body access token | `204` | `400`/`403` + `YggdrasilErrorBody` |
| `GET` | `/api/yggdrasil/sessionserver/session/minecraft/hasJoined` | None | `200` + `YggdrasilProfile` | miss `204`, invalid request `400` |
| `GET` | `/api/yggdrasil/sessionserver/session/minecraft/profile/{uuid}` | None | `200` + `YggdrasilProfile` | miss `204`, invalid request `400` |
| `GET` | `/api/yggdrasil/sessionserver/blockedservers` | None | No current `200` | `404` + `MinecraftServicesPathError` |

Implementation:

- route: `src/api/routes/yggdrasil.rs`
- blocked servers route: `src/api/routes/yggdrasil/minecraft_services.rs`
- service: `src/services/yggdrasil_service/session.rs`, `properties.rs`

Important contracts:

- `join` validates access token, selected profile, and serverId, then records profile, serverId, and client IP. It intentionally does not store the plain access token.
- `hasJoined` matches by username, serverId, and optional IP.
- `profile/{uuid}` defaults `unsigned` to `true`. With `unsigned=false`, the `textures` property is signed.
- `blockedservers` currently returns 404, equivalent to not providing a Mojang blocked server list. A future moderation system must not rely only on this endpoint; bans must also be enforced through `authenticate`, `refresh`, `validate`, `join`, and `hasJoined`.

## Profile Lookup

| Method | Path | Auth | Success | Failure |
| --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/api/profiles/minecraft` | None | `200` + `YggdrasilProfile[]` | `400` + `YggdrasilErrorBody` |

The request body is an array of profile names. The current limit is 100 names. Names must match Minecraft profile rules: 3-16 ASCII letters, digits, or underscores.

Missing names are omitted from the response array.

## Texture API

| Method | Path | Auth | Success | Failure |
| --- | --- | --- | --- | --- |
| `PUT` | `/api/yggdrasil/api/user/profile/{uuid}/{skin\|cape}` | Bearer access token | `204` | `400`/`401`/`403` + `YggdrasilErrorBody` |
| `DELETE` | `/api/yggdrasil/api/user/profile/{uuid}/{skin\|cape}` | Bearer access token | `204` | `400`/`401`/`403` + `YggdrasilErrorBody` |
| `GET` | `/api/yggdrasil/textures/{hash}` | None | `200 image/png` | `404` |

Implementation:

- route: `src/api/routes/yggdrasil/texture.rs`
- service: `src/services/texture_service/`

Upload contracts:

- `PUT` uses `multipart/form-data`.
- The file field is named `file` and must be `image/png`.
- Skins may include a text `model` field parsed by texture service. Capes always use the default model.
- Skin dimensions may be integer multiples of `64x32` or `64x64`.
- Cape dimensions may be integer multiples of `64x32` or `22x17`. Legacy `22x17` capes are padded with transparency to the standard `64x32` ratio.
- The server decodes PNG, validates pixel limits, re-encodes a clean PNG, then computes the SHA-256 hash from processed bytes.
- Public reads use hash URLs and include texture-service-provided `Cache-Control`, `ETag`, and `Content-Length`; matching `If-None-Match` requests return `304`.
- `/textures/{hash}` serves both database-backed textures and the embedded Steve/Alex default skin hashes. Profiles without a bound skin automatically receive a default skin in their `textures` property.

authlib-injector explicitly expects texture upload/delete missing or invalid tokens to return `401`. Do not normalize this into the ordinary Yggdrasil invalid-token `403 ForbiddenOperationException` behavior.

## Minecraft Services

These endpoints represent `api.minecraftservices.com` after authlib-injector redirection.

| Method | Path | Feature flag | Auth | Success | Failure |
| --- | --- | --- | --- | --- | --- |
| `POST` | `/api/yggdrasil/minecraftservices/player/certificates` | `feature.enable_profile_key` | Bearer access token with selected profile | `200` + `MinecraftServicesCertificateResp` | `401`/`404` |
| `GET` | `/api/yggdrasil/minecraftservices/privileges` | `feature.enable_mojang_anti_features` | Bearer access token | `200` + `MinecraftServicesPrivilegesResp` | `401`/`404` |
| `GET` | `/api/yggdrasil/minecraftservices/player/attributes` | `feature.enable_mojang_anti_features` | Bearer access token | `200` + `MinecraftServicesPlayerAttributesResp` | `401`/`404` |
| `GET` | `/api/yggdrasil/minecraftservices/privacy/blocklist` | `feature.enable_mojang_anti_features` | Bearer access token | `200` + `MinecraftServicesPrivacyBlocklistResp` | `401`/`404` |

Implementation:

- route: `src/api/routes/yggdrasil/minecraft_services.rs`
- service: `src/services/yggdrasil_service/minecraft_services.rs`

Error body follows the Mojang shape:

```json
{ "path": "/player/attributes" }
```

Missing, invalid, expired, revoked, or temporarily invalid tokens return `401`. Disabled feature flags and unknown paths return `404` with `Cache-Control: no-store`.

Current policy:

- `player/certificates` generates an ephemeral 2048-bit RSA keypair for an authenticated token with a selected profile.
- `publicKeySignature` and `publicKeySignatureV2` are authlib-injector-compatible dummy values, not official Mojang signatures. A self-hosted server cannot mint Mojang signatures.
- `expiresAt` is currently 48 hours after issue.
- `refreshedAfter` is currently 36 hours after issue.
- `privileges` currently returns chat, multiplayer, realms, telemetry, and optional telemetry enabled.
- `player/attributes` currently returns permissive privileges, profanity filter off, friend features disabled, text chat enabled, and empty ban scopes.
- `privacy/blocklist` currently returns an empty `blockedProfiles` array.

Future moderation integration points:

- `minecraft_services_privileges`: disable `onlineChat`, `multiplayerServer`, or `multiplayerRealms` from account/profile bans.
- `minecraft_services_player_attributes`: fill `friendsPreferences`, `chatPreferences`, and `banStatus.bannedScopes` from bans and social preferences.
- `minecraft_services_privacy_blocklist`: connect user block lists.
- `blockedservers`: implement a protocol-compatible response only if the product needs a server block list; otherwise 404 is acceptable.
- Enforcement still belongs in auth and join/session flows. Minecraft services policy is only the client-visible policy surface.

`keyPair.privateKey` is sensitive data returned to the client. Do not persist it, log it, or include it in audit details.

## Field Reference

This section explains wire fields. Rust field names may be snake_case, but protocol responses must keep the camelCase, dotted, or uppercase names shown here.

Field meanings are split into three confidence levels:

- **Specified by protocol**: behavior is directly covered by Yggdrasil/authlib-injector docs or current implementation.
- **Inferred from Mojang samples**: the field appears in Mojang responses or authlib-injector compatibility code, but this repository does not have a complete field-level spec. These rows explicitly say "inferred".
- **Aster current policy**: the field exists in the protocol shape, but Aster currently returns a fixed compatibility value. Future moderation, social, or preference systems should replace it with real state.

### Common Error Fields

| DTO | Field | Type | Meaning |
| --- | --- | --- | --- |
| `YggdrasilErrorBody` | `error` | string | Protocol error category, such as `ForbiddenOperationException` or `IllegalArgumentException`. Clients usually branch on this coarse type. |
| `YggdrasilErrorBody` | `errorMessage` | string | Human/debug-readable error message. Must not include access tokens, passwords, private keys, or other secrets. |
| `YggdrasilErrorBody` | `cause` | string? | Optional protocol-compatible cause. Current implementation usually omits it. |
| `MinecraftServicesPathError` | `path` | string | Mojang-style minecraftservices error body containing only the rejected relative path, for example `/privileges`. |

### Profile And Property Fields

| DTO | Field | Type | Meaning |
| --- | --- | --- | --- |
| `YggdrasilProfile` | `id` | string | Unsigned Minecraft UUID. Current profile UUIDs are generated and persisted when profiles are created. |
| `YggdrasilProfile` | `name` | string | Minecraft profile name: 3-16 ASCII letters, digits, or underscores. |
| `YggdrasilProfile` | `properties` | array? | Additional profile properties. authenticate/refresh summaries usually omit textures; session profiles include texture properties. |
| `YggdrasilProfileProperty` | `name` | string | Property name. Current values are mainly `textures` and `uploadableTextures`. |
| `YggdrasilProfileProperty` | `value` | string | Property value. `textures` is base64(JSON); `uploadableTextures` is a comma-separated capability list. |
| `YggdrasilProfileProperty` | `signature` | string? | Returned when `unsigned=false` or signing is required. Verify with metadata `signaturePublickey`. |
| `YggdrasilUser` | `id` | string | Protocol representation of the Aster user id. |
| `YggdrasilUser` | `properties` | array | User-level properties. Currently always empty; optional authlib-injector properties such as `preferredLanguage` are not exposed yet. |

Decoded `textures` property `value` JSON:

| Field | Type | Meaning |
| --- | --- | --- |
| `timestamp` | number | Server-side property generation timestamp in milliseconds. |
| `profileId` | string | Unsigned Minecraft UUID. |
| `profileName` | string | Profile name at property generation time. |
| `textures` | object | Texture map, usually keyed by `SKIN` and `CAPE`. |
| `textures.SKIN.url` | string | Public skin PNG URL. Its host must be covered by metadata `skinDomains`. |
| `textures.SKIN.metadata.model` | string? | Skin model. `slim` means slim arms; default model omits metadata. |
| `textures.CAPE.url` | string | Public cape PNG URL. |

If a profile has no bound skin, the current implementation adds an embedded default skin. The default model is selected stably from the profile UUID low bit: even uses Steve/default, odd uses Alex/slim. Default skins are also served through `/api/yggdrasil/textures/{hash}`.

`uploadableTextures.value` comes from the profile's upload capability state, for example `skin,cape`, `skin`, or `cape`. It tells authlib-injector clients which texture types can be uploaded for that profile through protocol endpoints.

### Authserver Request/Response Fields

| DTO | Field | Type | Meaning |
| --- | --- | --- | --- |
| `YggdrasilAgentReq` | `name` | string | Client-declared agent. Current implementation expects Minecraft semantics. |
| `YggdrasilAgentReq` | `version` | number | Agent version. Current validation requires `1`. |
| `YggdrasilAuthenticateReq` | `username` | string | Login identifier. Defaults to email/account identifier; profile names are accepted when profile-name login is enabled. |
| `YggdrasilAuthenticateReq` | `password` | string | Plain password for this verification only. Never log or persist it. |
| `YggdrasilAuthenticateReq` | `clientToken` | string? | Launcher-generated client identifier. The server generates and returns one when omitted. |
| `YggdrasilAuthenticateReq` | `requestUser` | bool | Whether the response should include `user`. |
| `YggdrasilAuthenticateReq` | `agent` | object? | Agent information. When present, it must validate. |
| `YggdrasilAuthenticateResp` | `accessToken` | string | Newly issued Yggdrasil access token. Returned only to the client; the database stores only a hash. |
| `YggdrasilAuthenticateResp` | `clientToken` | string | Bound client token, from the request or generated by the server. |
| `YggdrasilAuthenticateResp` | `availableProfiles` | array | Profiles available to this account. |
| `YggdrasilAuthenticateResp` | `selectedProfile` | object? | Selected profile when the protocol flow selects one. |
| `YggdrasilAuthenticateResp` | `user` | object? | Returned when `requestUser=true`. |
| `YggdrasilRefreshReq` | `accessToken` | string | Old access token to refresh. It is revoked after a successful refresh. |
| `YggdrasilRefreshReq` | `clientToken` | string? | If supplied, it must match the old token's bound client token. |
| `YggdrasilRefreshReq` | `requestUser` | bool | Whether the response should include `user`. |
| `YggdrasilRefreshReq` | `selectedProfile` | object? | Requests binding the token to a profile. A token already bound to a profile cannot be rebound. |
| `YggdrasilRefreshResp` | `accessToken` | string | Replacement access token. |
| `YggdrasilRefreshResp` | `clientToken` | string | Bound client token. |
| `YggdrasilRefreshResp` | `selectedProfile` | object? | Selected profile bound to the new token. |
| `YggdrasilRefreshResp` | `user` | object? | Returned when `requestUser=true`. |
| `YggdrasilTokenReq` | `accessToken` | string | Token used by validate/invalidate. |
| `YggdrasilTokenReq` | `clientToken` | string? | If supplied, it must match the token binding. |
| `YggdrasilSignoutReq` | `username` | string | Account login identifier. Current implementation reuses login resolution, so it may accept profile names. |
| `YggdrasilSignoutReq` | `password` | string | Plain password used only to authorize signout. |

### Sessionserver Fields

| DTO | Field | Type | Meaning |
| --- | --- | --- | --- |
| `YggdrasilJoinReq` | `accessToken` | string | Access token obtained after launcher login. |
| `YggdrasilJoinReq` | `selectedProfile` | string | Unsigned UUID. Must belong to the token's user/binding. |
| `YggdrasilJoinReq` | `serverId` | string | serverId/hash from the Minecraft client/server handshake. Logs may only record a hash. |
| `YggdrasilHasJoinedQuery` | `username` | string | Profile name being verified by the Minecraft server. |
| `YggdrasilHasJoinedQuery` | `serverId` | string | Same serverId/hash used in the join request. |
| `YggdrasilHasJoinedQuery` | `ip` | string? | Optional client IP. When supplied, it must match the join record. |
| `YggdrasilProfileQuery` | `unsigned` | bool? | Whether property signatures should be omitted. Missing means `true`; `false` returns signatures. |

### Metadata Fields

| DTO | Field | Type | Meaning |
| --- | --- | --- | --- |
| `YggdrasilMetaResp` | `meta` | object | Main authlib-injector metadata object. |
| `YggdrasilMetaResp` | `skinDomains` | string[] | Domain allowlist for texture URLs. Supports plain hosts and leading-dot suffix domains. |
| `YggdrasilMetaResp` | `signaturePublickey` | string | PEM RSA public key used to verify signed properties such as `textures` and `uploadableTextures`. |
| `YggdrasilMeta` | `serverName` | string | Display name for the service. |
| `YggdrasilMeta` | `implementationName` | string | Implementation name. Currently `AsterYggdrasil`. |
| `YggdrasilMeta` | `implementationVersion` | string | Current service version. |
| `YggdrasilMeta` | `links` | object? | Site links that authlib-injector can display. |
| `YggdrasilMeta` | `feature.non_email_login` | bool | Whether non-email identifiers are supported. Currently tied to profile-name login. |
| `YggdrasilMeta` | `feature.enable_profile_key` | bool | Whether the server handles the Minecraft profile key certificate endpoint. |
| `YggdrasilMeta` | `feature.enable_mojang_anti_features` | bool | Whether the server handles Mojang anti-features/policy endpoints. |
| `YggdrasilMeta` | `feature.username_check` | bool | Whether authlib-injector should enforce username character checks. Currently fixed to true because Aster profile names are Mojang-valid; future custom naming rules must re-evaluate this flag. |
| `YggdrasilMetaLinks` | `homepage` | string | Site home URL. |
| `YggdrasilMetaLinks` | `register` | string? | Registration URL. Omitted when registration is disabled. |

### Minecraft Services Fields

| DTO | Field | Type | Meaning |
| --- | --- | --- | --- |
| `MinecraftServicesCertificateResp` | `keyPair` | object | Ephemeral RSA keypair used by the client for profile key/chat signing flows. |
| `MinecraftServicesCertificateResp` | `publicKeySignature` | string | Signature field for the public key. Inferred purpose: lets the client verify that the profile key was issued by the service. Current value is an authlib-injector-compatible dummy and has no Mojang trust-chain semantics. |
| `MinecraftServicesCertificateResp` | `publicKeySignatureV2` | string | Second signature field used by newer clients. Inferred purpose: newer signature payload/algorithm compatibility. Current value is also a dummy. |
| `MinecraftServicesCertificateResp` | `expiresAt` | string | RFC3339 timestamp after which the client should stop using this keypair. |
| `MinecraftServicesCertificateResp` | `refreshedAfter` | string | RFC3339 timestamp after which the client should proactively refresh the certificate. |
| `MinecraftServicesKeyPair` | `privateKey` | string | PKCS#1 PEM private key. Returned only to the client; do not store or log it. |
| `MinecraftServicesKeyPair` | `publicKey` | string | PKCS#1 PEM public key. |
| `MinecraftServicesPrivilegesResp` | `privileges` | object | Effective service privileges for the account/profile. |
| `MinecraftServicesPlayerAttributesResp` | `privileges` | object | Same privileges object embedded in the attributes response. |
| `MinecraftServicesPlayerAttributesResp` | `profanityFilterPreferences` | object | Profanity filter preferences. Inferred purpose: affects local client chat filtering UI/behavior. Current value is always off. |
| `MinecraftServicesPlayerAttributesResp` | `friendsPreferences` | object | Friend-system preferences. Inferred purpose: affects social/invite client features. Current friends and acceptInvites values are `DISABLED`. |
| `MinecraftServicesPlayerAttributesResp` | `chatPreferences` | object | Chat preferences. Inferred purpose: controls the client's text communication switch. Current textCommunication value is `ENABLED`. |
| `MinecraftServicesPlayerAttributesResp` | `banStatus` | object | Ban status. Inferred purpose: allows the client to display or disable selected online features. Current bannedScopes is empty; real enforcement must still happen in server-side auth/join flows. |
| `MinecraftServicesPrivileges` | `onlineChat` | object | Whether online chat is allowed. Future chat bans should update this. |
| `MinecraftServicesPrivileges` | `multiplayerServer` | object | Whether joining multiplayer servers is allowed. Future multiplayer bans should update this. |
| `MinecraftServicesPrivileges` | `multiplayerRealms` | object | Whether Realms multiplayer is allowed. |
| `MinecraftServicesPrivileges` | `telemetry` | object | Inferred meaning: required telemetry capability/policy state. Mojang sample returns true; Aster currently returns true as a compatibility declaration. |
| `MinecraftServicesPrivileges` | `optionalTelemetry` | object | Inferred meaning: optional telemetry capability/policy state. Mojang sample returns true; Aster currently returns true as a compatibility declaration. |
| `MinecraftServicesPrivilege` | `enabled` | bool | Boolean state for one privilege. |
| `MinecraftServicesProfanityFilterPreferences` | `profanityFilterOn` | bool | Inferred meaning: whether the client should enable profanity filtering. |
| `MinecraftServicesFriendsPreferences` | `friends` | `ENABLED`/`DISABLED` | Inferred meaning: friend list feature state. |
| `MinecraftServicesFriendsPreferences` | `acceptInvites` | `ENABLED`/`DISABLED` | Inferred meaning: whether friend invites are accepted. |
| `MinecraftServicesChatPreferences` | `textCommunication` | `ENABLED`/`DISABLED` | Inferred meaning: text chat feature state. |
| `MinecraftServicesBanStatus` | `bannedScopes` | object | Inferred meaning: ban scope map. Empty object means no active bans. Future moderation support must use client-recognized scope names, not arbitrary custom names. |
| `MinecraftServicesPrivacyBlocklistResp` | `blockedProfiles` | string[] | Inferred meaning: profile UUIDs blocked by the current user for social/chat blocking. Currently empty. |

## Error Shapes

Ordinary Yggdrasil error body:

```json
{
  "error": "ForbiddenOperationException",
  "errorMessage": "Invalid token."
}
```

Mapping lives in `src/services/yggdrasil_service/error.rs`:

| Kind | HTTP | `error` |
| --- | --- | --- |
| `InvalidToken` | `403` | `ForbiddenOperationException` |
| `InvalidCredentials` | `403` | `ForbiddenOperationException` |
| `ForbiddenProfile` | `403` | `ForbiddenOperationException` |
| `BadRequest` | `400` | `IllegalArgumentException` |
| `AccessTokenAlreadyHasProfile` | `400` | `IllegalArgumentException` |
| `UnsupportedAgent` | `400` | `IllegalArgumentException` |
| `TooManyProfilesRequested` | `400` | `IllegalArgumentException` |
| `ProfileNotFound` | `204` | no body |
| `Internal` | `500` | `InternalServerError` |

Minecraft services endpoints are the exception: non-internal errors map to `401 { "path": "..." }`, while disabled features and unknown paths map to `404 { "path": "..." }`.

## Runtime Config

All keys belong to `CONFIG_CATEGORY_YGGDRASIL`:

| Key | Purpose |
| --- | --- |
| `yggdrasil_server_name` | metadata `meta.serverName`. |
| `yggdrasil_allow_profile_name_login` | Enables profile-name login and drives `feature.non_email_login`. |
| `yggdrasil_allow_skin_upload` | Controls Yggdrasil skin upload and affects the `uploadableTextures` property. |
| `yggdrasil_allow_cape_upload` | Controls Yggdrasil cape upload and affects the `uploadableTextures` property. |
| `yggdrasil_enable_profile_key` | Enables `/minecraftservices/player/certificates` and declares `feature.enable_profile_key`. |
| `yggdrasil_enable_mojang_anti_features` | Enables `/minecraftservices/privileges`, `/player/attributes`, `/privacy/blocklist`, and declares `feature.enable_mojang_anti_features`. |
| `yggdrasil_token_ttl_days` | Yggdrasil access token lifetime. |
| `yggdrasil_max_active_tokens` | Maximum active Yggdrasil tokens retained per user. |
| `yggdrasil_max_texture_upload_bytes` | Per-upload texture byte limit. |
| `yggdrasil_max_texture_pixels` | Decoded texture pixel limit. |
| `yggdrasil_skin_domains` | Extra texture domain allowlist entries. |
| `yggdrasil_public_base_url` | Advanced override used to build public Yggdrasil/texture URLs. |
| `yggdrasil_signature_public_key` | Optional public-key override. |
| `yggdrasil_signature_private_key` | Private key for signing textures properties. Sensitive config. |

When changing config, update:

- `src/config/definitions.rs`
- `src/config/yggdrasil.rs`
- `frontend-panel/src/i18n/locales/*/settings.json`
- OpenAPI/generated types when DTOs expose new contract
- `tests/test_yggdrasil.rs` or config normalizer tests

## OpenAPI And Generated Types

Protocol endpoints do not use the project envelope, but they are still registered in OpenAPI so frontend/debug tooling can see the full contract.

Change workflow:

```bash
cargo fmt
cargo test --features openapi --test generate_openapi
bun run --cwd frontend-panel generate-api
cargo test --test test_yggdrasil
cargo check
```

Documentation-only changes do not need these commands. DTO/route/schema changes should at least run OpenAPI generation and `test_yggdrasil`.

## Test Coverage

When adding or changing Yggdrasil behavior, cover at least:

- Successful response fields and HTTP status.
- Protocol error bodies, ensuring no project envelope leaks into protocol routes.
- Missing, invalid, expired, revoked, and clientToken-mismatched tokens.
- Selected profile ownership and temporary invalidation after rename.
- Signed `textures` property with `unsigned=false`.
- Texture upload content type, dimensions, limits, hash, public read, and delete behavior.
- Minecraft services feature flags, Bearer token auth, 401 path body, and 404 path body.
- Profile key response fields, without requiring official Mojang signatures.

Security-focused tests should verify that access tokens, client tokens, private keys, and profile-key private keys do not appear in logs, errors, audit details, or persisted plaintext fields.
