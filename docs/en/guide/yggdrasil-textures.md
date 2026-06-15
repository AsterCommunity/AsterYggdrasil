# Yggdrasil Textures

AsterYggdrasil has two texture entrypoints.

For launchers and authlib-injector-compatible tools, use the Yggdrasil texture API:

```text
PUT    /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
DELETE /api/yggdrasil/api/user/profile/{uuid}/{skin|cape}
GET    /api/yggdrasil/textures/{hash}
```

For site users, use wardrobe plus profile binding:

```text
GET    /api/v1/wardrobe/textures
POST   /api/v1/wardrobe/textures/{skin|cape}
PUT    /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
DELETE /api/v1/profiles/minecraft/{uuid}/textures/{skin|cape}
```

Yggdrasil protocol endpoints return the protocol response shape and do not use the project API envelope. `/api/v1/...` site and admin APIs continue to use the standard envelope.

## Upload Validation

The uploaded file must be `image/png`. The server reads the PNG header to obtain image dimensions before decoding the full image, so oversized PNG bombs are rejected before the bitmap is loaded.

Accepted dimensions:

- skin: multiples of `64x32`, or multiples of `64x64`.
- cape: multiples of `64x32`, or multiples of `22x17`.

Invalid dimensions return a Yggdrasil protocol error with `IllegalArgumentException`.

## Legacy 22x17 Cape Compatibility

The authlib-injector specification allows legacy capes sized as multiples of `22x17`, but that is not the standard cape canvas. AsterYggdrasil normalizes those capes during upload by padding them with transparent pixels to the matching standard `64x32` canvas:

- `22x17` is stored as `64x32`.
- `44x34` is stored as `128x64`.
- Original pixels are kept from the top-left corner.
- Newly added pixels are fully transparent.

This normalization happens before storage, so metadata, public reads, hashes, reference counting, and orphan cleanup all operate on the processed PNG. Clients always receive a sanitized PNG from the public texture URL.

## Re-encoding and Hashes

The server decodes uploaded PNG files to RGBA and writes them back as PNG. This strips non-bitmap PNG metadata and prevents clients from receiving extra data embedded in user uploads.

The texture hash is calculated from the processed PNG bytes and is used as the final path segment of the public URL:

```text
/api/yggdrasil/textures/{sha256}
```

Minecraft/authlib-injector clients treat the URL filename as the texture identifier for caching, so the same processed image maps to a stable hash.

## Public URL and skinDomains

The texture URL inside the `textures` property must be an absolute URL reachable by clients. For normal deployments, configure `public_site_url`; the server derives `{public_site_url}/api/yggdrasil/textures/{hash}`. If the advanced `yggdrasil_public_base_url` override is configured, the server uses the first valid http/https URL instead.

authlib-injector validates that texture URL domains are included in metadata `skinDomains`. Metadata automatically includes Mojang's official domains `.minecraft.net` and `.mojang.com`, plus the current effective texture URL host. `yggdrasil_skin_domains` is only for additional CDN or external texture domains.

## Storage and Maintenance

Local texture storage is currently supported and is configured explicitly through `texture_storage.local_root`. After a successful upload, only the processed PNG is stored; the raw upload is not retained.

Runtime maintenance tasks include:

- `yggdrasil-texture-cleanup`: deletes texture objects that have no database reference.
- `yggdrasil-storage-consistency-check`: checks whether database texture rows point to missing objects, and whether object hashes still match database records.
- `yggdrasil-token-cleanup`: removes expired or revoked Yggdrasil tokens.

Texture deletion removes the database reference first, then deletes the object only when the reference count reaches zero. If multiple profiles reference the same hash, the object remains available until the last reference is removed.
