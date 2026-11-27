import { expect, test } from "@playwright/test";
import {
	ACCESS_COOKIE_NAME,
	adminApiSession,
	apiSession,
	createUserViaApi,
	findUserByUsername,
	loginViaApi,
	REFRESH_COOKIE_NAME,
	revokeUserSessionsViaApi,
} from "./support/api";
import {
	changeForcedPasswordViaUi,
	loginViaUi,
	loginViaUiExpectingPasswordChange,
	logoutViaUi,
	registerViaUi,
} from "./support/auth";
import { ADMIN, e2eBaseURL, PLAYER, uniqueUsername } from "./support/fixtures";

test.describe("auth flows", () => {
	// These specs drive fresh browser contexts through the real login/logout
	// pages; the seeded admin storageState is not useful for them. The same
	// applies to the `request` fixture, so admin API calls open their own
	// session via `adminApiSession`.
	test.use({ storageState: { cookies: [], origins: [] } });

	test("registers a new account and lands in the workbench", async ({
		page,
	}) => {
		const username = uniqueUsername("reg");
		const password = `Rg8-${uniqueUsername("pw")}!x`;
		await registerViaUi(page, username, `${username}@example.test`, password);

		await expect(page).toHaveURL(/\/account$/);
		await expect(
			page.getByRole("heading", { name: new RegExp(username) }),
		).toBeVisible();
	});

	test("logs in with the seeded player account and logs back out", async ({
		page,
	}) => {
		await loginViaUi(page, PLAYER.username, PLAYER.password);
		await expect(
			page.getByRole("button", { name: PLAYER.username }),
		).toBeVisible();

		await logoutViaUi(page, PLAYER.username);
		// Regression for issue #37: reload must not resurrect the session.
		await page.reload();
		await expect(
			page.getByRole("link", { name: "Login / Register" }),
		).toBeVisible();

		await page.goto("/account");
		await expect(
			page.getByRole("heading", { name: "Login required" }),
		).toBeVisible();
	});

	test("rejects a wrong password without leaving the login page", async ({
		page,
	}) => {
		await page.goto("/login");
		await page.getByLabel("Email or username").fill(PLAYER.username);
		await page
			.getByRole("textbox", { exact: true, name: "Password" })
			.fill("definitely-not-the-password");
		await page.getByRole("button", { name: "Login", exact: true }).click();

		await expect(page).toHaveURL(/\/login$/);
		await expect(
			page.getByRole("textbox", { exact: true, name: "Password" }),
		).toBeVisible();
	});

	test("forces a password change before the workbench opens", async ({
		browser,
	}) => {
		const username = uniqueUsername("frc");
		const initialPassword = `Fr9-${uniqueUsername("pw")}!x`;
		const admin = await adminApiSession(e2eBaseURL());
		try {
			await createUserViaApi(admin, {
				email: `${username}@example.test`,
				mustChangePassword: true,
				password: initialPassword,
				role: "user",
				username,
			});
		} finally {
			await admin.dispose();
		}

		const context = await browser.newContext({
			storageState: { cookies: [], origins: [] },
		});
		const page = await context.newPage();
		try {
			// The authenticated gate must land on the forced change flow and
			// keep the user away from any other page until it completes.
			await loginViaUiExpectingPasswordChange(page, username, initialPassword);
			await page.goto("/account/profiles");
			await expect(page).toHaveURL(/\/force-password-change$/);

			const newPassword = `Ch7-${uniqueUsername("np")}!x`;
			await changeForcedPasswordViaUi(page, initialPassword, newPassword);

			// The new password is live: sign in again from a clean context.
			const secondContext = await browser.newContext({
				storageState: { cookies: [], origins: [] },
			});
			const secondPage = await secondContext.newPage();
			try {
				await loginViaUi(secondPage, username, newPassword);
				await expect(secondPage).toHaveURL(/\/account$/);
			} finally {
				await secondContext.close();
			}
		} finally {
			await context.close();
		}
	});

	test("recovers the session from the refresh cookie after access expiry", async ({
		browser,
	}) => {
		const context = await browser.newContext({
			storageState: { cookies: [], origins: [] },
		});
		const page = await context.newPage();
		try {
			await loginViaUi(page, PLAYER.username, PLAYER.password);

			// Simulate a short-lived access token expiring mid-session: drop the
			// access cookie while the long-lived refresh cookie stays. The HTTP
			// layer must refresh transparently instead of showing "login required".
			await context.clearCookies({ name: ACCESS_COOKIE_NAME });
			await expect
				.poll(async () =>
					(await context.cookies()).some(
						(cookie) => cookie.name === ACCESS_COOKIE_NAME,
					),
				)
				.toBe(false);
			expect(
				(await context.cookies()).some(
					(cookie) => cookie.name === REFRESH_COOKIE_NAME,
				),
			).toBe(true);

			await page.goto("/account");
			await expect(page).toHaveURL(/\/account$/);
			await expect(
				page.getByRole("button", { name: PLAYER.username }),
			).toBeVisible();
			await expect(
				page.getByRole("heading", { name: "Login required" }),
			).toHaveCount(0);
		} finally {
			await context.close();
		}
	});

	test("falls back to the login wall after the server revoked the session", async ({
		browser,
	}) => {
		// A dedicated account keeps this spec from revoking the shared player
		// sessions other parallel workers rely on.
		const username = uniqueUsername("rvk");
		const password = `Rv4-${uniqueUsername("pw")}!x`;
		const admin = await adminApiSession(e2eBaseURL());
		try {
			await createUserViaApi(admin, {
				email: `${username}@example.test`,
				password,
				role: "user",
				username,
			});
		} finally {
			await admin.dispose();
		}

		const context = await browser.newContext({
			storageState: { cookies: [], origins: [] },
		});
		const page = await context.newPage();
		try {
			await loginViaUi(page, username, password);

			// Revoke every session for this account through the admin API. The
			// browser still holds both cookies; the refresh handshake must be
			// rejected server-side and the UI must end on the login wall.
			const adminAgain = await adminApiSession(e2eBaseURL());
			try {
				const created = await findUserByUsername(adminAgain, username);
				expect(created).toBeTruthy();
				await revokeUserSessionsViaApi(adminAgain, created?.id ?? 0);
			} finally {
				await adminAgain.dispose();
			}

			await page.goto("/account");
			await expect(
				page.getByRole("heading", { name: "Login required" }),
			).toBeVisible();
		} finally {
			await context.close();
		}
	});

	test("exposes /api/v1/auth/me only for valid sessions", async () => {
		const anonymous = await apiSession(e2eBaseURL());
		try {
			const response = await anonymous.get("/api/v1/auth/me");
			expect(response.status()).toBe(401);
			const payload = (await response.json()) as {
				code?: string;
				msg?: string;
			};
			expect(payload.code).toBeDefined();
			expect(payload.code).not.toBe("success");
		} finally {
			await anonymous.dispose();
		}

		const admin = await adminApiSession(e2eBaseURL());
		try {
			const response = await admin.get("/api/v1/auth/me");
			expect(response.status()).toBe(200);
			const payload = (await response.json()) as {
				data?: { username: string; role: string };
			};
			expect(payload.data?.username).toBe(ADMIN.username);
			expect(payload.data?.role).toBe("admin");
		} finally {
			await admin.dispose();
		}
	});

	test("keeps an API login usable for session reuse via storageState", async () => {
		// Cover the seeding path the setup project relies on: API login plus
		// storageState round-trip must yield a working authenticated session.
		const loginContext = await apiSession(e2eBaseURL());
		const stateFile = `test-results/.auth/auth-spec-${uniqueUsername("st")}.json`;
		try {
			await loginViaApi(loginContext, PLAYER.username, PLAYER.password);
			await loginContext.storageState({ path: stateFile });
		} finally {
			await loginContext.dispose();
		}

		const replayContext = await apiSession(e2eBaseURL(), {
			storageState: stateFile,
		});
		try {
			const response = await replayContext.get("/api/v1/auth/me");
			expect(response.status()).toBe(200);
			const payload = (await response.json()) as {
				data?: { username: string };
			};
			expect(payload.data?.username).toBe(PLAYER.username);
		} finally {
			await replayContext.dispose();
		}
	});
});
