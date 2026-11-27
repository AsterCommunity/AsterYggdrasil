import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";

/**
 * Standalone Playwright project for the Yggdrasil/authlib-injector protocol
 * suite (`e2e/protocol/**`). It exercises the wire contract — authserver
 * token lifecycle, sessionserver profile/session lookups, and texture
 * endpoints — directly over HTTP request contexts, so no browser or UI setup
 * is needed and the suite can run without building a full UI session.
 *
 * Dependencies and startup:
 * - requires the Rust backend binary (see `scripts/run-e2e-server.sh`);
 *   `cargo build` output is cached in `target/e2e`, so set
 *   `ASTER_E2E_SKIP_BUILD=1` (plus `ASTER_E2E_SERVER_BIN` pointing at the
 *   built `aster_yggdrasil` executable) to skip rebuilds;
 * - the frontend only needs to be built when protocol specs also hit the
 *   embedded panel (they do not), so `bun run build` can be skipped here;
 * - runs on its own port (3311) and its own runtime directory/database, so it
 *   never races the UI suite in `playwright.config.ts`. Run the two configs
 *   one at a time:
 *       bunx playwright test --config playwright.protocol.config.ts
 *
 * Protocol endpoints keep their authlib-injector wire format (no project
 * envelope), so specs assert raw JSON shapes, statuses, and error bodies.
 * Workers stay at 1: the seed step must win the one-time instance setup race
 * on a fresh runtime database, and protocol cases are cheap HTTP exchanges.
 */
const port = Number(process.env.ASTER_E2E_PROTOCOL_PORT ?? "3311");
const baseURL =
	process.env.PLAYWRIGHT_PROTOCOL_BASE_URL ?? `http://localhost:${port}`;
const configDir = path.dirname(fileURLToPath(import.meta.url));
const webServerCommand = "./scripts/run-e2e-server.sh";

// Same rationale as the UI config: parallel-free but burst-heavy protocol
// exchanges must not trip the per-username authenticate/signout limiter.
const e2eRateLimitEnv = {
	ASTER__RATE_LIMIT__AUTH__SECONDS_PER_REQUEST: "1",
	ASTER__RATE_LIMIT__AUTH__BURST_SIZE: "1000",
	ASTER__RATE_LIMIT__PUBLIC__SECONDS_PER_REQUEST: "1",
	ASTER__RATE_LIMIT__PUBLIC__BURST_SIZE: "2000",
	ASTER__RATE_LIMIT__API__SECONDS_PER_REQUEST: "1",
	ASTER__RATE_LIMIT__API__BURST_SIZE: "5000",
	ASTER__RATE_LIMIT__WRITE__SECONDS_PER_REQUEST: "1",
	ASTER__RATE_LIMIT__WRITE__BURST_SIZE: "1000",
};

export default defineConfig({
	testDir: "./e2e/protocol",
	fullyParallel: true,
	workers: 1,
	retries: process.env.CI ? 1 : 0,
	timeout: 60_000,
	expect: {
		timeout: 10_000,
	},
	reporter: [
		["list"],
		["html", { open: "never", outputFolder: "playwright-report/protocol" }],
	],
	use: {
		baseURL,
		locale: "en-US",
		trace: "retain-on-failure",
	},
	webServer: {
		command: webServerCommand,
		cwd: configDir,
		reuseExistingServer: false,
		stderr: "pipe",
		stdout: "pipe",
		timeout: 10 * 60 * 1000,
		url: `${baseURL}/health/ready`,
		env: {
			...e2eRateLimitEnv,
			ASTER_E2E_PORT: String(port),
		},
	},
});
