import { type APIRequestContext, expect, test } from "@playwright/test";
import { createSkinPng, PLAYER } from "../support/fixtures";
import {
	type SeedResult,
	seedProtocolFixtures,
	YGGDRASIL_ROOT,
} from "./support/seed";

async function authenticate(
	request: APIRequestContext,
	username: string,
	password: string,
) {
	const response = await request.post(
		`${YGGDRASIL_ROOT}/authserver/authenticate`,
		{ data: { username, password } },
	);
	expect(response.status()).toBe(200);
	return (await response.json()) as {
		accessToken: string;
		selectedProfile?: { id: string };
	};
}

test.describe("texture upload, fetch semantics, and error shapes", () => {
	let seed: SeedResult;

	test.beforeAll(async ({ request }) => {
		seed = await seedProtocolFixtures(request);
	});

	test("uploads a skin with the bearer token and serves it by hash", async ({
		request,
	}) => {
		const token = await authenticate(
			request,
			seed.profileName,
			PLAYER.password,
		);
		const profileUuid = token.selectedProfile?.id ?? seed.profileUuid;

		// authlib-injector uploads are multipart with a PNG `file` field.
		const upload = await request.put(
			`${YGGDRASIL_ROOT}/api/user/profile/${profileUuid}/skin`,
			{
				multipart: {
					file: {
						buffer: createSkinPng(),
						mimeType: "image/png",
						name: "protocol-skin.png",
					},
				},
				headers: { Authorization: `Bearer ${token.accessToken}` },
			},
		);
		expect(upload.status()).toBe(204);

		// The profile now advertises the skin; decode the property and fetch
		// the texture by URL hash like a launcher would.
		const profileResponse = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/profile/${profileUuid}?unsigned=true`,
		);
		expect(profileResponse.status()).toBe(200);
		const profile = (await profileResponse.json()) as {
			properties?: Array<{ name: string; value: string }>;
		};
		const texturesProperty = profile.properties?.find(
			(property) => property.name === "textures",
		);
		expect(texturesProperty).toBeTruthy();
		const decoded = JSON.parse(
			Buffer.from(texturesProperty?.value ?? "", "base64").toString("utf8"),
		) as {
			timestamp: number;
			profileId: string;
			profileName: string;
			textures: Record<string, { url: string; metadata?: { model: string } }>;
		};
		expect(decoded.profileId).toBe(profileUuid);
		expect(typeof decoded.timestamp).toBe("number");
		const skinUrl = decoded.textures.SKIN?.url ?? "";
		expect(skinUrl).toContain("/textures/");

		const hash = skinUrl.split("/textures/")[1]?.split("?")[0] ?? "";
		expect(hash).toMatch(/^[0-9a-f]{64}$/);

		const textureResponse = await request.get(
			`${YGGDRASIL_ROOT}/textures/${hash}`,
		);
		expect(textureResponse.status()).toBe(200);
		// Texture blobs are immutable: long-lived caching headers are part of
		// the contract launchers rely on.
		expect(textureResponse.headers()["cache-control"]).toBeTruthy();
		expect(textureResponse.headers()["content-type"]).toContain("image/png");
		const body = await textureResponse.body();
		expect(body.subarray(0, 8)).toEqual(
			Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
		);
	});

	test("requires the Authorization header and rejects bad tokens", async ({
		request,
	}) => {
		// Missing Authorization is a 401 per authlib-injector semantics.
		const missing = await request.put(
			`${YGGDRASIL_ROOT}/api/user/profile/${seed.profileUuid}/skin`,
			{
				multipart: {
					file: {
						buffer: createSkinPng(),
						mimeType: "image/png",
						name: "no-auth.png",
					},
				},
			},
		);
		expect(missing.status()).toBe(401);
		const missingBody = (await missing.json()) as { error: string };
		expect(missingBody.error).toBe("ForbiddenOperationException");

		// A garbage token is rejected as invalid; the texture endpoints keep
		// the authlib-injector 401 semantics for token failures.
		const badToken = await request.put(
			`${YGGDRASIL_ROOT}/api/user/profile/${seed.profileUuid}/skin`,
			{
				headers: { Authorization: "Bearer not-a-real-token" },
				multipart: {
					file: {
						buffer: createSkinPng(),
						mimeType: "image/png",
						name: "bad-token.png",
					},
				},
			},
		);
		expect(badToken.status()).toBe(401);
	});

	test("rejects invalid profile UUIDs and invalid texture types", async ({
		request,
	}) => {
		const token = await authenticate(
			request,
			seed.profileName,
			PLAYER.password,
		);
		const headers = { Authorization: `Bearer ${token.accessToken}` };

		const badUuid = await request.put(
			`${YGGDRASIL_ROOT}/api/user/profile/not-a-uuid/skin`,
			{
				headers,
				multipart: {
					file: {
						buffer: createSkinPng(),
						mimeType: "image/png",
						name: "bad-uuid.png",
					},
				},
			},
		);
		expect(badUuid.status()).toBe(400);
		expect(((await badUuid.json()) as { error: string }).error).toBe(
			"IllegalArgumentException",
		);

		const badType = await request.put(
			`${YGGDRASIL_ROOT}/api/user/profile/${seed.profileUuid}/horns`,
			{
				headers,
				multipart: {
					file: {
						buffer: createSkinPng(),
						mimeType: "image/png",
						name: "bad-type.png",
					},
				},
			},
		);
		expect(badType.status()).toBe(400);
	});

	test("rejects textures that are not valid skins", async ({ request }) => {
		const token = await authenticate(
			request,
			seed.profileName,
			PLAYER.password,
		);
		const response = await request.put(
			`${YGGDRASIL_ROOT}/api/user/profile/${seed.profileUuid}/skin`,
			{
				headers: { Authorization: `Bearer ${token.accessToken}` },
				multipart: {
					file: {
						buffer: Buffer.from("definitely not a png"),
						mimeType: "image/png",
						name: "fake.png",
					},
				},
			},
		);
		expect(response.status()).toBe(400);
		const body = (await response.json()) as { error: string };
		expect(body.error).toBe("IllegalArgumentException");
	});

	test("deletes a texture slot through the protocol endpoint", async ({
		request,
	}) => {
		const token = await authenticate(
			request,
			seed.profileName,
			PLAYER.password,
		);
		const profileUuid = token.selectedProfile?.id ?? seed.profileUuid;

		const upload = await request.put(
			`${YGGDRASIL_ROOT}/api/user/profile/${profileUuid}/skin`,
			{
				headers: { Authorization: `Bearer ${token.accessToken}` },
				multipart: {
					file: {
						buffer: createSkinPng(),
						mimeType: "image/png",
						name: "to-delete.png",
					},
				},
			},
		);
		expect(upload.status()).toBe(204);

		const remove = await request.delete(
			`${YGGDRASIL_ROOT}/api/user/profile/${profileUuid}/skin`,
			{ headers: { Authorization: `Bearer ${token.accessToken}` } },
		);
		expect(remove.status()).toBe(204);

		const profileResponse = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/profile/${profileUuid}?unsigned=true`,
		);
		const profile = (await profileResponse.json()) as {
			properties?: Array<{ name: string; value: string }>;
		};
		const texturesProperty = profile.properties?.find(
			(property) => property.name === "textures",
		);
		if (texturesProperty) {
			const decoded = JSON.parse(
				Buffer.from(texturesProperty.value, "base64").toString("utf8"),
			) as { textures: Record<string, unknown> };
			expect(Object.keys(decoded.textures)).not.toContain("skin");
		}
	});

	test("answers 404 for unknown texture hashes", async ({ request }) => {
		const missing = await request.get(
			`${YGGDRASIL_ROOT}/textures/${"0".repeat(64)}`,
		);
		expect(missing.status()).toBe(404);
	});
});
