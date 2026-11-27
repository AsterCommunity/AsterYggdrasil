import { expect, test } from "@playwright/test";
import { seedProtocolFixtures, YGGDRASIL_ROOT } from "./support/seed";

test.describe("authlib-injector service metadata", () => {
	test.beforeAll(async ({ request }) => {
		await seedProtocolFixtures(request);
	});

	test("serves the metadata document with the authlib-injector wire shape", async ({
		request,
	}) => {
		const response = await request.get(YGGDRASIL_ROOT);
		expect(response.status()).toBe(200);
		// Launchers treat the metadata as highly volatile; the backend marks it
		// must-revalidate so authlib-injector does not pin stale endpoints.
		expect(response.headers()["cache-control"]).toBe(
			"no-cache, no-store, must-revalidate",
		);

		const meta = (await response.json()) as {
			meta: {
				serverName: string;
				implementationName: string;
				implementationVersion: string;
				links?: { homepage?: string; register?: string };
				"feature.non_email_login": boolean;
				"feature.enable_profile_key": boolean;
				"feature.enable_mojang_anti_features": boolean;
				"feature.username_check": boolean;
			};
			skinDomains: string[];
			signaturePublickey: string;
		};

		expect(meta.meta.serverName.length).toBeGreaterThan(0);
		expect(meta.meta.implementationName).toBe("AsterYggdrasil");
		expect(meta.meta.implementationVersion.length).toBeGreaterThan(0);
		// authlib-injector encodes feature flags as dotted keys inside `meta`,
		// not nested objects.
		expect(meta.meta["feature.non_email_login"]).toBe(true);
		expect(meta.meta["feature.username_check"]).toBe(true);
		expect(typeof meta.meta["feature.enable_profile_key"]).toBe("boolean");
		expect(typeof meta.meta["feature.enable_mojang_anti_features"]).toBe(
			"boolean",
		);

		// Registration is enabled on a fresh instance, so the launcher shows
		// the register link pointing at the panel route.
		expect(meta.meta.links?.register).toContain("/register");
		expect(meta.meta.links?.homepage).toContain("http");

		expect(Array.isArray(meta.skinDomains)).toBe(true);
		expect(meta.skinDomains.length).toBeGreaterThan(0);
		expect(meta.signaturePublickey).toContain("BEGIN PUBLIC KEY");
	});

	test("answers on the trailing-slash API root variant too", async ({
		request,
	}) => {
		const response = await request.get(`${YGGDRASIL_ROOT}/`);
		expect(response.status()).toBe(200);
		const meta = (await response.json()) as {
			meta: { implementationName: string };
		};
		expect(meta.meta.implementationName).toBe("AsterYggdrasil");
	});
});
