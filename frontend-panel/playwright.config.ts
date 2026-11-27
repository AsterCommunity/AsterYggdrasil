import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "@playwright/test";
import { STORAGE_STATE_FILES } from "./e2e/support/fixtures";

const port = Number(process.env.ASTER_E2E_PORT ?? "3310");
const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? `http://localhost:${port}`;
const configDir = path.dirname(fileURLToPath(import.meta.url));
const webServerCommand =
	process.env.ASTER_E2E_SKIP_BUILD === "1"
		? "./scripts/run-e2e-server.sh"
		: "bun run build && ./scripts/run-e2e-server.sh";

// The embedded backend applies token-bucket rate limits per client/username.
// Defaults (2s/request, burst 5 on auth) starve parallel workers, so the e2e
// runtime lifts the bursts while keeping the tiers enabled. `try_parsing`
// accepts these as numbers, and NonZero fields reject zero.
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
	testDir: "./e2e",
	testIgnore: ["**/protocol/**"],
	fullyParallel: true,
	workers: process.env.CI ? 2 : 4,
	retries: process.env.CI ? 1 : 0,
	timeout: 90_000,
	expect: {
		timeout: 15_000,
	},
	reporter: [
		["list"],
		["html", { open: "never", outputFolder: "playwright-report" }],
	],
	use: {
		baseURL,
		acceptDownloads: true,
		locale: "en-US",
		serviceWorkers: "block",
		screenshot: "only-on-failure",
		trace: "retain-on-failure",
		video: "retain-on-failure",
		viewport: { width: 1440, height: 960 },
	},
	projects: [
		{
			name: "setup",
			testMatch: /support\/global-setup\.spec\.ts$/,
		},
		{
			name: "chromium",
			dependencies: ["setup"],
			testIgnore: [/support\//, /protocol\//],
			use: {
				storageState: STORAGE_STATE_FILES.admin,
			},
		},
	],
	webServer: {
		command: webServerCommand,
		cwd: configDir,
		reuseExistingServer: false,
		stderr: "pipe",
		stdout: "pipe",
		timeout: 10 * 60 * 1000,
		url: `${baseURL}/health/ready`,
		env: e2eRateLimitEnv,
	},
});
