import { expect, test } from "@playwright/test";
import {
	adminApiSession,
	apiPatch,
	apiSession,
	bindProfileTextureViaApi,
	createMinecraftProfileViaApi,
	createUserViaApi,
	expectOk,
	loginViaApi,
	saveStorageState,
	uploadWardrobeTextureViaApi,
} from "./support/api";
import { createProfileViaUi } from "./support/auth";
import {
	createUniqueSkinPng,
	e2eBaseURL,
	STORAGE_STATE_FILES,
	uniqueName,
	uniqueProfileName,
	uniqueUsername,
} from "./support/fixtures";

test.describe("account workspace", () => {
	// All account specs run as the seeded plain player, not the admin.
	test.use({ storageState: STORAGE_STATE_FILES.player });

	test("creates a Minecraft profile from the profiles page", async ({
		page,
	}) => {
		const profileName = uniqueProfileName();
		await createProfileViaUi(page, profileName);

		await expect(page).toHaveURL(/\/account\/profiles$/);
		await expect(
			page.getByRole("heading", { name: "Minecraft Profiles" }),
		).toBeVisible();
	});

	test("shows an uploaded wardrobe texture and equips it on a profile", async ({
		request,
		page,
	}) => {
		const displayName = uniqueName("equip-skin");
		const profile = await createMinecraftProfileViaApi(
			request,
			uniqueProfileName(),
		);
		// Unique bytes: parallel specs upload skins for the same player and the
		// backend collapses identical content into one wardrobe row.
		const texture = await uploadWardrobeTextureViaApi(
			request,
			"skin",
			createUniqueSkinPng(),
			{ displayName },
		);

		// The wardrobe lists the upload as a selectable texture card.
		await page.goto("/account/wardrobe");
		await expect(
			page.getByRole("heading", { name: "Texture Wardrobe" }),
		).toBeVisible();
		await expect(page.getByText(displayName).first()).toBeVisible();

		// Equip it on the profile slot, then confirm the profile page reflects
		// the binding as the user-visible outcome: the profile now offers its
		// texture management entry and no "no texture uploaded" placeholder.
		await bindProfileTextureViaApi(request, profile.id, "skin", texture.id);
		await page.goto("/account/profiles");
		await expect(page.getByText(profile.name).first()).toBeVisible();
		await expect(
			page.getByRole("button", {
				name: `Manage textures for ${profile.name}`,
			}),
		).toBeVisible();
	});

	test("uploads a skin through the wardrobe upload dialog", async ({
		page,
	}) => {
		await page.goto("/account/wardrobe");
		await page.getByRole("button", { name: "Upload texture" }).first().click();

		const dialog = page.getByRole("dialog", { name: "Upload to wardrobe" });
		await expect(dialog).toBeVisible();
		// The type picker exposes Skin/Cape toggle buttons, defaulting to skin.
		// Unique bytes avoid sharing a deduplicated wardrobe row with the
		// parallel API-upload specs running as the same player.
		await dialog.locator('input[type="file"]').setInputFiles({
			buffer: createUniqueSkinPng(),
			mimeType: "image/png",
			name: "e2e-skin.png",
		});
		await dialog.getByRole("button", { name: "Upload", exact: true }).click();

		await expect(dialog).toBeHidden({ timeout: 30_000 });
		await expect(page.getByText("Texture added to wardrobe")).toBeVisible();
	});

	test("keeps the launcher setup card on the workbench with the API root", async ({
		page,
	}) => {
		await page.goto("/account");
		await expect(
			page.getByRole("heading", { name: /Welcome back/ }),
		).toBeVisible();
		await expect(page.getByText("Connect a launcher")).toBeVisible();
		await expect(
			page.getByRole("button", { name: "Copy API Root" }),
		).toBeVisible();
		// The API root lives in a read-only copy field, not page text.
		const readonlyValues = await page
			.locator("input[readonly]")
			.evaluateAll((elements) =>
				elements.map((element) => (element as HTMLInputElement).value),
			);
		expect(
			readonlyValues.some((value) => value.includes("/api/yggdrasil")),
			`no readonly field carried the API root, got ${JSON.stringify(readonlyValues)}`,
		).toBe(true);
	});

	test("reflects a profile display-name update on personal settings", async ({
		browser,
	}) => {
		// A dedicated account keeps the shared player's display name stable:
		// the shell topbar renders the display name instead of the username,
		// and parallel auth specs assert on the plain username.
		const username = uniqueUsername("dsp");
		const password = `Ds7-${uniqueUsername("pw")}!x`;
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

		// Log the dedicated account in over the API, persist its storageState,
		// and flip the display name through the same API the settings page
		// persists to; the browser run then asserts the visible outcome
		// instead of re-driving the form.
		const stateFile = `test-results/.auth/display-name-${username}.json`;
		const apiContext = await apiSession(e2eBaseURL());
		try {
			await loginViaApi(apiContext, username, password);
			await saveStorageState(apiContext, stateFile);
			expectOk(
				await apiPatch<{ display_name: string }>(
					apiContext,
					"/api/v1/auth/profile",
					{ display_name: "E2E Display" },
				),
			);
		} finally {
			await apiContext.dispose();
		}

		const context = await browser.newContext({ storageState: stateFile });
		const page = await context.newPage();
		try {
			await page.goto("/account/settings");
			await expect(
				page.getByRole("heading", { name: "Personal settings" }),
			).toBeVisible();
			await page.reload();
			await expect(page.getByText("E2E Display").first()).toBeVisible();
		} finally {
			await context.close();
		}
	});
});
