import { expect, type Page } from "@playwright/test";

/**
 * Page-level auth operations. Specs keep these UI flows to the critical
 * user-visible outcomes; bulk data setup lives in `support/api.ts`.
 */

/** Logs in through the login page form and waits for the workbench. */
export async function loginViaUi(
	page: Page,
	identifier: string,
	password: string,
) {
	await page.goto("/login");
	await page.getByLabel("Email or username").fill(identifier);
	await page
		.getByRole("textbox", { exact: true, name: "Password" })
		.fill(password);
	await page.getByRole("button", { name: "Login", exact: true }).click();
	await expect(page).toHaveURL(/\/account$/);
}

/** Registers a new account through the register page form. */
export async function registerViaUi(
	page: Page,
	username: string,
	email: string,
	password: string,
) {
	await page.goto("/register");
	await page.getByLabel("Username").fill(username);
	await page.getByLabel("Email").fill(email);
	await page
		.getByRole("textbox", { exact: true, name: "Password" })
		.fill(password);
	await page.getByLabel("Confirm password").fill(password);
	// The terms checkbox is visually hidden (sr-only inside its label), so
	// click the visible label text to toggle it.
	await page
		.getByText(
			"I understand this account is used for Aster profile, texture, and authentication management.",
		)
		.click();
	const submit = page.getByRole("button", { name: "Register now" });
	await expect(submit).toBeEnabled({ timeout: 10_000 });
	await submit.click();
	await expect(page).toHaveURL(/\/account$/);
}

/**
 * Logs in through the login page form and waits for the forced password
 * change flow (the authenticated gate redirects must-change users there).
 */
export async function loginViaUiExpectingPasswordChange(
	page: Page,
	identifier: string,
	password: string,
) {
	await page.goto("/login");
	await page.getByLabel("Email or username").fill(identifier);
	await page
		.getByRole("textbox", { exact: true, name: "Password" })
		.fill(password);
	await page.getByRole("button", { name: "Login", exact: true }).click();
	await expect(page).toHaveURL(/\/force-password-change$/);
}

/** Logs out from the shell user menu and waits for the public entry. */
export async function logoutViaUi(page: Page, displayName: string) {
	await page.getByRole("button", { name: displayName }).click();
	await page.getByRole("menuitem", { name: "Logout" }).click();
	await expect(page).toHaveURL(/\/$/);
}

/**
 * Completes the forced password change flow: fill current/new password on
 * `/force-password-change` and wait for the workbench after success.
 */
export async function changeForcedPasswordViaUi(
	page: Page,
	currentPassword: string,
	newPassword: string,
) {
	await expect(page).toHaveURL(/\/force-password-change$/);
	await expect(
		page.getByRole("heading", { name: "Password change required" }),
	).toBeVisible();
	await page.getByLabel("Current password").fill(currentPassword);
	await page.getByLabel("New password", { exact: true }).fill(newPassword);
	await page.getByLabel("Confirm password").fill(newPassword);
	await page.getByRole("button", { name: "Change password" }).click();
	await expect(page).toHaveURL(/\/account$/);
}

/** Creates a Minecraft profile through the profiles page inline form. */
export async function createProfileViaUi(page: Page, profileName: string) {
	await page.goto("/account/profiles");
	await page.getByLabel("Profile name").fill(profileName);
	await page.getByRole("button", { name: "Create", exact: true }).click();
	await expect(page.getByText(profileName).first()).toBeVisible();
}
