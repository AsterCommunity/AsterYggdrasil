import { expect, test } from "@playwright/test";
import { loginViaUi, logoutViaUi } from "./support/auth";
import { ADMIN, STORAGE_STATE_FILES } from "./support/fixtures";

test.describe("route guards", () => {
	test("a signed-in user hitting /login sees the already-signed-in state", async ({
		page,
	}) => {
		// Real user flow: sign in once (cookies + local session marker), then
		// revisit the guest-only routes.
		await loginViaUi(page, ADMIN.username, ADMIN.password);
		await logoutViaUi(page, ADMIN.username);
		await loginViaUi(page, ADMIN.username, ADMIN.password);

		await page.goto("/login");
		await expect(
			page.getByRole("heading", { name: "Already signed in" }),
		).toBeVisible();
		const openWorkbench = page.getByRole("link", {
			name: "Open workbench",
		});
		await expect(openWorkbench).toHaveAttribute("href", "/account");

		await page.goto("/register");
		await expect(
			page.getByRole("heading", { name: "Already signed in" }),
		).toBeVisible();
	});

	test("guests keep login and register reachable without a session", async ({
		browser,
	}) => {
		const context = await browser.newContext({
			storageState: { cookies: [], origins: [] },
		});
		const page = await context.newPage();
		try {
			await page.goto("/login");
			await expect(page).toHaveURL(/\/login$/);
			await expect(
				page.getByRole("heading", { name: "Login", exact: true }),
			).toBeVisible();

			await page.goto("/register");
			await expect(
				page.getByRole("heading", { name: "Create account" }),
			).toBeVisible();
		} finally {
			await context.close();
		}
	});

	test("protected account routes show the login wall for guests", async ({
		browser,
	}) => {
		const context = await browser.newContext({
			storageState: { cookies: [], origins: [] },
		});
		const page = await context.newPage();
		try {
			await page.goto("/account");
			await expect(
				page.getByRole("heading", { name: "Login required" }),
			).toBeVisible();
			await expect(
				page.getByRole("link", { name: "Go to login" }),
			).toHaveAttribute("href", "/login");
		} finally {
			await context.close();
		}
	});

	test("a plain user gets the admin-required state instead of admin pages", async ({
		browser,
	}) => {
		const context = await browser.newContext({
			storageState: STORAGE_STATE_FILES.player,
		});
		const page = await context.newPage();
		try {
			await page.goto("/admin");
			await expect(
				page.getByRole("heading", { name: "Admin access required" }),
			).toBeVisible();
			const backLink = page.getByRole("link", {
				name: "Back to workbench",
			});
			await expect(backLink).toHaveAttribute("href", "/account");

			await page.goto("/admin/users");
			await expect(
				page.getByRole("heading", { name: "Admin access required" }),
			).toBeVisible();
		} finally {
			await context.close();
		}
	});

	test("an operator scoped to users can open /admin/users but not settings", async ({
		browser,
	}) => {
		const context = await browser.newContext({
			storageState: STORAGE_STATE_FILES.operator,
		});
		const page = await context.newPage();
		try {
			await page.goto("/admin/users");
			await expect(
				page.getByRole("heading", { name: "User Management" }),
			).toBeVisible();
			await expect(page).toHaveURL(/\/admin\/users$/);

			await page.goto("/admin/settings");
			await expect(
				page.getByRole("heading", { name: "Admin access required" }),
			).toBeVisible();
		} finally {
			await context.close();
		}
	});

	test("an initialized instance turns /init into the setup-complete state", async ({
		page,
	}) => {
		await page.goto("/init");
		await expect(
			page.getByRole("heading", { name: "Setup already complete" }),
		).toBeVisible();
		const toLogin = page.getByRole("link", { name: "Go to login" });
		await expect(toLogin).toHaveAttribute("href", "/login");
	});

	test("shows the public entry without any session", async ({ browser }) => {
		const context = await browser.newContext({
			storageState: { cookies: [], origins: [] },
		});
		const page = await context.newPage();
		try {
			await page.goto("/");
			await expect(page).toHaveURL(/\/$/);
			await expect(
				page.getByRole("heading", {
					name: "Your Minecraft identity and skin hub",
				}),
			).toBeVisible();
			await expect(
				page.getByRole("link", { name: "Login / Register" }),
			).toBeVisible();
		} finally {
			await context.close();
		}
	});
});
