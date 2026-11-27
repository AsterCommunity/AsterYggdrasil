import { expect, test } from "@playwright/test";
import {
	adminApiSession,
	apiGet,
	apiPut,
	apiSession,
	deleteUserViaApi,
	expectOk,
	findUserByUsername,
	submitLibraryReviewViaApi,
	updateUserViaApi,
	uploadWardrobeTextureViaApi,
} from "./support/api";
import {
	createUniqueSkinPng,
	e2eBaseURL,
	STORAGE_STATE_FILES,
	uniqueName,
	uniqueUsername,
} from "./support/fixtures";

const OPERATOR_USERS_SCOPE = "com.astercosm.yggdrasil.permission.admin.users";

test.describe("admin console", () => {
	// Admin specs run with the seeded admin storageState.
	test.use({ storageState: STORAGE_STATE_FILES.admin });

	test("creates a user through the management page and deletes it again", async ({
		page,
	}) => {
		const username = uniqueUsername("adm");
		const password = `Ad6-${uniqueUsername("pw")}!x`;

		await page.goto("/admin/users");
		await expect(
			page.getByRole("heading", { name: "User Management" }),
		).toBeVisible();

		await page.getByRole("button", { name: "Add user" }).click();
		const dialog = page.getByRole("dialog", { name: "Add user" });
		await expect(dialog).toBeVisible();
		await dialog.getByLabel("Username").fill(username);
		await dialog.getByLabel("Email").fill(`${username}@example.test`);
		await dialog.getByLabel("Password", { exact: true }).fill(password);
		await dialog.getByRole("button", { name: "Create", exact: true }).click();
		await expect(dialog).toBeHidden();
		await expect(page.getByText("User created")).toBeVisible();
		await expect(
			page.getByRole("cell", { name: username }).first(),
		).toBeVisible();

		// Backend-visible outcome: the account exists and can be re-scoped.
		const admin = await adminApiSession(e2eBaseURL());
		try {
			const found = await findUserByUsername(admin, username);
			expect(found).toBeTruthy();
			const userId = found?.id ?? 0;

			// Operator-scope round-trip: promote to an operator scoped to
			// user management only, then back to a plain user.
			const updated = await updateUserViaApi(admin, userId, {
				operator_scopes: [OPERATOR_USERS_SCOPE],
				role: "operator",
			});
			expect(updated.role).toBe("operator");
			expect(updated.operator_scopes).toContain(OPERATOR_USERS_SCOPE);

			await deleteUserViaApi(admin, userId);
		} finally {
			await admin.dispose();
		}

		await page.reload();
		await expect(page.getByRole("cell", { name: username })).toHaveCount(0);
	});

	test("moderates a submitted texture from the review queue", async ({
		request,
		page,
	}) => {
		// Seed: player uploads a uniquely named texture and submits it for
		// library review.
		const displayName = uniqueName("review-skin");
		const playerContext = await apiSession(e2eBaseURL(), {
			storageState: STORAGE_STATE_FILES.player,
		});
		let textureId: number | undefined;
		try {
			// Library submissions require a public texture. The bytes are unique
			// per call: parallel account specs upload for the same player, and
			// the backend deduplicates wardrobe rows by content hash — a shared
			// image would hand this spec a private row owned by another spec.
			const texture = await uploadWardrobeTextureViaApi(
				playerContext,
				"skin",
				createUniqueSkinPng(),
				{ displayName, visibility: "public" },
			);
			textureId = texture.id;
			await submitLibraryReviewViaApi(playerContext, texture.id);
		} finally {
			await playerContext.dispose();
		}

		await page.goto("/admin/texture-library/reviews");
		await expect(
			page.getByRole("heading", { name: "Texture Reviews" }),
		).toBeVisible();

		const row = page.getByRole("row").filter({ hasText: displayName }).first();
		await expect(row).toBeVisible();
		await row.getByRole("button", { name: "Approve" }).click();

		const dialog = page.getByRole("dialog", {
			name: "Approve this texture?",
		});
		await expect(dialog).toBeVisible();
		await dialog.getByRole("button", { name: "Approve" }).click();
		await expect(dialog).toBeHidden();

		// Public-library outcome: the approved texture is listed as published
		// through the same API the public gallery uses.
		const library = expectOk<{
			items: Array<{ id: number; library_status: string }>;
		}>(
			await apiGet(
				request,
				"/api/v1/texture-library/textures?library_status=published",
			),
		);
		expect(
			library.items.some((item) => item.id === textureId),
			"approved texture should be listed as published",
		).toBe(true);
	});

	test("saves a runtime setting and reads it back through the API", async ({
		page,
	}) => {
		const admin = await adminApiSession(e2eBaseURL());
		const configUrl = "/api/v1/admin/config/auth_access_token_ttl_secs";
		let original: string | null = null;
		try {
			original = expectOk<{ value: string }>(
				await apiGet(admin, configUrl),
			).value;

			await page.goto("/admin/settings/auth");
			await expect(
				page.getByRole("heading", { name: "Settings" }),
			).toBeVisible();

			// The row is the innermost container holding both the label text and
			// the numeric input. The `_secs` field renders with a unit selector:
			// the displayed number is in the selected unit, not raw seconds, so
			// the unit is pinned to "Seconds" first to keep the typed value
			// unambiguous.
			const row = page
				.locator("div")
				.filter({ has: page.getByText("Access token lifetime") })
				.filter({ has: page.getByRole("spinbutton") })
				.last();
			const input = row.getByRole("spinbutton");
			await expect(input).toBeVisible();

			await row.getByRole("combobox", { name: "Time unit" }).click();
			await page.getByRole("option", { name: "Seconds" }).click();
			await input.fill("1800");

			await page
				.getByTestId("settings-save-bar")
				.getByRole("button", { name: "Save changes" })
				.click();
			await expect(page.getByText(/saved/i).first()).toBeVisible({
				timeout: 30_000,
			});

			// Backend round-trip: the API, not the reloaded UI, is the source of
			// truth — the unit-aware input would re-render 1800s as "30" hours.
			await expect
				.poll(async () => {
					const config = expectOk<{ value: string }>(
						await apiGet(admin, configUrl),
					);
					return config.value;
				})
				.toBe("1800");
		} finally {
			// Restore the original value so later runs start clean.
			if (original !== null) {
				await apiPut(admin, configUrl, { value: original });
			}
			await admin.dispose();
		}
	});
});
