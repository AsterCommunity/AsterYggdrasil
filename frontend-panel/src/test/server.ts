import { setupServer } from "msw/node";

/** Path matcher for project API routes that tolerates any configured origin. */
export function httpPath(path: string) {
	return `*${path}`;
}

/**
 * Shared MSW server for network-boundary tests.
 *
 * Start it in a test file that exercises real HTTP traffic (services hitting
 * axios instead of `vi.mock`-ed modules):
 *
 * ```ts
 * import { server } from "@/test/server";
 * import { http, HttpResponse } from "msw";
 *
 * beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
 * afterEach(() => server.resetHandlers());
 * afterAll(() => server.close());
 *
 * // per-test handler overrides (match any origin with a wildcard)
 * server.use(
 *   http.get(httpPath("/api/v1/admin/users"), () =>
 *     HttpResponse.json(apiResponse(createAdminUserPage())),
 *   ),
 * );
 * ```
 *
 * `onUnhandledFrame: "error"` (set globally in setup.ts) keeps unmocked
 * endpoints from silently hitting the network inside jsdom. See
 * src/test/README.md for the vi.mock-vs-MSW boundary rules.
 */
export const server = setupServer();
