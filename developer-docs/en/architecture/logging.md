# Logging Conventions

This page defines AsterYggdrasil's logging discipline: what goes into logs, what never does, how to pick levels, and how logging differs from audit. New code follows this page; review findings are corrected against it.

The mechanism layer (declarative field contracts, error classification, stable JSON envelopes) is owned by Forge: track [AsterForge#50](https://github.com/AsterCommunity/AsterForge/issues/50) (field contract and envelope) and [#45](https://github.com/AsterCommunity/AsterForge/issues/45) (OTEL tracing and lifecycle). This page is the product-side policy — #50 explicitly does not define product field semantics or privacy policy; that part is ours.

## Logging and audit are two systems

| | Logs (tracing) | Audit (`audit_writer`) |
| --- | --- | --- |
| Audience | Operators (process behavior) | Admins/users (who did what to what) |
| Carrier | stdout / log files, ephemeral | DB tables, queryable in the admin panel |
| Identity | Technical context (task_id, user_id) | actor + entity + action |
| Content | Technical events and failures | Business and security events |

The same event may appear in both, with different fields and audiences. Never substitute logs for audit, and never carry audit content into the log stream.

## Infrastructure

`aster_forge_logging::init_logging` (called from bootstrap, installed once):

- `level` defaults to `info`; `RUST_LOG` takes precedence and a warning names the winner on conflict
- `format`: `text` / `json`
- Empty `file` writes stdout; a non-empty value enables daily rotation with `max_backups` retention
- File writer failures fall back to stdout with a warning; a second subscriber install warns instead of panicking

No per-log configuration is needed; tune `RUST_LOG` when debugging.

## Level semantics

| Level | Use | Examples |
| --- | --- | --- |
| `warn` | Recoverable failures, degradations, ignored input — visible to operators, not page-worthy | mail outbox dispatch failure with retry, config value parse failure skipped |
| `debug` | Routine operation detail; enable via `RUST_LOG` when debugging | login rejections, token creation, cleanup row counts |
| `info` | Process milestones — **at default level, info logs must narrate the process's life** | startup, config load, task enqueue committed, password hash policy upgrade |
| `error` | Failures that should page someone | runtime component startup failure |
| `trace` | Almost never | — |

Baseline: `info` stays rare (15 call sites today, all lifecycle events); `debug` is the workhorse; `warn` covers everything "failed but the process continues". Per-request access logs **do not exist** — request observability is Prometheus metrics (`MetricsMiddleware`) plus the request id middleware. Do not reintroduce `middleware::Logger`-style per-request logging.

## Field discipline

- **Structured fields over string interpolation**: `tracing::debug!(user_id, "login rejected")`, not `tracing::debug!("login rejected for user {username}")`.
- **Numeric IDs only** for identity (`user_id` / `task_id` / `texture_id`); never username, email, or profile names. Human context comes from joining audit tables.
- **Existence over value** for sensitive context: `has_user_agent = user_agent.is_some()`, `has_ip_address = ip_address.is_some()` (the token/session paths already do this).
- **Error objects into warn/error**: `error = %error`; let tracing carry the structure instead of hand-building strings.
- **Lifecycle phase names**: transaction boundaries must be visible — task enqueue distinguishes `record_inserted` (transaction possibly uncommitted) from `enqueue_committed`.

## Never log

The following never appear in logs, error messages, or audit plaintext (matches the AGENTS.md security rules):

- access/refresh/client tokens, session secrets, JWTs, signing private keys
- passwords, MFA secrets, external auth credentials, SMTP/storage credentials
- the **values** of email addresses, IP addresses, user agents (existence booleans are fine)
- business payloads, file names, or storage-key contents beyond texture hashes

**Two known traps new code must avoid:**

1. **Error text embeds input.** `aster_forge_validation::ValidationError`'s Display looks like `invalid email policy domain '{value}'`; serde_json's invalid-type branch embeds value fragments too. Passing `error = %error` through writes the raw input into logs. For inputs containing personal data (emails, player names), log a closed-set classification (e.g. `parse/empty/length/non_ascii/malformed`) plus length/position, never the error text. See `src/config/local_email_policy.rs` and its `redaction_tests`.
2. **Debug output is not a redaction boundary.** `SecretString`'s Debug is redacted, but ordinary structs Debug-print every field — do not use `%?` / `{:?}` on structs carrying user data.

## Redaction regression tests

For any "X must not appear in logs" behavior, write a regression test with `aster_forge_test::logs::CapturedLogs`: capture the log text and assert it does not contain the sensitive original. See the `redaction_tests` module in `src/config/local_email_policy.rs`. Run the matching capture test when changing log fields.

## Mechanism roadmap (Forge)

- **#50 field contract**: field whitelist (undeclared fields never enter records), resource/scope/record JSON envelope with a versioned mapping id, errors as classes/codes, bounded flush-failure counters. This page keeps the product-side field semantics and privacy policy, to be mapped into declarations when it lands.
- **#45 OTEL tracing**: trace provider/propagator/lifecycle and local log correlation. The product owns service identity, sampling trust, and business spans.
- Until both land, the runtime discipline on this page is the entire defense; migrate per their guides afterwards, keeping the product-policy sections of this page.
