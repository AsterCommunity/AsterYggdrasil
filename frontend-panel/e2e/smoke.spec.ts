import { expect, test } from "@playwright/test";
import { loginViaUi, logoutViaUi } from "./support/auth";
import { ADMIN } from "./support/fixtures";

// Smoke checks cover the guest-visible entry points, so they run without any
// seeded session even though the project provides one by default.
test.use({ storageState: { cookies: [], origins: [] } });

// Kept from the original smoke suite: the first-run entry points a brand-new
// instance depends on. Accounts and initialization come from the setup
// project; credentials live in `support/fixtures.ts` (env-overridable), not
// inline in this file.
async function ensureInitialized(page: import("@playwright/test").Page) {
	await page.goto("/init");
	if (
		await page
			.getByRole("heading", { name: "Setup already complete" })
			.isVisible()
	) {
		return;
	}
	await page.getByLabel("Username").fill(ADMIN.username);
	await page.getByLabel("Email").fill(ADMIN.email);
	await page
		.getByRole("textbox", { exact: true, name: "Password" })
		.fill(ADMIN.password);
	await page.getByLabel("Confirm password").fill(ADMIN.password);
	await page.getByRole("button", { name: "Create admin" }).click();
	await expect(page).toHaveURL(/\/account$/);
	await logoutViaUi(page, ADMIN.username);
}

test("serves the public Yggdrasil entry screen", async ({ page }) => {
	await ensureInitialized(page);

	await page.goto("/");
	await expect(page).toHaveURL(/\/$/);
	await expect(
		page.getByRole("heading", { name: "Your Minecraft identity and skin hub" }),
	).toBeVisible();
	await expect(
		page.getByRole("link", { name: "Login / Register" }),
	).toBeVisible();
	await expect(page.getByText("Safe and reliable")).toBeVisible();
	await expect(page.getByText("Skin management")).toBeVisible();
	await expect(page.getByText("Fast and stable")).toBeVisible();
});

test("shows login as a standalone route and protects account routes", async ({
	page,
}) => {
	await ensureInitialized(page);

	await page.goto("/login");
	await expect(page).toHaveURL(/\/login$/);
	await expect(page.getByRole("heading", { name: "Login" })).toBeVisible();
	await expect(page.getByLabel("Email or username")).toBeVisible();
	await expect(
		page.getByRole("textbox", { exact: true, name: "Password" }),
	).toBeVisible();
	await page.getByRole("link", { name: "Register now" }).click();
	await expect(page).toHaveURL(/\/register$/);
	await expect(
		page.getByRole("heading", { name: "Create account" }),
	).toBeVisible();
	await expect(page.getByLabel("Confirm password")).toBeVisible();
	await page.getByRole("link", { name: "Login" }).click();
	await expect(page).toHaveURL(/\/login$/);

	await page.goto("/account");
	await expect(
		page.getByRole("heading", { name: "Login required" }),
	).toBeVisible();
	await expect(page.getByRole("link", { name: "Go to login" })).toHaveAttribute(
		"href",
		"/login",
	);
});

test("keeps the public entry usable on mobile", async ({ page }) => {
	await ensureInitialized(page);

	await page.setViewportSize({ width: 390, height: 844 });
	await page.goto("/");

	await expect(
		page.getByRole("link", { name: "Get started" }).first(),
	).toBeVisible();
	await expect(
		page.getByRole("link", { name: "Public texture library" }).first(),
	).toBeVisible();
	await expect(page.getByText("Safe and reliable")).toBeVisible();
	await expect(page.getByText("Skin management")).toBeVisible();
});

test("keeps app topbar compact across account and admin shells", async ({
	page,
}) => {
	await ensureInitialized(page);
	await loginViaUi(page, ADMIN.username, ADMIN.password);

	await page.setViewportSize({ width: 390, height: 844 });
	await expect(page).toHaveURL(/\/account$/);
	await expect(page.getByRole("banner").getByRole("link")).toHaveCount(0);
	await expect(
		page.getByPlaceholder("Search players, UUIDs, sessions, or settings..."),
	).toHaveCount(0);
	await expect(page.getByText("⌘K")).toHaveCount(0);

	await page.setViewportSize({ width: 1440, height: 960 });
	await expect(
		page.getByPlaceholder("Search players, UUIDs, sessions, or settings..."),
	).toHaveCount(0);
	await expect(page.getByText("⌘K")).toHaveCount(0);

	await page.setViewportSize({ width: 390, height: 844 });
	await page.goto("/admin/settings");
	await expect(page).toHaveURL(/\/admin\/settings$/);
	await expect(page.getByRole("banner").getByRole("link")).toBeVisible();
	await expect(
		page.getByPlaceholder("Search players, UUIDs, sessions, or settings..."),
	).toHaveCount(0);
	await expect(page.getByText("⌘K")).toHaveCount(0);

	await logoutViaUi(page, ADMIN.username);
});

test("keeps users logged out after reload once logout succeeds", async ({
	page,
}) => {
	await ensureInitialized(page);
	await loginViaUi(page, ADMIN.username, ADMIN.password);

	await logoutViaUi(page, ADMIN.username);
	await expect(page).toHaveURL(/\/$/);

	// Regression for issue #37: after a successful logout, a page reload must
	// not resurrect the session; protected routes stay inaccessible.
	await page.reload();
	await expect(
		page.getByRole("link", { name: "Login / Register" }),
	).toBeVisible();

	await page.goto("/account");
	await expect(
		page.getByRole("heading", { name: "Login required" }),
	).toBeVisible();
	await expect(page.getByRole("link", { name: "Go to login" })).toHaveAttribute(
		"href",
		"/login",
	);
});
