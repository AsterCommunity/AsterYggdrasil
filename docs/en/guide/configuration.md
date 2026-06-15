# Config and Keys

AsterYggdrasil separates static config from runtime config.

- Static config lives in `data/config.toml` and controls database, bind address, cache, texture storage, and startup-time settings.
- Runtime config lives in the `system_config` table and controls Yggdrasil policy, public URLs, upload switches, token policy, and signing keys.

## Static Config

Example:

```toml
[server]
host = "127.0.0.1"
port = 3000
temp_dir = ".tmp"
start_mode = "primary"

[database]
url = "sqlite://asteryggdrasil.db?mode=rwc"

[cache]
enabled = true
backend = "memory"

[texture_storage]
backend = "local"
local_root = "textures"
```

Relative paths resolve against the directory containing `data/config.toml`. The default `local_root = "textures"` resolves to `data/textures`.

## Yggdrasil Runtime Config

Common keys:

```text
yggdrasil_server_name
yggdrasil_allow_profile_name_login
yggdrasil_allow_skin_upload
yggdrasil_allow_cape_upload
yggdrasil_token_ttl_days
yggdrasil_max_active_tokens
yggdrasil_max_texture_upload_bytes
yggdrasil_max_texture_pixels
yggdrasil_skin_domains
yggdrasil_public_base_url
yggdrasil_signature_public_key
yggdrasil_signature_private_key
```

Admin Config API:

```text
GET    /api/v1/admin/config
GET    /api/v1/admin/config/schema
PUT    /api/v1/admin/config/{key}
DELETE /api/v1/admin/config/{key}
POST   /api/v1/admin/config/yggdrasil/action
```

Config writes go through typed normalizers and validators. Do not bypass the service layer and write `system_config` directly.

## public base URL

For normal deployments, configure `public_site_url` first:

```json
["https://skin.example.com"]
```

When `yggdrasil_public_base_url` is not configured, the server derives the Yggdrasil API root and texture URLs from the first valid `public_site_url`:

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

`yggdrasil_public_base_url` is an advanced override. It is also a JSON string array:

```json
["https://skin.example.com/api/yggdrasil"]
```

When configured, the server uses the first valid http/https URL to build texture URLs. Base URLs may include a path:

```text
https://skin.example.com/api/yggdrasil/textures/{hash}
```

If neither `yggdrasil_public_base_url` nor `public_site_url` has a usable value, Yggdrasil profile texture responses return a configuration error. Protocol responses do not emit relative texture URLs.

## skinDomains

`yggdrasil_skin_domains` is also a JSON string array. It is an extra texture domain allowlist. authlib-injector validates that texture URL hosts are covered by metadata `skinDomains`.

Rules can be:

- Exact domains, such as `skin.example.com`.
- Dot-prefixed domains, such as `.example.com`.

Metadata responses automatically include Mojang's official domains `.minecraft.net` and `.mojang.com`, plus the current effective texture URL host. Configure `yggdrasil_skin_domains` only when allowing additional CDN or external texture domains.

## Signing Keys

authlib-injector requires the server to sign some profile properties:

- `hasJoined` responses.
- `profile/{uuid}?unsigned=false` responses.

AsterYggdrasil signs with an RSA private key and exposes the public key in metadata.

The private key cannot be changed through the normal config set API. Rotate it with the config action:

```text
POST /api/v1/admin/config/yggdrasil/action
```

Action type:

```text
rotate_yggdrasil_signature_key
```

After rotation:

- Newly generated textures properties use the new private key.
- Metadata derives and returns the new public key.
- Existing tokens do not need to be reissued; signatures are generated when profile properties are built and are not stored in tokens.
- If launchers or servers cached old metadata, verification may fail briefly until metadata is fetched again.

## Sensitive Config

`yggdrasil_signature_private_key` is sensitive:

- It cannot be directly changed from the frontend.
- It must not appear in normal API responses, audit details, or error messages.
- It should be changed only through the rotate action.

`yggdrasil_signature_public_key` mainly exists as a fallback when no private key is available. In normal operation, the public key derived from the private key is authoritative.
