import { beforeEach, describe, expect, it, vi } from "vitest";
import { STORAGE_KEYS } from "@/lib/storage";
import type { PublicFrontendConfig } from "@/types/api";

const frontendConfigServiceMock = vi.hoisted(() => ({
	get: vi.fn(),
}));

vi.mock("@/services/frontendConfigService", () => ({
	frontendConfigService: frontendConfigServiceMock,
}));

const frontendConfig = {
	version: 1,
	branding: {
		allow_user_registration: true,
		passkey_login_enabled: false,
		description: "A test Yggdrasil server",
		favicon_url: "/test-icon.svg",
		site_urls: ["https://example.test"],
		title: "TestYggdrasil",
		wordmark_dark_url: "/wordmark-dark.svg",
		wordmark_light_url: "/wordmark-light.svg",
	},
	yggdrasil: {
		allow_cape_upload: true,
		allow_profile_name_login: true,
		allow_skin_upload: true,
		max_texture_pixels: 4096 * 4096,
		max_texture_upload_bytes: 4 * 1024 * 1024,
		public_base_urls: ["https://example.test/api/yggdrasil"],
		server_name: "TestYggdrasil",
		skin_domains: ["textures.example.test"],
	},
} satisfies PublicFrontendConfig;

async function loadStore() {
	vi.resetModules();
	return await import("@/stores/frontendConfigStore");
}

describe("frontendConfigStore cache", () => {
	beforeEach(() => {
		localStorage.clear();
		frontendConfigServiceMock.get.mockReset();
		frontendConfigServiceMock.get.mockResolvedValue(frontendConfig);
		document.title = "";
		document.head.innerHTML = "";
	});

	it("hydrates initial state from a valid cached config", async () => {
		localStorage.setItem(
			STORAGE_KEYS.cachedFrontendConfig,
			JSON.stringify({
				config: frontendConfig,
				cachedAt: 123,
			}),
		);

		const { useFrontendConfigStore } = await loadStore();

		expect(useFrontendConfigStore.getState()).toMatchObject({
			allowUserRegistration: true,
			passkeyLoginEnabled: false,
			isLoaded: true,
			yggdrasil: {
				server_name: "TestYggdrasil",
			},
		});
		expect(useFrontendConfigStore.getState().branding.title).toBe(
			"TestYggdrasil",
		);
	});

	it("removes malformed JSON cache entries during startup", async () => {
		localStorage.setItem(STORAGE_KEYS.cachedFrontendConfig, "{");

		const { useFrontendConfigStore } = await loadStore();

		expect(useFrontendConfigStore.getState().isLoaded).toBe(false);
		expect(localStorage.getItem(STORAGE_KEYS.cachedFrontendConfig)).toBeNull();
	});

	it("removes invalid cache shapes during startup", async () => {
		localStorage.setItem(
			STORAGE_KEYS.cachedFrontendConfig,
			JSON.stringify({
				config: {
					version: 1,
				},
			}),
		);

		const { useFrontendConfigStore } = await loadStore();

		expect(useFrontendConfigStore.getState().isLoaded).toBe(false);
		expect(localStorage.getItem(STORAGE_KEYS.cachedFrontendConfig)).toBeNull();
	});

	it("writes fetched configs to cache", async () => {
		const { useFrontendConfigStore } = await loadStore();

		await useFrontendConfigStore.getState().load({ force: true });

		expect(frontendConfigServiceMock.get).toHaveBeenCalledTimes(1);
		expect(useFrontendConfigStore.getState().branding.title).toBe(
			"TestYggdrasil",
		);
		expect(localStorage.getItem(STORAGE_KEYS.cachedFrontendConfig)).toContain(
			"TestYggdrasil",
		);
	});

	it("applies fetched configs when cache writes fail", async () => {
		const setItemSpy = vi
			.spyOn(Storage.prototype, "setItem")
			.mockImplementation(() => {
				throw new Error("cache write blocked");
			});

		try {
			const { useFrontendConfigStore } = await loadStore();

			await useFrontendConfigStore.getState().load({ force: true });

			expect(useFrontendConfigStore.getState()).toMatchObject({
				isLoaded: true,
				passkeyLoginEnabled: false,
			});
			expect(useFrontendConfigStore.getState().branding.title).toBe(
				"TestYggdrasil",
			);
		} finally {
			setItemSpy.mockRestore();
		}
	});

	it("falls back to defaults when the fetched config is invalid", async () => {
		frontendConfigServiceMock.get.mockResolvedValue({
			version: 1,
		});
		const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

		try {
			const { useFrontendConfigStore } = await loadStore();

			await useFrontendConfigStore.getState().load({ force: true });

			expect(useFrontendConfigStore.getState()).toMatchObject({
				allowUserRegistration: true,
				config: null,
				isLoaded: true,
				yggdrasil: null,
			});
		} finally {
			warnSpy.mockRestore();
		}
	});
});
