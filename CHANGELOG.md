# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Build identity** — `aster_yggdrasil version [--json]` reports the package version, Git revision and working-tree state when known, target, profile, compiled features, and build time without loading instance configuration. Cargo's package scan also refreshes the identity when untracked source files are added or modified without treating `target/` artifacts as source changes. (#68)

### Changed

- **Operator CLI builds** — Default builds, release binaries, and published images include `operator-cli`. A build with `--no-default-features --features server` keeps `serve`, `version`, and piped `hash-password` input. Masked terminal input requires `operator-cli`. (#70)

## [v0.1.0-beta.2] - 2026-11-28

### Release Highlights

**beta.2 is an engineering-quality release.** The frontend panel is structurally aligned with the AsterDrive paradigm — shared data layer, feature directories and quality gates — while the backend gains process role separation and a formal CLI. No protocol or schema changes.

- **Frontend structure migration aligned with AsterDrive** — unified data-acquisition layer, 9 oversized pages split into feature directories, dead code and dead dependencies purged (#52-#59)
- **authlib-injector protocol e2e suite** — authserver / metadata / profile / textures covered by a dedicated Playwright project (22 cases), plus 29 UI cases across 5 core-flow specs
- **UI guidelines with a react-doctor gate** — the skin-station/admin-app paradigm is written down and enforced by `doctor.config.ts`
- **i18n consolidation with a parity gate** — dead label tables removed, plural-aware keys, en/zh key-parity test and a duplicate-key merge guard
- **Backend serve role separation and CLI** — `serve [all|api|worker]`, clap 4 with `hash-password` and `--config-file` / `--data-dir`, and texture preview rendering moved off the request path

### Added

- **Formal clap 4 CLI** — `aster_yggdrasil` now uses a clap 4 command-line framework; invoking it without a subcommand behaves exactly like the historical single-process start. New `hash-password` subcommand: interactive input goes through an inquire masked prompt with retry on invalid input, piped input is read to EOF (`take(4099)` reads one extra byte so a truncated secret is rejected instead of silently accepted) and stripped of CRLF; stdout carries only the Argon2id PHC string and never touches configuration, logs or server resources. Secrets are held in `secrecy::SecretString`, and Argon2 runs on a dedicated `PasswordHashRuntime` instance that does not share the login worker pool.

- **serve role process separation** — `aster_yggdrasil serve [all|api|worker]` (default `all`, fully backwards compatible) trims each process to its role. The `api` role serves HTTP only while background tasks stay queued for a worker; the `worker` role consumes background tasks and mounts only the health probes (`/health` and `/health/ready` for orchestrators, with `RoleHealth` judging ready/live from initialization flags, task-consumer running state and the shutdown signal). Process-level maintenance (system metric sampling, runtime config hot-reload subscription) keeps running in every role, and the shutdown dependency graph is trimmed per role (mail drain and database close only wait on `task_consumers` in consuming roles).

- **CLI `--config-file` / `--data-dir`** — global flags (mutually exclusive, also available as the `ASTER_CLI_CONFIG_FILE` / `ASTER_CLI_DATA_DIR` environment variables). Config parsing, default config generation, relative path anchoring and the crash log directory all follow the instance location resolved by the CLI; with no flags, behavior is identical to the past (`data/config.toml` under the working directory).

- **Basic auth for the Prometheus scrape endpoint** — new `[observability.prometheus.basic_auth]` (`enabled` / `username` / `password_hash`). The password is stored as an Argon2id PHC verification string and checked against an instance-independent Argon2 runtime (default concurrency 2); once the budget is exhausted, requests are rejected immediately without competing with login verification for working memory. Failures return `401` with `WWW-Authenticate: Basic`, `Cache-Control: no-store` and an empty body; successful responses are equally no-store.

- **Configurable Argon2 concurrency** — new `[auth] password_hash_max_concurrency`, default `2` (from `aster_forge_auth::DEFAULT_PASSWORD_HASH_MAX_CONCURRENCY`). Each Argon2 task occupies roughly 64 MiB of working memory, so the default worst-case concurrent footprint is about 128 MiB. Changing it requires a restart.

- **Audit shutdown recovery** — on process shutdown the audit writer completes a final drain; records that remain unacknowledged are written by Forge into the private recovery directory `data/audit-recovery/audit-{uuid}.json` (0600 files, 0700 directory), and shutdown ends with an error reporting the path for manual ops reconciliation. The system never replays these records automatically.

- **Unified frontend data-acquisition layer** — `useCursorList` (semantic cursor stack, generation race guard, single request path via anchor-signature reset, row-level `setItems`/`reloadFirstPage`), schema-driven `useManagedListQueryState` (URL params preserved, invalid params normalized, legacy offset params removed), `useDebouncedValue`, ref-latched `usePendingAction`/`usePendingId`, generic `useConfirmDialog` and a `ttlPromiseCache` primitive. 18 hand-rolled cursor/debounce/URL-sync sites migrated onto the skeleton; error normalization collapsed to one implementation (transport error kind in `http.ts`, message chain in `useApiError`); texture tag cache ownership moved from `yggdrasilService` into `textureTagStore`, leaving the service layer as pure adapters. (#57)

- **Frontend UI guidelines and react-doctor gate** — `frontend-uiux-guidelines.md` codifies the binary paradigm (skin-station user area vs admin app area), entry/dialog taxonomies, motion budget and a state checklist; `doctor.config.ts` enforces it through react-doctor with per-file reasoned exemptions. Guideline violations fixed along the way: `dark:bg-[...]` theme swaps replaced with dual-layer opacity crossfades, public-area colors tokenized via `--public-*` / `--shell-sidebar` tokens, `TextureTagFilterPopover` rebuilt on Base UI (focus trap, aria attributes), dialog Close aria-label localized, region spinners moved to Skeleton. (#54)

- **Frontend test infrastructure** — new `src/test/` support: typed fixtures (API response envelope + entity factories), interpolated mock `t`, deferred promises, auth state seeding, and an MSW server that errors on unhandled frames; `msw` and `@testing-library/user-event` added. The 1577-line `App.test.tsx` is split into colocated test files (all 601 baseline tests kept) and the worst-offender admin/audit tests migrate onto typed fixtures — 668 unit tests total. The mock-vs-network boundary is documented in `src/test/README.md`. (#53)

- **E2E suites** — shared `e2e/support/` layer (API seeding, page auth flows, env-based credentials) and a setup project persisting storageStates for parallel workers. Five UI specs cover auth (9), account (5), admin (4), guards (6) and smoke (5) — 29 cases green. The authlib-injector protocol suite runs on its own Playwright config and port: authserver (6), metadata (2), profile (7), textures (7) — 22 cases green, driven through the request context. (#56)

- **Admin forms onto zod and shared interaction components** — `RefreshButton`/`LoadingButton` in `components/common` replace all 21 spinner-toggle refresh buttons, with the three refresh icon names unified into one `Refresh` entry in the icon registry; `useClipboard` collapses 6 hand-written clipboard call sites; the dialog lifecycle (DialogPhase state machine, exit-complete signals, jsdom-safe motion) is ported from AsterDrive with its full test suite. `validation.ts` gains `inviteUserSchema`/`banFormSchema`, dropping two duplicated `isValidEmail` copies while messages stay key-in-schema for l10n reactivity. (#58)

### Changed

- **Texture previews generated as background tasks (AsterDrive thumbnail paradigm)** — a cache miss on `GET /api/v1/texture-previews/{hash}/{file_name}` no longer renders synchronously in the request path (previously a ~76 ms `spawn_blocking` 3D render inside the api process); the endpoint enqueues an idempotent `texture_preview_generate` background task and returns `202 Accepted + Retry-After: 2`. The worker's dispatch lane renders and writes the result back to object storage, after which the next request returns a 200 immutable PNG. Enqueueing deduplicates against active tasks (only waking the dispatcher), failed tasks are no longer rebuilt by request traffic (the endpoint turns 404, preventing retry storms), and concurrent first requests are covered by the dedupe key. The api process does zero rendering, so `serve api` / worker separation now covers the rendering surface too. The render target (hash + skin model) is decoupled from render parameters (the current runtime config spec); parameter adjustments take effect naturally with the next task.

- **Removed actix per-request access log** — the HTTP middleware chain drops `middleware::Logger::default()` (same as AsterDrive); request observability is carried by Prometheus metrics and request ids. The middleware previously emitted one INFO line containing source paths for every frontend static asset request, producing heavy noise under service worker precaching.

- **Observability config onto the Forge shared validation pipeline** — the static config now uses Forge `aster_forge_metrics::ObservabilityConfig` (deleting the locally copied defaults, redacted Debug, OTLP mapping and provider assembly); the root config derives `aster_forge_config::ValidateConfig` and loads through `aster_forge_config::load_config` (section validation + cross-section deployment rules in one pass; config sections without a compiled exporter are zeroed at deserialization). Metrics identity is provided via `MetricsIdentity`, `service_instance_id` comes from the config-sync runtime id, and the `com.astercosm.yggdrasil` profile namespace and `astercosm-metrics/v1` schema token are unified through Forge constants.

- **Argon2 default concurrency lowered from 8 to 2** — deployments that do not set `password_hash_max_concurrency` explicitly drop their worst-case concurrent Argon2 working memory from ~512 MiB to ~128 MiB per process after upgrading. Configure it explicitly if you need higher concurrency.

- **Forge #90 batch upgrade** — all `aster_forge_*` dependencies upgraded to the published crates.io batch (audit/auth/cache/config/crypto/db/db_migration/external_auth/http/logging/mail/metrics/middleware/observability/panic/runtime/tasks/test/utils). The panic hook's crash-log ancestor directory is now created explicitly by bootstrap (`data/`); Linux keeps the private crash log behavior, while on platforms where Forge private-file mechanics are not yet supported (macOS etc.) startup degrades to unavailable with a warning (fail closed), leaving normal service unaffected.

- **GPL rate-limiting dependency replaced** — `actix-governor` (GPL-3.0-or-later) removed; admin IP rate limiting now uses the GPL-free Actix middleware from Forge `aster_forge_middleware` 0.1.0-alpha.6 (wrapping the MIT OR Apache-2.0 `governor` crate). The 429 response envelope, burst/period semantics, trusted-proxy client IP bucketing and cross-worker shared quota behavior are unchanged. `cargo deny check` licenses passes again without exceptions.

- **Frontend toolchain migrated to TypeScript 7 GA** — `@typescript/native-preview` (tsgo) is replaced by `typescript` 7.0.2, with `build`/`check` running `tsc`. `generate-api` depends on openapi-typescript's TS JS API and now uses `bunx` to temporarily inject `@typescript/typescript6` 6.0.2 for generation while the project itself stays on TS7 (following the AsterDrive pattern). Adapted to react-i18next 18 `TFunction` types, jsdom 31 `getComputedStyle` behavior and biome schema 2.6.8; frontend dependencies fully refreshed (vitest 5, jsdom 31, vite-plugin-pwa 2, i18next 27, react-i18next 18, etc.).

- **Frontend feature-directory migration** — 9 oversized pages split following the admin-users-page pattern (page shell + controller + private components + colocated tests), zero behavior change: AdminSettingsPage 2444 → 13 files (shell 177; draft engine wired to settingsNumberDraftSchema; save-bar/setting-row DOM preserved), TextureWardrobePage 1579 → 11 files (shell 266; 22+ states grouped into the controller by responsibility), AdminTextureLibraryTexturesPage 1445 → 8 files (shell 373), AdminExternalAuthPage and its 887-line provider dialog → 8 files (dialog now rides DialogLifecycle; +5 tests for the previously untested page), AccountSettingsPage 1096 → 6 files (shell 128), MinecraftProfilesPage 1063 → 6 files (shell 33; tests split by domain), LoginPage 899 → login-page/ plus a new RegisterPage (registration is its own route-rendered page; login keeps recovery only), AdminYggdrasilForwardingPage 949 → 6 files (shell 201), PublicTextureLibraryPage 770 → 8 files (shell 274). Page shells stay ≤ 373 lines, no file exceeds 800, page-private components remain inside their page directory, and e2e selectors (settings-save-bar, auth form labels, wardrobe flows) are preserved. (#59)

- **i18n consolidation with parity gate** — the hardcoded 546-line English presentation label table is deleted (pages already consume the t-aware `tasks.ts`/`audit.ts`); broken `t("cancel")` keys fixed to `common.cancel`; duration units rendered through plural-aware `time.json` keys; `document.documentElement.lang` synced with the active locale. The resources merge now guards against duplicate keys (throws with the key path and file names involved) and the silently-shadowed personalSettings subtree is dropped. Six dead en `_plural` keys fixed to `_one`/`_other` (counts other than 1 previously rendered singular). `keyParity.test.ts` (9 cases) enforces that stripped-plural key sets match between en and zh, with real regressions covering cancel/duration/plurals. (#55)

- **Dead code and dependencies purged** — zero-consumer hooks (`useApiList`, `useAsyncTask`) and `AdminLayout` removed; router alias files deleted with `App.test.tsx` importing the guards directly; `react-devicons` and `xml-formatter` dropped along with their vite chunk/inline/precache branches; unused data-theme color presets removed from `index.css`. shadcn stays: its CSS export is the Tailwind 4 theme base (imported by `index.css`). (#52)

### Fixed

- **Frontend race and dialog-lifecycle hardening** — review rounds across the migration: profile texture loads guarded by a generation counter so stale responses can no longer overwrite the selected profile's textures or loading flag; admin keyword drafts no longer overwrite restored URL filters (URL stays authoritative until the user edits; resets adopt URL state); settings labels associated with their controls (`htmlFor` / `fieldset` + `aria-labelledby`), enum toggle buttons expose `aria-pressed`, the display unit is pinned after a successful conversion, and signature-key rotation is gated while unsaved drafts exist (with a bilingual dirty hint); texture library detail loads are numbered so a slow response for texture A cannot replace texture B's detail or loading state; `usePendingAction`/`usePendingId` use operation tokens so a cleared action's `finally` can never release the lock belonging to a newer action; external-auth saves carry their dialog session so late completions cannot close a newer dialog or wipe its drafts; settings save switched to `Promise.allSettled` (partial success commits fulfilled configs without resubmission and keeps failed drafts); reviews-mode enum filters accept `__all__` so user-selected values survive URL round-trips; wardrobe edit save is session-scoped end to end (late metadata completions only affect their own session; a failing tag write keeps the dialog open with the draft intact and retries only the tag write; skip-reuse compares the request payload); forwarding form updates send only fields changed from the edit-session baseline; `useCursorList` reloads instead of short-circuiting when disabled mid-flight. (#64)

### Security

- **Timing enumeration fix for unknown-account login** — virtual Argon2 verification for unknown identifiers now uses the same wait semantics as existing accounts (`verify_password`). It previously used the non-blocking `try_verify_password`, which returns immediately when the permit pool is saturated, letting an attacker saturate it with concurrent requests and distinguish account existence from response timing. The lowered default concurrency makes the trigger threshold easier to reach.

- **Frontend dependency supply-chain hardening** — fixed the `brace-expansion` (5.0.11), `fast-uri` (3.1.7) and `ip-address` (10.7.6) transitive dependency vulnerabilities; `bun audit --audit-level=high` is back to zero. New `frontend-panel/bunfig.toml` sets a 1-day release cooldown (`minimumReleaseAge = "1d"`): `bun install`/`bun update` no longer auto-resolves versions published less than 24 hours ago, with per-package exemption via `bun update --minimum-release-age=0 <pkg>` for emergencies.

## [v0.1.0-beta.1] - 2026-06-20

### Release Highlights

**AsterYggdrasil enters the beta stage!** After 6 alpha iterations the core functionality (full Yggdrasil protocol, skin station, multiple login methods, texture library review workflow, fine-grained operator management) has stabilized; this release moves to the beta channel, focusing on operational governance and extensibility.

- **User capability bans** — administrators can precisely ban specific user capabilities per scope (Yggdrasil authentication / joining servers / profile management / texture upload / texture library interaction), permanent or time-limited, with public reasons, revocation and append-only event auditing, without disabling the whole account
- **Yggdrasil upstream session forwarding** — `hasJoined` verification can orchestrate local and remote (including genuine Mojang) Yggdrasil-compatible session servers, probing by priority/weight with optional upstream texture proxying, supporting hybrid authentication architectures
- **System-wide cursor pagination** — all list endpoints migrate from offset to cursor pagination, significantly improving performance under large datasets (composite indexes + fixed ordering)
- **External authentication enhancements** — email supplemental verification, password binding, auto-provisioning/linking, email domain allowlists, Microsoft tenant, OIDC claim mapping; third-party login becomes more flexible and controllable

### Breaking Changes

- **List endpoints unified onto cursor pagination**: audit logs, background tasks, users, profiles, textures, sessions, passkeys, invitations and other list endpoints migrate from offset pagination (`{ items, limit, offset, total }`) to cursor pagination. Cursors are based on the composite key `(created_at, id)` (some tables `(updated_at, id)`); responses gain `next_cursor` / `has_more` fields; query parameters drop `sort_by` / `sort_order` in favor of a fixed `created_at DESC, id DESC` ordering, with pagination driven by `limit` + `after_created_at` / `after_id` (or a string cursor). Migration `m20260619_000001_cursor_pagination_indexes` adds composite indexes to the affected tables to support cursor queries. Third-party scripts and custom clients calling list endpoints with the old offset / sort parameters must adapt to the new response shape and query parameters.

### Added

- **User capability bans**: administrators can issue fine-grained capability bans against users, precisely blocking specific operations per scope without disabling the whole account. There are 5 ban scopes: `yggdrasil_access` (Yggdrasil authentication / token validate), `yggdrasil_join` (session join / hasJoined), `minecraft_profile_manage` (profile rename), `texture_upload` (skin / cape upload), `texture_library_interact` (public library copy / submit / report). Ban status `UserBanStatus`: `active` / `revoked` / `expired`; responses additionally provide `effective_status` (`active` but past `expires_at` returns `expired`) and an `effective` boolean. Bans support a time window `starts_at` / `expires_at` (empty `expires_at` means permanent), `reason` (required, ≤128 chars), `public_reason` / `admin_note` (optional, ≤1000 chars) and `revoke_note`. Admin APIs: `GET /api/v1/admin/user-bans` (cursor pagination, filterable by `user_id` / `scope` / `status` / `effective_only`), `GET /api/v1/admin/user-bans/{ban_id}`, `POST /api/v1/admin/users/{user_id}/bans` (creates, building the ban + append-only `created` event inside one transaction), `PATCH /api/v1/admin/user-bans/{ban_id}` (updates, only active bans, verifying no other active ban of the same scope before changing scope), `POST /api/v1/admin/user-bans/{ban_id}/revoke` (revoke + `revoked` event), `GET /api/v1/admin/user-bans/{ban_id}/events` (append-only event history). Enforcement: `yggdrasil_access` intercepts `authenticate` / `validate`, `yggdrasil_join` intercepts `join` / `hasJoined` / Minecraft Services, `minecraft_profile_manage` intercepts profile renames, `texture_upload` / `texture_library_interact` intercept wardrobe uploads and public library copy / submit / report, hits returning `user_ban.forbidden` (in the `auth_forbidden` family). Audit actions `AdminCreateUserBan` / `AdminUpdateUserBan` / `AdminRevokeUserBan`, new entity type `UserBan`, and a new `UserDetailBanSection` on the frontend user detail page (en/zh i18n). Migration `m20260620_000002_user_bans` creates two tables: `user_bans` (with `user_id` CASCADE / `created_by` / `revoked_by` SET NULL FK groups, indexes `idx_user_bans_user_status` / `idx_user_bans_status_created` / `idx_user_bans_expires_at`) and append-only `user_ban_events` (`ban_id` CASCADE / `actor` SET NULL, recording created / updated / revoked with before-after state / scope / expiry, indexes `idx_user_ban_events_ban_created` / `idx_user_ban_events_actor_created`).

- **Yggdrasil upstream session forwarding**: `sessionserver/hasJoined` verification can orchestrate local and remote Yggdrasil-compatible session servers. New table `yggdrasil_session_forward_servers` (migration `m20260620_000001`, seeded with a local provider plus one disabled Mojang remote entry). The session forward service selects servers by `priority` / `weight` to call upstream hasJoined, optionally proxying upstream textures via the new `forwardedTextures` route on a hit. Admin CRUD APIs `GET/POST /api/v1/admin/yggdrasil/session-forward-servers`, `GET/PATCH/DELETE /api/v1/admin/yggdrasil/session-forward-servers/{id}` (all governed by the `settings` operator scope); the frontend gains `AdminYggdrasilForwardingPage` (ordering, health status, enable / texture-forward toggles). Audit: forwarding server create / update / delete and every forwarding check are audited (en/zh i18n). New documentation page "Yggdrasil Forwarding" (en/zh) wired into the docs site navigation.

- **External authentication enhancements**: external auth identities gain a local verification flow for already-verified emails (when the provider callback carries no verified email, local email supplemental verification kicks in); password-based binding of external identities; new auto-provisioning / auto-linking config switches, per-provider allowed email domains filtering, Microsoft tenant configuration (`consumers` / `organizations` / `common` / custom), custom OIDC claim mappings; external auth success provides a redirect toast.

- **Cursor pagination infrastructure**: new `DateTimeIdCursor` / `DateTimeStringCursor` cursor types and the `CursorPage` response structure (`items` / `total` / `limit` / `next_cursor` / `has_more`), providing a unified abstraction for system-wide cursor pagination of list endpoints.

- **Audit action and error code extensions**: new audit actions `admin_create_user_ban` / `admin_update_user_ban` / `admin_revoke_user_ban` (group `admin`) plus Yggdrasil session forwarding server management and forwarding check actions; new entity type `UserBan`. New error codes: `user_ban.not_found` / `user_ban.already_active` / `user_ban.not_active` / `user_ban.duration_invalid` / `user_ban.reason_invalid` / `user_ban.forbidden`.

### Changed

- **Dependency upgrades and MSRV bump**: `sysinfo` to 0.39, `rust-version` (MSRV) raised to 1.95.0; along the way `aws-sdk-s3` / `redis` / `sea-orm` / `sea-orm-migration` / `@base-ui/react` / `@types/node` / `@typescript/native-preview` upgraded, and the unused `papaparse` dependency and its vite chunk removed.

---

**Statistics**:
- 213 files changed, 20810 insertions(+), 4222 deletions(-)
- 4 commits (3 features + 1 dependency upgrade)
- 3 new database migrations (`m20260619_000001` cursor pagination indexes, `m20260620_000001` `yggdrasil_session_forward_servers`, `m20260620_000002` `user_bans` + `user_ban_events`)

## [v0.1.0-alpha.6] - 2026-06-19

### Breaking Changes

- **Storage backends unified; legacy avatars and local texture paths need migration**: texture and avatar storage are merged onto a single `ObjectStorage` backend. (1) avatar storage_keys are uniformly prefixed with `avatar/` (`avatar/user/{id}/v{version}/{size}.webp`); after upgrading, avatars with old keys in the database become unreadable (returning `AvatarNotFound`) — **users must re-upload avatars**; (2) the `config.toml` section `[texture_storage]` is renamed to `[object_storage]` (the old name still loads via `#[serde(alias = "texture_storage")]`), and the default `local_root` changes from `textures` to `storage` — **before upgrading a local deployment, migrate the `textures/` directory to `storage/`, or set `local_root` explicitly**; (3) avatar storage now also supports S3/MinIO (previously local only).

- **Admin authorization model change**: all `/api/v1/admin/*` endpoints no longer require only the `Admin` role; they now accept `Admin` **or** an `Operator` holding the corresponding scope. Third-party scripts using a plain admin token are unaffected; new operator accounts can access the subsets of previously admin-only endpoints granted to them.

- **Admin avatar endpoint path migration**: `GET /api/v1/admin/users/{id}/avatar/{size}` → `GET /api/v1/admin/avatars/users/{id}/{size}`; callers must update paths (the frontend `adminService` is updated in the same change).

- **System config categories regrouped**: `site` / `auth` / `network` / `audit` / `runtime` / `yggdrasil` are split into multiple sub-categories (e.g. `auth.session` / `auth.registration` / `auth.recovery` / `auth.login` / `auth.email_policy` / `yggdrasil.metadata`); the admin settings UI categories and the `SYSTEM_CONFIG_ALLOWED_CATEGORIES` whitelist change accordingly — external scripts querying by old category names must adapt.

### Added

- **Public texture library**: a public skin / cape browsing and adoption system for all users. Public browsing requires no login: `GET /api/v1/texture-library/tags` (tag list, keyword search + pagination), `GET /api/v1/texture-library/textures` (public texture list with `keyword`, `texture_type`, `tag_ids` (comma-separated, up to 16), `tag_search_method`=`all`|`any` defaulting to `all`), `GET /api/v1/texture-library/textures/{texture_id}` (detail). The public library filter is `is_wardrobe_item = true AND visibility = Public`. Logged-in users can `POST /api/v1/texture-library/textures/{texture_id}/copy` to copy a public texture into their wardrobe (body `CopyPublicTextureReq { display_name?: NullablePatch<String> }`; the backend reuses an existing wardrobe entry on hash + user_id + texture_type + model match, otherwise creates a `visibility=Private` copy carrying all of the source texture's tag bindings). Wardrobe endpoint extensions: `GET /api/v1/wardrobe/tags`, `PATCH /api/v1/wardrobe/textures/{texture_id}` (update display_name / visibility), `PUT /api/v1/wardrobe/textures/{texture_id}/tags` (batch-replace tags), and the wardrobe upload `POST /api/v1/wardrobe/textures/{texture_type}` multipart gains optional `name` / `display_name` fields. Data-layer migration `m20260618_000004_texture_library_metadata`: adds `display_name` (VARCHAR 96, null), `library_status` (VARCHAR 24, default `private`, index `idx_minecraft_textures_library_status`) to `minecraft_textures`; creates `minecraft_texture_tags` (`normalizedName` unique, `color` default `#64748b`, `sortOrder`, indexes `idx_minecraft_texture_tags_name_unique` / `idx_minecraft_texture_tags_sort`) and `minecraft_texture_tag_binding` (bidirectional CASCADE, composite unique `idx_minecraft_texture_tag_bindings_unique(texture_id, tag_id)`).

- **Texture library review workflow**: the `MinecraftTextureLibraryStatus` state machine `private` / `pending_review` / `published` / `rejected`. User submission: with `review_required=true` it goes `private`→`pending_review`, otherwise directly →`published` (writing `library_reviewed_at` immediately, reviewer left empty); withdrawing returns to `private` and clears the timestamps. Administrator (`RequireAdminOrScope(OperatorScope::TextureLibrary)`) endpoints: `GET /api/v1/admin/texture-library/textures` (filterable by keyword / texture_type / visibility / library_status / published / tag_ids / tag_search_method), `GET /api/v1/admin/texture-library/textures/{texture_id}`, `POST .../approve` (body `ReviewTextureLibraryTextureReq { review_note?, tag_ids? }`, requires `visibility=public`), `POST .../reject` (`review_note` required, ≤512 chars), `POST .../unpublish` (note optional, side effect: calls `handle_pending_for_texture` to auto-accept all `pending` reports for the texture). Migration `m20260618_000006_texture_library_review` adds `library_submitted_at` / `library_reviewed_at` / `library_reviewer_user_id` (i64 null) / `library_review_note` (text null) to `minecraft_textures` plus indexes `idx_minecraft_textures_library_public` / `idx_minecraft_textures_library_review_queue` / `idx_minecraft_textures_user_library_status`.

- **Texture library report workflow**: new table `minecraft_texture_reports` (migration `m20260618_000007_texture_library_reports`, three FKs: texture CASCADE / reporter CASCADE / handler SET NULL, indexes `idx_texture_reports_status_created` / `idx_texture_reports_texture_status` / `idx_texture_reports_reporter_status`). `MinecraftTextureReportReason`: `inappropriate` / `offensive` / `copyright` / `misleading` / `broken` / `spam` / `other`; `MinecraftTextureReportStatus`: `pending` / `accepted` / `rejected`. User endpoint `POST /api/v1/texture-library/textures/{texture_id}/reports` (body `CreateTextureReportReq { reason required, message? ≤1000 chars }`), with preconditions: texture must be `published` + `visibility=public`, no self-reporting, one `pending` report per user per texture. Admin endpoints `GET /api/v1/admin/texture-library/reports`, `GET .../reports/{report_id}`, `POST .../reports/{report_id}/accept` (side effect: unpublishes the reported texture, `library_status`→`private`, i.e. accept = auto unpublish), `POST .../reports/{report_id}/reject` (changes only the report status); accept/reject both require the report to be `pending`.

- **Wardrobe → public library submission workflow**: regular users submit wardrobe textures into the public library, mounted under the `/api/v1/wardrobe` scope — `POST /api/v1/wardrobe/textures/{texture_id}/library-submission` (preconditions `texture_library_enabled=true`, texture must be a wardrobe item of the current user with `visibility=public`; enters `pending_review` or `published` directly depending on `texture_library_review_required`), `DELETE /api/v1/wardrobe/textures/{texture_id}/library-submission` (withdraw, clearing review fields back to `private`).

- **Operator role and fine-grained management scopes**: `UserRole` gains `Operator` (DB value `"operator"`), forming the `Admin` > `Operator` > `User` hierarchy. The `OperatorScope` enum has 8 values (snake_case, DB length 32): `overview` / `users` / `profiles` / `texture_library` / `audit` / `tasks` / `settings` / `external_auth`. Migration `m20260618_000005_operator_scopes` creates the `user_operator_scopes` table (`userId` + `scope` + composite unique `(user_id, scope)`, FK→users CASCADE). New auth middleware `RequireAdminOrScope(OperatorScope)`: `role.is_admin() || (role == Operator && operator_scopes.contains(scope))`, otherwise `auth_admin_required`; each `/admin/*` sub-route mounts its own scope. Security rule `normalize_role_and_scopes`: a request carrying both `role=Admin` and `operator_scopes` is automatically downgraded to Operator (avoiding scoped super admins); role / status / scope modifications and deletion of the super admin (`id=1`) are always rejected; `AuthUserInfo` loads scopes only for `Operator`. The frontend drives navigation from the `operator_scopes` field of `GET /auth/check`.

- **Self-hosted image captcha system**: JPEG base64 rendered locally via `captcha-rs` (180×54~64 px), no third-party service; the character set excludes easily confused characters (only `2-9 A C D E F G H J K M N P Q R T W X Y`). Verification is triggered per-endpoint by `RuntimeCaptchaPolicy` on `POST /auth/login`, `POST /auth/register`, `POST /auth/users/invitations/{token}/accept`, `POST /auth/register/resend-activation` via `verify_if_required`. Challenge endpoints (public): `GET /auth/captcha/policy` (returns `PublicCaptchaPolicyResp`, each `*_required` = `enabled && sub-switch`), `POST /auth/captcha` (returns `CaptchaChallengeResponse { challenge_id, image_base64, mime: "image/jpeg", expires_in }`). Challenges are stored under cache key `auth:captcha:{challenge_id}` (SHA-256 answer hash + `attempts`/`max_attempts`/`expires_at`), deleted after the failure limit, answers normalized (whitespace stripped + uppercased). Config group `captcha`: `auth_captcha_enabled` (default `false`), `auth_captcha_login_required` / `auth_captcha_register_required` / `auth_captcha_invitation_accept_required` / `auth_captcha_register_activation_resend_required` (all default `true`), `auth_captcha_preset` (`readable` / `balanced` default / `hardened`), `auth_captcha_ttl_secs` (default 120), `auth_captcha_length` (4–8, default 5), `auth_captcha_max_attempts` (1–10, default 3). New config action `preview_captcha` lets the admin panel preview images under different parameters (no audit write). The frontend wires `useCaptchaChallenge` + a `CaptchaField` component into the login / invitation / register / resend-activation forms.

- **CPU texture preview rendering**: new standalone crate `aster-texture-renderer` (CPU-only, no GPU dependency) provides 2D / 3D preview PNGs for the texture library. Public endpoint `GET /api/v1/texture-previews/{hash}/{file_name}` returns `image/png` (via `conditional_bytes_response`). New `texture_preview_*` runtime config: `texture_preview_engine` (`skin-3d` default / `skin-2d`), `texture_preview_quality_profile` (`fast` / `default` / `quality`), `texture_preview_width` / `texture_preview_height` / `texture_preview_background` / `texture_preview_show_outer_layer`, plus 3D parameters (`scale` / `pitch` / `front_yaw` / `back_yaw` / `spacing` / `x_offset` / `y_offset` / `center_y` / `supersampling`) and 2D parameters (`padding` / `spacing`).

- **Audit action and error code extensions**: new audit actions `minecraft_texture_library_submit` / `_withdraw` (group `user`), `minecraft_texture_library_approve` / `_reject` / `_unpublish` (group `admin`), `minecraft_texture_report_create` (group `user`), `minecraft_texture_report_accept` / `_reject` (group `admin`). New error codes: `auth.captcha_required` / `auth.captcha_invalid` / `auth.captcha_expired`, `wardrobe.texture_name_invalid` / `wardrobe.texture_name_taken`, `texture_library.tag_not_found` / `tag_name_invalid` / `tag_color_invalid` / `tag_name_taken` / `texture_not_found` / `disabled` / `texture_not_public` / `texture_not_pending` / `texture_not_published` / `review_note_invalid`, `texture_report.texture_not_reportable` / `self_report_not_allowed` / `pending_exists` / `message_invalid` / `not_found` / `not_pending`.

- **Texture library runtime config**: new `texture_library_enabled` (default `true`) and `texture_library_review_required` (default `true`), exposed to `/api/v1/frontend-config` via `PublicTextureLibraryConfig`; the frontend uses them to control sidebar / footer / homepage CTA visibility.

### Changed

- **Storage backend unified (object_storage)**: the `TextureStorage` trait is renamed `ObjectStorage`, metadata struct `TextureBlobMetadata` → `ObjectBlobMetadata`, factory `create_object_storage()` accepts `""` / `local` / `s3` / `minio`; `AppState.texture_storage` → `object_storage`, runtime trait `TextureStorageRuntimeState` → `ObjectStorageRuntimeState`. `avatar_storage.rs` slims from 174 to 74 lines, dropping local directory cleanup logic (`cleanup_empty_avatar_dirs` / `normalize_absolute_path`) in favor of `object_storage().put_file` / `get_stream` / `delete`. Error code `MinecraftTextureStorageFailed` → `MinecraftObjectStorageFailed`; the wire code string `minecraft_texture.storage_failed` is unchanged (protocol-level compatibility). No schema migration; the config-layer change is cleaned up at startup via `ensure_defaults`'s `delete_deprecated_keys()`. See Breaking Changes above for upgrade impact.

- **Wardrobe texture metadata response structure**: `MinecraftWardrobeTextureMetadata` gains `name` / `display_name` / `library_status` / `tags` / `preview_url` fields; the upload multipart gains optional `display_name` / `name` fields.

- **PWA update detection enhanced**: `usePwaUpdate` extracts `requestServiceWorkerUpdate(reg, reason)` (debug logging per `registered` / `interval` / `visibility` reason, capturing `update()` failures), checks once immediately after successful registration, adds a `visibilitychange` foreground check, and the effect teardown cleans up the interval and listeners; `vite.config.ts`'s `VitePWA.registerType` changes from `prompt` to `autoUpdate`, applying new versions automatically instead of asking the user (this also fixes the old registration interval never being cleaned up).

- **conditional_bytes_response gains a `Content-Length` response header**, affecting client caching and range request behavior for all bytes responses (avatar / static assets / texture / default skins).

- **New system config value type `StringEnum`** (with `string_enum_set` storage) for single-choice enums like the captcha preset; external tools reading the `system_config` table directly must be aware of the new type.

- **Orphan texture cleanup matching tightened**: `maintenance.rs` only matches storage keys consisting of a valid two-digit hex prefix + a 64-digit hex hash, avoiding accidental deletions.

- **Robustness hardening**: the `src`, `main`, `migration` and `aster-texture-renderer` crates enable `clippy::panic` deny; `expect` / `unwrap` / `panic` in shutdown / mail sender / rate limiter / external auth providers / config helpers are replaced with error propagation.

### Removed

- Removed the system config entries `avatar_dir` (with `normalize_avatar_dir_config_value`, `DEFAULT_AVATAR_DIR`), auto-cleaned at startup by `delete_deprecated_keys()`; the avatar root directory is now governed solely by `[object_storage]`.

---

**Statistics**:
- 275 files changed, 33,142 insertions(+), 3,866 deletions(-)
- 4 commits (3 features + 1 refactor)
- 4 new database migrations (`m20260618_000004` ~ `m20260618_000007`), new tables `minecraft_texture_tags` / `minecraft_texture_tag_binding` / `user_operator_scopes` / `minecraft_texture_reports`, new fields `minecraft_textures.display_name` / `library_status` / `library_submitted_at` / `library_reviewed_at` / `library_reviewer_user_id` / `library_review_note`

## [v0.1.0-alpha.5] - 2026-06-18

### Added

- **S3 / MinIO storage backend**: when `texture_storage.backend` is set to `s3` or `minio` (equivalent), `S3TextureStorage` is enabled; `local` keeps the local implementation. Based on `aws-sdk-s3`, compatible with any S3-compatible service such as AWS S3 / MinIO / RustFS (via `endpoint` + `force_path_style`), default region `us-east-1`, timeouts connect 5s / read 30s / operation 60s. New config `[texture_storage.s3].base_path` (in-bucket prefix, default empty). **Key behavior: storage_keys in the database never contain the base_path prefix** — the S3 backend joins `base_path/key` dynamically on every object storage call and still returns the bare key upstream, so local and S3 backends can switch losslessly. Uploads use server-side streaming (`ByteStream::from_path`, content-type fixed `image/png`) with no presigned-URL client uploads. Key sanitization rejects absolute paths / `\` / trailing slashes / `..` / empty segments. 404s map uniformly to `record_not_found`; other SDK errors are logged with the full `http_status`/`code`/`message`/`source chain`. Integration tests verify with rustfs testcontainers.

- **Admin overview workbench**: new `GET /api/v1/admin/overview` (`JwtAuth` + `RequireAdmin`) returning summary / service status / system health / activity trends / recent activity / system info. Summary contains 7 counters (total users, Minecraft profiles, textures, active sessions, active Yggdrasil tokens, processing / pending background tasks); service status contains 5 fixed checks (database / yggdrasil / session / texture_storage showing the backend name / background_tasks, degraded to warning when pending>0); activity trends are fixed 7-day UTC daily aggregates with 4 metrics per day (distinct active users, active players by Yggdrasil auth actions, new textures, Yggdrasil API call counts); system health merges the latest `SystemHealthCheck` and `YggdrasilStorageConsistencyCheck` task results (worst-status merge, failed→Unhealthy, pending→Degraded). The frontend gains `AdminOverviewPage` (route `/admin`: hero health status + health banner + summary cards + trend chart + recent audit + service status + quick entries + system info) and `OverviewTrendChartContent` (Recharts line chart, 4 lines lazy-loaded), plus the `StatusIndicator` component and en/zh i18n.

- **CDN / object storage texture URLs**: new runtime config `yggdrasil_texture_public_base_url` (default empty, `requires_restart=false`). For uploaded textures (with a storage_key), the textures property URL prefers `{texture_public_base_url}/{storage_key}`; if unset it falls back to the existing `{public_base_url}/textures/{hash}` or `/api/yggdrasil/textures/{hash}`; if all three are empty a config error is raised. The `texture_public_base_url` host is automatically added to the skinDomains allowlist (deduplicated). Combined with an S3/MinIO backend, textures can go straight through a CDN / object storage public domain, bypassing the Yggdrasil API routes.

- **Generic HTTP base URL normalization utility**: new `utils/url.rs::normalize_http_base_url` (trim + trailing-slash removal + http/https + host validation, optionally forbidding query/fragment). `normalize_public_base_url`, `normalize_required_public_base_url`, `normalize_gravatar_base_url_config_value` and the S3 `endpoint` normalization all switch to it.

- **Audit log activity aggregate index**: new migration `m20260618_000003_audit_log_activity_indexes` creates the composite index `idx_audit_logs_action_created_user(action, created_at, user_id)` on `audit_logs`, covering aggregate queries like the admin overview's 7-day activity trend `COUNT(DISTINCT user_id) WHERE action IN (...) AND created_at BETWEEN ...` and avoiding full table scans.

- **Docs site navigation regrouped**: the docs site is reorganized from 2 groups into 6 (Getting started / Player usage / Protocol integration / Administration / Deployment / Project reference), with new bilingual pages: usage guide overview, admin guide, quick FAQ, troubleshooting (routed by symptom), launcher setup, documentation contribution guide, and deployment overview (pre-launch checklist / public URLs / reverse proxy / persistence / backup / acceptance). `storage.md`'s S3/minio section is completed from "reserved, not implemented" into full usage documentation (base_path, CORS requirements, CDN URLs), and `configuration.md` documents `yggdrasil_texture_public_base_url` with its CORS requirements.

### Changed

- **Storage consistency check now validates only the storage key format** (performance + security): the old implementation called `exists()` + `get_stream()` per texture and re-hashed SHA-256 comparisons (N streaming reads); the new implementation pulls the full key set with one `list_keys("")` + `HashSet` membership, and hash comparison becomes validating only that storage_key matches the `{hash first 2}/{hash}.png` format. Avoids N+1 head/get_object calls under S3 backends with a large speedup. `TextureErrorKind::Storage`'s protocol_message is unified to "Texture storage failed.", no longer leaking endpoint/bucket internals (security hardening).

- **Textures property URL priority change**: uploaded textures now prefer object storage URLs — after configuring `yggdrasil_texture_public_base_url` the path shape changes from `/textures/{hash}` to `{base}/{storage_key}`; properties cached by clients / launchers refresh naturally, and skinDomains automatically covers the new host.

- **Base URL validation tightened (breaking)**: `public_site_url`, `yggdrasil_public_base_url`, the gravatar base URL and the S3 endpoint now uniformly reject URLs carrying a query or fragment (previously public_base_url silently ignored them). Existing configurations with `?` / `#` now raise validation errors on save.

- **gravatar base URL blank falls back to default**: runtime blank values now fall back to `https://www.gravatar.com/avatar`, previously only trimmed to an empty string.

- **Consistency check failure summary enhanced**: failure summaries now list up to 5 concrete issues (`missing object: texture #N, key K, expected hash H` / `hash/key mismatch: ...`), with "and N more" beyond.

- **Failed runtime task health status fallback**: when `system_health` is None the error status text is used as fallback instead of showing an empty state.

### Fixed

- **External auth provider query N+1**: `external_auth_provider_repo` gains `find_by_ids`; `list_links_paginated` changes from `find_all` (full table scan) + in-memory filtering to a single `WHERE id IN (...)` query, significantly improving performance when the provider table is large.

---

**Statistics**:
- 126 files changed, 8,042 insertions(+), 401 deletions(-)
- 3 commits (1 feature + 1 refactor + 1 docs)
- 1 new database migration (`m20260618_000003`)

## [v0.1.0-alpha.4] - 2026-06-18

### Added

- **Email registration activation flow**: `POST /auth/register` branches on `auth_policy.register_activation_enabled` — when enabled and not the first user, an `Active` user with an empty `email_verified_at` is created, a `RegisterActivation` verification token is issued and an activation email is delivered via outbox inside the transaction, returning `{ expires_in: 0, requires_activation: true }` without setting cookies; the first user (auto-admin) and deployments with activation disabled keep the old immediate-login path. New `POST /auth/register/resend` (with `register_activation_resend` throttling, responses do not leak account existence), and `GET /auth/contact-verification/confirm` dispatches by `VerificationPurpose`: `RegisterActivation` activates the account, writes a `UserConfirmRegistration` audit and 302-redirects to `/login?contact_verification=register-activated`; `ContactChange` submits `pending_email`; `PasswordReset` is not consumed here.

- **Password reset and change**: new `POST /auth/password/reset/request` (sends a `PasswordReset` token by email, throttled by `auth_password_reset_request_cooldown`, fixed response text not leaking account existence), `POST /auth/password/reset/confirm` (consumes the token, 410 when expired, writes `UserConfirmPasswordReset`), `PUT /auth/password` (logged-in change, requires a verified email, validates the current password, forbids old==new; inside one transaction `update_password_in_connection` + `revoke_sessions_for_user` re-issuing the token; logic centralized in the new `auth_service/password_change` module, audit `UserChangePassword`). Frontend gains `/reset-password` and `/force-password-change` routes with status banners.

- **User invitation system**: admin-only. New table `user_invitations` (unique `token_hash`, status pending/accepted/expired/revoked, `invited_by` Cascade, `accepted_user_id` Set Null, four index groups), TTL controlled by the new config `auth_user_invitation_ttl_secs` (default 7 days). Admin endpoints `POST/GET /admin/users/invitations` and `POST /admin/users/invitations/{id}/revoke`; on creation the transaction normalizes the email + validates the local email policy + generates the token + revokes older pending invitations of the same email + delivers the invitation mail via outbox, with the response carrying `invitation_url` and `mail_queued`; public endpoints `GET /auth/invitations/{token}` (no login, returns email / expires_at), `POST /auth/invitations/{token}/accept` (no login; inside one transaction validates pending/not-expired/email policy/email availability, creates an `Active` regular user with a verified email, CAS `mark_accepted_if_pending` guards concurrency, audits `UserRegister`, HTTP 201). Error codes `AuthInvitationInvalid/Expired/Accepted/Revoked`. Audit actions `AdminCreateInvitation`/`AdminRevokeInvitation`, new `AuditEntityType::Invitation`.

- **Invitation mail template**: new `MailTemplateCode::UserInvitation` (`user_invitation`), template files `user_invitation.html` + `user_invitation.subject.txt`, placeholders `{{email}}`, `{{invitation_url}}`, `{{site_name}}`, `{{expires_in}}`; new configurable entries `mail_template_user_invitation_subject` (String) and `mail_template_user_invitation_html` (Multiline), editable in the admin panel's mail template group, with expiry rendered via `format_mail_duration_seconds`.

- **Contact verification token infrastructure**: new table `contact_verification_tokens` (`channel` email / `purpose` register_activation|contact_change|password_reset / unique `token_hash` / `consumed_at`), with the `idx_contact_verification_tokens_single_active` expression unique index (one unconsumed token per user per channel+purpose at a time, multi-DB compatible formulation). Repository `contact_verification_token_repo` provides `create`/`find_by_token_hash`/`find_latest_active_for_user`/`delete_active_for_user`/`mark_consumed_if_unused` (CAS against concurrent consumption)/`delete_expired`. `VerificationChannel` and `VerificationPurpose` are `DeriveActiveEnum`s.

- **Email change flow**: `users` gains a `pending_email` (unique index). `POST /auth/email/change` validates the current account has a verified email and the new email passes the local policy and is unused, then inside one transaction writes `pending_email` + issues a `ContactChange` token + delivers the confirmation email via outbox, auditing `UserRequestEmailChange`; `POST /auth/email/change/resend` is throttled and audits `UserResendEmailChange`; after the confirm endpoint verifies, `pending_email` is promoted to the official email and cleared, auditing `UserConfirmEmailChange`.

- **Forced password change (must_change_password)**: `users` gains `must_change_password` (standalone migration `m20260618_000002`). New module `auth_service/token_scope`: with `must_change_password=true` issued tokens carry a `password_change` claim and requests only pass the `(GET /auth/me, PUT /auth/password, POST /auth/logout)` allowlist, everything else returns `AuthPasswordChangeRequired` (error code `auth.password_change_required`); `auth_service` cross-checks the claim against the DB state in both directions. Frontend `/force-password-change` route + `AuthenticatedGate`/`GuestOnlyGate` guide the redirect when the flag is detected.

- **Admin user creation: generated password + forced change**: `POST /admin/users`'s `CreateAdminUserReq.password` becomes `Option<String>` (when absent the backend generates a temporary password and forces `must_change_password=true`), and username validation uses `validate_auth_username` (the same 4-16 char rule as regular registration). The response becomes `CreateAdminUserOutput { user, generated_password }`. Audit details add `temporary_password_generated`. The frontend `GeneratedPasswordDialog` shows the plaintext temporary password once (read-only + copy, with a not-shown-again notice). `UpdateAdminUserReq` also supports `must_change_password` and the unified 8-128 password bounds. Admin user write paths move from `auth_service` into the new `admin_user_service` module, protecting the `id=1` super admin from role/status changes.

- **User deletion (hard delete + cascade)**: `DELETE /admin/users/{id}` calls `admin_user_service::delete_user`, forbidding deletion of the `id=1` super admin. Cascade order: delete local avatar files + clear fields → per Minecraft profile call `delete_profile_for_user` (unbinding and purging unreferenced texture blobs, revoking Yggdrasil tokens selecting that profile) → delete all wardrobe textures (with blobs) → revoke all browser sessions → revoke all launcher tokens → delete the `users` row (remaining tables cascade via FK `on_delete`). The response `DeleteAdminUserOutput` carries per-stage cleanup counts. Audit `AdminDeleteUser`, marked `danger` tone in the frontend `audit.ts`.

- **Yggdrasil default skins**: `steve.png` / `alex.png` are embedded into the binary via `include_bytes!` (`texture_service/default_skin`); `for_profile_uuid` returns Alex (Slim) / Steve (Default) by the UUID's lowest-bit parity, falling back to Steve on parse failure. `texture_metadata_for_profile` automatically appends the default skin (with `source = Default`) when no skin binding exists, `properties.rs::texture_property_value` now unconditionally emits the textures property to keep protocol-level visibility, and `texture_by_hash` falls back to the embedded default skins on DB misses (200 + `image/png` + immutable ETag + 304 support). Default skin SHA-256 constants are test-guarded.

- **Minecraft Services compatibility layer**: new Bearer-authenticated, Mojang-style `{"path": "..."}` error body endpoints — `POST /minecraftservices/player/certificates` (generates a 2048-bit RSA profile key on the fly with `expiresAt = now+48h`, `refreshedAfter = now+36h`, dummy signature entries), `GET /minecraftservices/privileges` (`permissive_privileges` all on by default), `GET /minecraftservices/player/attributes`, `GET /minecraftservices/privacy/blocklist` (empty), `GET /sessionserver/blockedservers` (404); unmatched paths uniformly 404. New runtime config toggles `yggdrasil_enable_profile_key` (default true) and `yggdrasil_enable_mojang_anti_features` (default true).

- **Yggdrasil metadata extensions**: `YggdrasilMeta` gains `links: { homepage, register? }` (homepage derived from public_site_url; register present only when `allow_user_registration`), and `feature` gains `enable_profile_key`, `enable_mojang_anti_features`, `username_check` (fixed true).

- **Texture source field + ETag/conditional requests**: `MinecraftTextureMetadata` gains a `source` (`bound` / `default`) enum. The new `api/cache` module provides `weak_etag_for_*`, `request_etag_matches` (weak/strong ETags, `*`, lists) and `conditional_bytes_response` (automatic ETag, 304 on match). Frontend static assets, avatars, textures and default skins all adopt ETag + `If-None-Match` 304.

- **Texture dimension validation**: the backend `validate_texture_dimensions` validates per type — skin requires 64×32 or 64×64 uniform scale-up, cape requires 64×32 or 22×17 (legacy 22×17 capes are auto-padded to 64×32). The frontend gains `minecraftTextureValidation.ts` pre-upload validation (MIME/size/PNG signature/pixel count/dimensions, mirroring backend rules), and `PublicYggdrasilConfig` adds `max_texture_upload_bytes` and `max_texture_pixels` passed through for frontend pre-validation.

- **Profile / Wardrobe search filters**: `GET /profiles/minecraft` gains `query` (up to 64) with fuzzy name matching; `GET /wardrobe/textures` gains `keyword` (up to 96) and `texture_type`, with keyword doing hash/mimetype fuzzy + normalized type/model quick matching (typing `skin`/`slim` etc. triggers the corresponding column filter). Query parse errors go through `project_query_config` into `RequestMalformed`, avoiding leaking raw actix error bodies.

- **Skin avatar component**: new `MinecraftSkinAvatar` (crops the head 8×8 region from a skin texture URL using two 8x-scaled pixelated `<img>` overlays, falling back to a User icon on failure without duplicate retries), used on account profiles and admin user/profile detail pages; `MinecraftPreview` becomes `lazy` loaded (three/skinview3d split into a separate vendor chunk) and `MinecraftPreviewPanel` provides Suspense + Skeleton placeholders.

- **Public legal pages**: new `/tos`, `/privacy` routes and a generic `LegalPage` (TOC, Last updated badge, en/zh i18n `public.json.legal.*`); the Footer gains links.

- **Sidebar collapsing**: the desktop shell sidebar supports collapse/expand persisted to localStorage, with a new `AnimatedCollapsible` height-animation container used in the filter toolbar and sub-items.

- **Frontend infrastructure**: new `lib/storage.ts` (unified localStorage/sessionStorage JSON access + SSR guard), `lib/validation.ts` (username/email/password schema sets based on `zod/v4`), `lib/contactVerificationRedirect.ts` / `passwordResetRedirect.ts` (URL state banner parsing), `lib/user.ts` (`getUserDisplayName`), `DateTimeText` (an accessible `<time dateTime title>` wrapper replacing site-wide inline `formatDateTime`), `AuthFormPrimitives` (shared atoms for login/init/reset/forced-change/invitation forms), `GeneratedPasswordDialog`.

- **Yggdrasil API documentation**: new `developer-docs/{en,zh-CN}/yggdrasil-api.md` (a complete protocol implementation guide: route roots, metadata, authserver/sessionserver/texture APIs, the Minecraft Services compatibility layer, wire fields, error shapes, runtime config, OpenAPI, testing boundaries), and `docs/guide/profiles.md` now recommends the controlled rename API with a warning against direct database edits.

### Changed

- **Password length cap tightened (breaking)**: `validate_password` changes from 8-256 to 8-128 (frontend `passwordSchema` in sync); admin create/update user password validation uniformly uses `validate_optional_auth_password`.

- **Admin user creation response shape (breaking)**: the `POST /admin/users` response changes from `AdminUserInfo` to `CreateAdminUserOutput { user, generated_password }`; consumers must read `.user`.

- **Emails lowercased (breaking)**: `normalize_email` uniformly lowercases before persistence and lookup at registration, invitations, admin user creation, email change and other entry points.

- **Yggdrasil authentication requires a verified email (breaking)**: `yggdrasil_service/auth::authenticate` rejects accounts with an empty `email_verified_at` before password verification (non-activated accounts cannot log in via Yggdrasil), still returning `InvalidCredentials` externally to avoid leaking status.

- **Coarse external auth toggles removed (breaking)**: runtime config entries `external_auth_enabled`, `external_auth_auto_register` and the `external_auth` category constant (with i18n keys) are removed. External auth provider management itself (AdminExternalAuthPage, related routes and repos) remains usable; deployments depending on those two keys must migrate.

- **Cache policy unified**: the `avatar_image_response` helper is removed; `AVATAR_CACHE_CONTROL`/`AVATAR_CONTENT_TYPE`/`TEXTURE_CACHE_CONTROL` are promoted, and all avatar, static asset and texture routes go through `conditional_bytes_response`.

- **Public Yggdrasil config**: `PublicYggdrasilConfig` gains `max_texture_upload_bytes` and `max_texture_pixels`; the frontend `frontendConfigStore` exposes them in sync to drive pre-validation.

- **Local email policy error codes**: allowlist/blocklist rejections now use `validation_error_code` responses (`AuthEmailNotAllowlisted`/`AuthEmailBlocked`) so the frontend can show targeted messages.

- **Audit action extensions**: 9 new audit actions (`user_confirm_registration`, `user_request_email_change`, `user_resend_email_change`, `user_confirm_email_change`, `user_request_password_reset`, `user_confirm_password_reset`, `admin_create_invitation`, `admin_revoke_invitation`, `admin_delete_user`) and the `Invitation` entity type.

- **Registration endpoint response type**: the frontend `authService.register` changes from `AuthTokenResponse` to `RegisterResponse { expires_in, requires_activation }`; callers decide between showing the activation notice and entering the workbench based on `requires_activation`.

- **Properties building**: `build_profile_properties` no longer checks `textures.is_empty()` and unconditionally builds the textures property to support default skin injection.

- **Admin settings / about pages**: the settings page drops the category description cards and per-group count badges; the about page links to the external docs site and adjusts brand mark size and resource link icons.

### Fixed

- **Error response consistency**: list query parse errors uniformly go through `project_query_config` into `RequestMalformed`; local email policy rejections use error-coded responses, avoiding leaking raw actix error bodies.

- **Accessibility**: site-wide timestamps wrapped in `<time dateTime title>` (`DateTimeText`) so screen readers and browsers get machine-readable time, with visible text still formatted per the i18n locale.

- **Route scroll behavior**: `AppLayout` auto-scrolls to top on route change (without hash), preserving position for hash-anchored links.

- **Auth chain logging**: `auth::check`/`auth::me` gain `tracing::debug!` for token chain troubleshooting.

### Removed

- Removed config entries `external_auth_enabled`, `external_auth_auto_register` and the `external_auth` category (with i18n keys).
- Removed the `avatar_image_response` inline helper and site-wide inline `formatDateTime` calls (unified onto `DateTimeText`).

---

**Statistics**:
- 258 files changed, 29,992 insertions(+), 5,818 deletions(-)
- 4 commits (4 features)
- 2 new database migrations (`m20260618_000001`, `m20260618_000002`), new tables `contact_verification_tokens`, `user_invitations`, new fields `users.pending_email`, `users.must_change_password`

## [v0.1.0-alpha.3] - 2026-06-16

### Added

- **Unified list endpoint pagination**: profiles, wardrobe, sessions, passkeys, external auth link/provider and other list endpoints move from bare arrays to offset-paginated responses. Affected endpoints return `OffsetPage<T>` (with `items` / `limit` / `offset` / `total`): `GET /profiles/minecraft`, `GET /wardrobe/textures`, `GET /auth/external-auth/providers`, `GET /auth/external-auth/links`, `GET /admin/users/{id}/minecraft-profiles`; the auth_session / minecraft_texture / external_auth_identity / passkey repositories gain `list_by_user_paginated` with matching service-layer pagination wrappers.

- **Account area APIs and workbench**: the `/account` route group lands. `GET /account/overview` returns the current user's profile count and the 5 most recent activities (audit logs); `GET /account/audit-logs` provides paginated + filtered (action / entity_type / entity_id / time range) + sorted audit log queries for the current user, with the user_id boundary enforced server-side and not overridable by the frontend. Frontend counterparts: `accountService`, the `AccountOverviewPage` workbench and `AccountAuditPage` (`/account/audit`), with en/zh i18n.

- **Frontend navigation and route architecture refactor**: unified shell — `AccountShell` and `AdminShell` merge into a single parameterized `AppShell`; route guards become standalone components `AuthenticatedGate` / `GuestOnlyGate` / `AdminOnlyGate` / `InitGate`; `RouteAccessState` replaces silent redirects, showing contextual notices instead of silent redirects on denial; all paths centralize into `routePaths.ts` with dynamic segment helpers. Navigation becomes collapsible `ShellNavSection` structures by account / admin partitions. Theme switching becomes a dual-icon crossfade animation and adds `data-theme-surface` (AsterDrive-style soft transitions); the password strength meter adopts red / amber / emerald tiers; the topbar search is removed and large chunks get a PWA precache exclusion list.

### Changed

- **Route and naming normalization (breaking)**: `/dashboard` → `/account`, `/dashboard/admin/*` → `/admin/*`; legacy `/auth`, `/dashboard/launcher` and legacy `/app/*` routes are removed with no automatic redirects kept. Page names unified: `ProfilesPage` → `MinecraftProfilesPage`, `WardrobePage` → `TextureWardrobePage`, etc.; common components migrate from the `panel/` namespace to `common/`; i18n splits `dashboard.json` out into `account.json` plus new route status strings.

- **List response format (breaking)**: the affected list endpoints change from bare arrays to `{ items, limit, offset, total }` pagination objects; third-party / custom clients must adapt.

- **Audit sort type sharing**: `AdminAuditLogSortBy` renames to `AuditLogSortBy`, with sort query handling moved into `audit_service::filters`, shared between user and admin queries.

### Removed

- Removed the developer-facing legacy concept pages: the Yggdrasil info page, diagnostics panel (`ServiceDiagnosticsPanel` / `useServiceDiagnostics` / `diagnosticsService`), API catalog (`EndpointCard` / `SectionTitle` / `AccessBadge`), forge onboarding (`FoundationSummary` / `ModuleRail`), `ExternalAuthPage`, `AuthPage`, `AdminConfigPage`, `AdminYggdrasilPage`, `WorkbenchPage`, and `legacyRedirects`.

---

**Statistics**:
- 148 files changed, 6,840 insertions(+), 4,050 deletions(-)
- 2 commits (2 features)
- No new database migrations

## [v0.1.0-alpha.2] - 2026-06-16

### Added

- **Profile rename and token temporary invalidation**: both users and administrators can rename Minecraft profiles (user endpoint `PUT /api/v1/profiles/minecraft/{uuid}/name`, admin endpoint `PUT /api/v1/admin/minecraft-profiles/{uuid}/name`). On rename, Yggdrasil tokens bound to the profile enter a temporarily-invalidated state — blocking `validate` / `join` but allowing `refresh` (clearing invalidation and issuing a new token with the new name), forcing launchers to pull updated profile metadata before play. Name uniqueness validation (rejecting duplicates, same-name treated as no-op); new migration `m20260616_000001` adds `temporarily_invalidated_at` to `yggdrasil_tokens`, recording a `MinecraftProfileRename` audit event (old name / new name / invalidated token count); rename dialogs added on the user profile page and the admin profile detail page with en/zh i18n and audit display.

- **Yggdrasil protocol rate limiting**: dedicated token-bucket rate limits for `authserver/authenticate` and `authserver/signout` (anti credential stuffing). Based on the governor keyed limiter, bucketed by normalized username (trim + lowercase), with independent buckets for `authenticate` and `signout`; on trigger it returns `429` + `Retry-After` with an error body following the authlib-injector protocol format (`TooManyRequestsException`). Reuses the `rate_limit.auth` tier config.

### Fixed

- **Yggdrasil protocol**: the `uploadableTextures` profile property previously hardcoded `signature: None`, violating the authlib-injector spec. Now, in the `unsigned=false` and sessionserver `hasJoined` server-side verification scenarios, the property is signed with the RSA private key just like the textures property, letting servers fully verify all profile properties.

### Changed

- **Rate limiting enabled by default**: `RateLimitConfig.enabled`'s default changes from `false` to `true`, with `config.example.toml` in sync. New deployments enable rate limiting for authentication-class requests by default; existing deployments' explicit `config.toml` values take precedence and are unaffected.

- **Docs**: corrected the `clientToken` behavior description in `invalidate` — per the Yggdrasil spec, invalidate validates only `accessToken` and `clientToken` plays no part in revocation (endpoint behavior unchanged, only the previously misleading documentation corrected, en/zh in sync).
- **Docs**: the signature wording for `hasJoined` and `profile/{uuid}` generalized from "textures property signature" to "profile properties signature", aligned with the new signing behavior (en/zh in sync).

---

**Statistics**:
- 64 files changed, 2,108 insertions(+), 50 deletions(-)
- 4 commits (3 features + 1 CI)
- 1 new database migration (`m20260616_000001`)

## [v0.1.0-alpha.1] - 2026-06-16

### Release Highlights

**The first public release of AsterYggdrasil!** A self-hosted Minecraft skin station and Yggdrasil authentication server, distributed as a single binary, written in Rust, MIT licensed, fully authlib-injector compatible.

- **Complete Yggdrasil protocol** — authserver (authenticate / refresh / validate / invalidate / signout) + sessionserver (join / hasJoined / profile), RSA 4096-signed texture properties
- **Skin station and wardrobe** — skin / cape upload (1.8 + legacy dimension validation, PNG sanitization), SHA-256 deduplication, avatar WebP generation, multi-slot profile texture management
- **Multiple login methods** — password (argon2) + Passkey / WebAuthn + third-party (OIDC / OAuth2 / GitHub / Google / Microsoft / QQ)
- **Multi-database support** — SQLite / MySQL / PostgreSQL, SeaORM migrations with unified time columns
- **Enterprise-grade infrastructure** — runtime hot config, Redis / in-memory cache, Prometheus metrics, audit logs, background task engine, SMTP outbox mail
- **Frontend panel** — React 19 + Vite 8 + Tailwind 4, skinview3d 3D preview, rust-embed embedded into the single binary

### Added

- **Yggdrasil authentication protocol**
  - Full authlib-injector protocol implementation: authserver (authenticate / refresh / validate / invalidate / signout) and sessionserver (minecraft/join, minecraft/hasJoined, minecraft/profile/{uuid}, api/profiles/minecraft)
  - Access tokens stored as SHA-256 hashes with client token comparison, per-user active token cap trimming, expiry / revocation cleanup
  - join flow uses the serverId SHA-256 as cache key (shared cache, TTL 30s); hasJoined optionally validates the client IP against proxy traversal
  - RSA 4096 + SHA-1 PKCS#1 v1.5 signing of the textures property, the private key auto-generated and persisted on first start, graceful degradation without one
  - `/api/yggdrasil` metadata endpoint returns signaturePublickey / skinDomains / serverName / feature.nonEmailLogin
  - Texture properties base64-JSON serialized per the authlib-injector spec (timestamp / model included), with an independent YggdrasilError error mapping layer that does not pollute the global envelope

- **Account authentication and sessions**
  - `/auth/setup` first-admin bootstrap, `/auth/register` regular registration (disableable at runtime), the first registered user automatically becomes Admin
  - Login accepts email or username identifiers, argon2 password verification, and the super admin (`id=1`) role / status cannot be downgraded
  - JWT dual tokens (HS256): access 15 minutes and refresh 7 days by default, jti rotation on the auth_sessions table with replay detection
  - session_version global session invalidation: password change / user disable / revoke_all immediately voids all existing tokens
  - Session list, single-session revocation, revoke-all-except-current, expired session background cleanup, all audited

- **Passkey / WebAuthn passwordless login**
  - Based on webauthn-rs, enforcing Discoverable Credential (resident_key=Required) + user_verification=Required
  - Two-phase registration / login (challenge cached with TTL 300s), with conditional UI (autofill) re-verification
  - Relying Party derived from public_site_url as rp_id / rp_name, multi-origin allowlist, HTTP allowed on localhost
  - Users self-list / rename / delete passkeys; successful login issues the same JWT dual tokens as password login

- **Third-party login / external authentication**
  - Six providers: OIDC, generic OAuth2, GitHub, Google, Microsoft, QQ, with a unified trait + registry dispatch
  - OIDC via openidconnect 4.0 with issuer discovery, PKCE, nonce validation, ID Token claims
  - Login flow (state / nonce / pkce persisted, TTL 5 minutes) supports auto-provisioning users and verified-email auto-binding
  - Account binding / unbinding (constant-time argon2 + dummy hash against enumeration), email domain allowlist
  - Provider callbacks without a verified email trigger the local email supplemental verification flow (TTL 30 minutes, idempotent confirm)

- **Mail system**
  - lettre 0.11 SMTP (implicit TLS 465 / STARTTLS / plaintext), outbox mode writing only the table inside transactions with background batched claim delivery
  - Stepped backoff retries (5 / 15 / 60 / 300 / 900 / 1800s, up to 6 attempts), stale rows safely claimed by multiple workers
  - Seven built-in templates (registration activation / email change confirmation / password reset / reset notice / change notice / email verification / login verification code), subject / html / text three-part
  - Templates stored in system_config, editable at runtime; delivery success / failure both audited; admins can send test emails

- **Textures / skins / profiles / wardrobe**
  - skin + cape upload supporting default / slim models, multipart streaming to disk with realtime oversize rejection
  - Decoded via the image crate and rewritten as clean RGBA8 PNG (against malicious metadata / APNG), with PNG bomb pixel pre-checks
  - 1.8 (64x64) + legacy (64x32) skin dimension validation; legacy 22x17 capes auto-padded to 64x32
  - SHA-256 content fingerprint deduplication with reference-counted deletion (blobs deleted only at zero references); wardrobe textures can bind multiple profiles for reuse
  - Avatars support none / Gravatar / local upload, center-cropped square 512 / 1024 WebP generation, versioned + immutable caching
  - Maintenance tools: legacy textures to wardrobe, duplicate fingerprint merging, storage consistency inspection (missing / mismatched / orphan blobs)

- **Storage backends**
  - TextureStorage async trait abstraction (put / get_stream / delete / exists / metadata / list_keys)
  - LocalTextureStorage filesystem implementation with storage key path sanitization against directory traversal
  - S3 / minio config structs reserved (explicitly rejected at initialization), interface contracts and test skeletons in place

- **Database and migrations**
  - 4 migrations organized by sea-orm-migration (foundation / Yggdrasil profiles / textures / passkeys)
  - Full modeling of 18 tables, all with foreign key cascades and unique indexes
  - Multi-DB compatibility: time.rs unified time columns (MySQL datetime(6) / PG+SQLite timestamptz), migrations write no backend-specific SQL
  - Yggdrasil domain strong constraints: profile UUID / name unique, tokens stored hashed, ON DELETE SET NULL / CASCADE

- **Background task engine**
  - Persistent background_tasks table (kind / status / payload / result / steps / progress / attempt / lease)
  - Lease-based concurrency: processing_token CAS claim + heartbeat renewal + lane capacity re-check against oversell
  - Three retry tiers (Auto / Manual / Never), auto retries at 5 / 15 / 60 / 300s increments, graceful shutdown releases back to Retry
  - Step-level progress (TaskStepInfo), cleanup_expired purges terminal-task temp directories

- **Audit logs**
  - Fully asynchronous fire-and-forget: 4096 buffer + 100 batch flush, degraded to direct writes when full, never blocking business
  - 47 AuditActions × 14 EntityTypes across nine groups (login / config / user / admin / task / mail / external auth / Minecraft / Yggdrasil)
  - build_audit_presentation generates structured summary / target / detail messages for frontend rendering
  - Runtime-controlled recorded_actions + retention_days (default 90) auto-cleanup, admin-side multi-dimensional filtered pagination

- **Config system**
  - Static toml (data/config.toml, auto-generated on first start) + ASTER\_\_SECTION\_\_KEY environment variable overrides with automatic type inference
  - 80+ runtime configs persisted to system_config, ConfigDef centrally defining type / default / requires_restart / is_sensitive, hot-applied via a parking_lot::RwLock
  - normalize_system_value performs trim / type / range / cross-field validation (e.g. credentials=true forbids origins=*)
  - Path sandboxing (rejects beyond base_dir / SQLite URLs), deprecated key rejection, Primary / Follower node modes

- **Cache**
  - Unified CacheBackend trait + three backends: memory (moka 64MiB + weigher + TTL) / redis (ConnectionManager + 250ms timeout) / Noop
  - set_bytes_if_absent atomic placement (ReservationSet + dashmap), once-only claim semantics
  - Redis failure circuit breaking: skip Redis for memory during a 5s cooldown, with a single transition log on recovery

- **Metrics**
  - Prometheus under the metrics feature: HTTP requests / latency, DB queries, auth events, background tasks, pending counts and other metric families
  - sysinfo periodic RSS / CPU / uptime sampling (primary node)
  - MetricsMiddleware records per match_pattern route labels, short-circuiting when disabled

- **Runtime and logging**
  - tracing structured logging (text / json), level / file output / daily rotation / max_backups configurable
  - Graceful shutdown (SIGINT / SIGTERM): background tasks (30s grace) → audit flush → DB close
  - Panic hook dual-channel recording, background task catch_unwind converting to failed results
  - primary starts 9+ periodic tasks (mail / token / audit / task / texture cleanup, etc.), exponential backoff + wakeup event + jitter
  - Optional jemalloc global allocator (with stats / profiling), TrackingAlloc heap-peak statistics by default

- **HTTP middleware**
  - Runtime CORS (policy built live from RuntimeConfig, Vary maintained)
  - actix-governor four rate-limit tiers (auth / public / api / write), trusted_proxies CIDR parsing XFF for keys, 429 with Retry-After
  - Security response headers (X-Frame-Options / Referrer-Policy / X-Content-Type-Options)
  - Request-ID (UUID v4 + X-Request-ID response header + info_span log correlation)
  - CSRF double submit (cookie vs X-CSRF-Token) + Origin / Referer / Sec-Fetch-Site source validation
  - JWT cookie first + Bearer fallback, JwtAuth / RequireAdmin middleware

- **Error handling**
  - Unified AsterError with stable AsterErrorCodes (80+ snake_case codes like auth.credentials_failed / config.validation_failed)
  - Yggdrasil protocol endpoints use an independent error mapping that does not pollute the global envelope

- **Admin API**
  - User management (filter / sort / paginate / CRUD / avatar / one-click revoke all sessions)
  - Minecraft profile management (view / delete / batch delete textures by hash)
  - System config management (list / write / Schema / template variable metadata / actions like sending test emails)
  - Audit log queries, background task management (retry / cleanup), external auth provider CRUD + connection testing
  - System info (version / build time), Admin scope enforcing JWT + admin + trusted-proxy rate limiting

- **Frontend panel**
  - React 19 + TypeScript (tsgo native preview) + Vite 8 + Tailwind CSS 4 + @base-ui/react (shadcn)
  - Zustand 5 state, react-router-dom 7 routing, axios HTTP, vite-plugin-pwa offline
  - skinview3d 3D skin preview, react-image-crop avatar cropping, i18next en/zh bilingual
  - Pages: login / init / external auth callback / personal settings / profiles / wardrobe / workbench; admin users / profiles / config / audit / tasks / external auth / Yggdrasil info / about
  - Vitest unit tests + Playwright E2E full flows, openapi-typescript reverse-generated API types

- **API documentation**
  - utoipa 5 + utoipa-swagger-ui 9, Swagger UI and OpenAPI JSON mounted under debug + openapi features
  - api-docs-macros per-route path annotations, static openapi.json generation + frontend type reverse-generation

- **Test suite**
  - Rust integration tests ~200+ cases / ~15000 lines covering authentication (incl. passkeys) / OAuth2 / OIDC / Yggdrasil (47 cases) / CORS / cache / config / audit / admin tasks / multi-database backends
  - Frontend unit tests (stores / pages / i18n / CSS) + Playwright E2E full flows

- **Deployment and build**
  - Multi-stage Dockerfile: Node 24 + Bun building the frontend, Rust 1 Alpine building the backend (precompiled dependency cache layer + strip)
  - Final image on alpine:3.23 with built-in SQLite, non-root (UID 10001), built-in HEALTHCHECK
  - docker-compose out of the box (mounting ./data), image ghcr.io/astercommunity/asteryggdrasil
  - Frontend rust-embed embedded into the binary with ./frontend-override runtime override, CSP / PWA / SPA fallback / tiered caching
  - Bilingual docs site (quick start / config / storage / profiles / Yggdrasil API / textures / launcher login / audit tasks / Docker)

### Dependencies

- **Web**: actix-web 4.13, actix-governor 0.10, actix-multipart 0.7
- **ORM**: sea-orm 2.0.0-rc.40 (SQLite / MySQL / PostgreSQL) + sea-orm-migration
- **Auth**: jsonwebtoken 10, argon2 0.5, webauthn-rs 0.6.1-dev, openidconnect 4.0
- **Mail**: lettre 0.11 (rustls)
- **Crypto / signing**: rsa 0.10, sha1 0.11, sha2 0.11, md-5 0.11
- **Cache**: moka 0.12, redis 1.2
- **Image**: image 0.25 (jpeg / png / gif / webp)
- **API docs**: utoipa 5.5, utoipa-swagger-ui 9.0
- **Runtime**: tokio 1, tracing 0.1, tracing-subscriber 0.3
- **Allocator**: tikv-jemallocator 0.7 (optional)
- **Frontend**: React 19, Vite 8, Tailwind CSS 4, @base-ui/react, Zustand 5, react-router-dom 7, skinview3d 3.4, axios, i18next 26

---

**Statistics**:
- 596 files changed, 130,592 insertions(+)
- 2 commits
- Rust ~55,500 lines (src + migration + macros), tests ~15,300 lines
- 189 frontend TS/TSX files
- Rust Edition 2024, MSRV 1.94.0

[Unreleased]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-beta.2...HEAD
[v0.1.0-beta.2]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-beta.1...v0.1.0-beta.2
[v0.1.0-beta.1]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.6...v0.1.0-beta.1
[v0.1.0-alpha.6]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.5...v0.1.0-alpha.6
[v0.1.0-alpha.5]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.4...v0.1.0-alpha.5
[v0.1.0-alpha.4]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.3...v0.1.0-alpha.4
[v0.1.0-alpha.3]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.2...v0.1.0-alpha.3
[v0.1.0-alpha.2]: https://github.com/AsterCommunity/AsterYggdrasil/compare/v0.1.0-alpha.1...v0.1.0-alpha.2
[v0.1.0-alpha.1]: https://github.com/AsterCommunity/AsterYggdrasil/releases/tag/v0.1.0-alpha.1
