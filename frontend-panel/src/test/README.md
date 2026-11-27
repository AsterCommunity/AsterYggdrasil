# Test support

Shared helpers for Vitest unit tests. Everything here is test-only code and is
excluded from coverage.

## Files

- `setup.ts` — global vitest setup: jest-dom matchers, `matchMedia` /
  `ResizeObserver` / `IntersectionObserver` mocks, storage cleanup, and the
  global MSW server lifecycle.
- `fixtures.ts` — typed factories derived from `@/types/api` (`createAdminUser`,
  `createAuthSession`, `createTask`, `createAuditEntry`, cursor-page helpers,
  `apiResponse` envelope factory, …). The envelope mirrors what
  `services/http.ts` unwraps: `{ code: "success", msg, data }`.
- `i18n.ts` — `createMockTWithInterpolation(translations)` for component tests
  that mock `react-i18next` and assert rendered copy instead of bare keys.
- `deferred.ts` — manually resolved promises for stale-response / loading-state
  assertions without timers.
- `authState.ts` — `setAuthenticatedUser(user, { isAdmin })` seeds the auth
  store for tests that render guarded components directly.
- `permissions.ts` — the permission registry fixture used by operator-scope
  tests.
- `server.ts` — shared MSW `setupServer` instance plus `httpPath()` origin
  wildcard helper.

## Mock boundaries: `vi.mock` vs MSW

Two kinds of seams, two tools. Pick by what the test is about:

- **`vi.mock` (component boundary)** — the default in this suite. Unit tests
  render components and mock the service module
  (`vi.mock("@/services/adminService", …)`). The test asserts how the component
  reacts to _resolved values_, not how HTTP behaves. This keeps rendering tests
  fast and independent of transport details.
- **MSW (network boundary)** — for tests that exercise real HTTP traffic:
  service modules against the axios layer, envelope unwrapping, CSRF headers,
  token-refresh interception, and error-shape mapping. Start the server in the
  test file and add per-test handlers:

  ```ts
  import { http, HttpResponse } from "msw";
  import { apiResponse, createAdminUserPage } from "@/test/fixtures";
  import { httpPath, server } from "@/test/server";

  beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
  afterEach(() => server.resetHandlers());
  afterAll(() => server.close());

  server.use(
    http.get(httpPath("/api/v1/admin/users"), () =>
      HttpResponse.json(apiResponse(createAdminUserPage())),
    ),
  );
  ```

Do not mix the two for the same seam: if the component test already mocks the
service module, adding an MSW handler changes nothing, and if a service test
needs MSW, `vi.mock`-ing the service itself would defeat the test.

## MSW decision

MSW is enabled **globally** in `setup.ts` with
`server.listen({ onUnhandledFrame: "error" })` (msw 3.x renamed
`onUnhandledRequest` to `onUnhandledFrame`). Consequences:

- Any request that escapes `vi.mock` service boundaries and has no handler
  fails loudly instead of silently hitting the network (or erroring deep inside
  axios with a confusing `network_error`).
- Adopting it did not require handlers for existing tests: the whole existing
  suite (75 files) passes unchanged because every existing test mocks the
  service module boundary and never sends real requests.
- New network-boundary tests opt in per file as shown above; global `error`
  mode guarantees an unhandled endpoint is a test failure, not a leak.

## Conventions

- Entity fixtures always come from `fixtures.ts`; do not hand-write inline
  literals for types that already have a factory. If a test needs a new
  entity, add a factory here instead of a local literal.
- Assertions on translated UI copy use either the real i18n pipeline
  (`import "@/i18n"`, English or `zh-CN` via `i18next.changeLanguage`) or the
  mock `t` from `i18n.ts` with the en-US locale strings as the translation
  map. Do not assert bare `snake_case` i18n keys unless the test's subject is
  the key itself (for example key-passing regression tests that mock `t` to
  return keys on purpose).
