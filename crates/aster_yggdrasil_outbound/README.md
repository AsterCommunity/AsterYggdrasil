# aster_yggdrasil_outbound

AsterYggdrasil outbound HTTP policy, mirroring `aster_drive_outbound`: the
single construction point for every outbound HTTP client the product makes.

A purpose [`Profile`] selects timeout defaults and participates in telemetry
through a tracing observer; `aster_forge_http` keeps redirects and automatic
retries disabled, so transport retries stay an explicit caller decision. TLS
is rustls (`rustls-aws-lc-rs`), matching the workspace's reqwest resolution.
