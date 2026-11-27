# Frontend e2e suite

Playwright suite for the embedded panel: core user flows in `e2e/*.spec.ts`
and the Yggdrasil/authlib-injector protocol suite in `e2e/protocol/`.

## Layout

```text
e2e/
├── support/               Shared helpers (API seeding, page operations, fixtures)
│   ├── api.ts             API session + data seeding (login, users, profiles, textures)
│   ├── api-response.ts    Aster envelope assertions
│   ├── auth.ts            Page-level login/register/logout/change-password flows
│   ├── fixtures.ts        Credentials (env-overridable), unique names, PNG factory
│   ├── global-setup.spec.ts  Setup project: initializes instance + storageStates
│   └── ...
├── protocol/              Protocol suite (own playwright.protocol.config.ts)
│   ├── support/seed.ts    Instance/player/profile seeding over HTTP
│   ├── authserver.spec.ts   authenticate/refresh/validate/invalidate/signout
│   ├── metadata.spec.ts     authlib-injector metadata document
│   ├── profile.spec.ts      profiles/minecraft + sessionserver lookup/join
│   └── textures.spec.ts     texture upload/fetch/delete + error shapes
├── auth.spec.ts           Login/register/logout/forced change/session expiry
├── account.spec.ts        Profiles, wardrobe upload/equip, launcher card
├── admin.spec.ts          Users CRUD + scopes, texture review, settings round-trip
├── guards.spec.ts         Admin gate, guest redirect, init gate
└── smoke.spec.ts          Original smoke checks (credentials moved to fixtures)
```

## Running

The Rust backend is started by Playwright's `webServer` through
`scripts/run-e2e-server.sh` (fresh temp runtime + SQLite per run, port 3310).
Rust compilation is the slow part; the build is cached in `target/e2e`:

```bash
# One-time backend build into the e2e target dir (reuses cargo cache afterwards):
CARGO_TARGET_DIR=target/e2e cargo build --no-default-features --features server

# Skip the per-run cargo step by pointing the script at the built binary:
ASTER_E2E_SKIP_BUILD=1 \
ASTER_E2E_SERVER_BIN="$PWD/target/e2e/debug/aster_yggdrasil" \
  bunx playwright test

# UI suite only (implicit: also builds the frontend first):
bun run test:e2e

# Protocol suite (own config, own port/runtime; do not run both at once):
ASTER_E2E_SKIP_BUILD=1 \
ASTER_E2E_SERVER_BIN="$PWD/target/e2e/debug/aster_yggdrasil" \
  bunx playwright test --config playwright.protocol.config.ts

# Type-check the suite itself (tsconfig.app.json only covers `src/`):
bunx tsc -p tsconfig.e2e-check.json
```

The suite is parallel: the `setup` project runs first, initializes the
instance, seeds the admin/player/operator accounts, and persists their
`storageState` files under `test-results/.auth/`; the `chromium` project then
runs all specs in parallel with those sessions. Rate limits are lifted via
`ASTER__RATE_LIMIT__*` env overrides so parallel workers are not throttled.

## Credential environment variables

Specs never contain hardcoded passwords. Accounts only exist inside the
throwaway e2e runtime database; values below fall back to dev-only defaults,
and CI can override any of them:

| Variable                      | Account        | Purpose                                     |
| ----------------------------- | -------------- | ------------------------------------------- |
| `ASTER_E2E_ADMIN_PASSWORD`    | `e2e_admin`    | First admin created by the setup project    |
| `ASTER_E2E_PLAYER_PASSWORD`   | `e2e_player`   | Plain user for account/guard/protocol flows |
| `ASTER_E2E_OPERATOR_PASSWORD` | `e2e_operator` | Operator scoped to `admin.users`            |

Username/email are fixed (`e2e_admin`, `e2e_player`, `e2e_operator` at
`@example.test`) because the setup project re-creates them once per run.
Ports are overridable with `ASTER_E2E_PORT` (default 3310) and
`ASTER_E2E_PROTOCOL_PORT` (default 3311).
