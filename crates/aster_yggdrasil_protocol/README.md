# aster_yggdrasil_protocol

Project-agnostic Yggdrasil/authlib-injector protocol crate: the wire model
(profiles, properties, request/response bodies, metadata, Minecraft services
compatibility payloads) and the protocol error state machine
(`YggdrasilErrorKind` → wire error name, message, HTTP status).

The crate deliberately contains no database, cache, HTTP-server, or
application policy. Hosts assemble the wire types from their own storage and
configuration, then serve them with any HTTP stack. Session-forwarding, token
lifecycle persistence, and signature key management are product
infrastructure and stay out of scope.

Dependencies are limited to pure wire concerns: `serde`, `serde_json`,
`base64`, `http`, and `validator` (plus optional feature-gated `utoipa` for
schema generation). The `http` version matches the one re-exported by
`actix-web`, so hosts can return `YggdrasilError::status_code()` directly.

Feature flags:

- `openapi`: adds `utoipa::ToSchema` derives for OpenAPI generation
  (`debug_assertions`-gated, matching the application's schema export flow).
