import { type APIRequestContext, expect, test } from "@playwright/test";
import { PLAYER } from "../support/fixtures";
import {
	type SeedResult,
	seedProtocolFixtures,
	YGGDRASIL_ROOT,
} from "./support/seed";

interface YggdrasilProfile {
	id: string;
	name: string;
	properties?: Array<{ name: string; value: string; signature?: string }>;
}

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
		clientToken: string;
		selectedProfile?: { id: string };
	};
}

test.describe("profile lookup and sessionserver", () => {
	let seed: SeedResult;

	test.beforeAll(async ({ request }) => {
		seed = await seedProtocolFixtures(request);
	});

	test("resolves profile names in batches and skips unknown names", async ({
		request,
	}) => {
		const response = await request.post(
			`${YGGDRASIL_ROOT}/api/profiles/minecraft`,
			{
				data: [seed.profileName, "NoSuchPlayer42"],
			},
		);
		expect(response.status()).toBe(200);

		const profiles = (await response.json()) as YggdrasilProfile[];
		expect(profiles).toHaveLength(1);
		expect(profiles[0].name).toBe(seed.profileName);
		expect(profiles[0].id).toBe(seed.profileUuid);
		// Batch lookups never carry signatures.
		expect(profiles[0].properties).toBeUndefined();
	});

	test("rejects invalid batch entries with a 400", async ({ request }) => {
		const invalid = await request.post(
			`${YGGDRASIL_ROOT}/api/profiles/minecraft`,
			{ data: ["this-name-is-way-too-long-for-minecraft"] },
		);
		expect(invalid.status()).toBe(400);
		const body = (await invalid.json()) as { error: string };
		expect(body.error).toBe("IllegalArgumentException");

		// An empty batch is a valid request that resolves to nothing.
		const empty = await request.post(
			`${YGGDRASIL_ROOT}/api/profiles/minecraft`,
			{ data: [] },
		);
		expect(empty.status()).toBe(200);
		expect((await empty.json()) as unknown[]).toEqual([]);
	});

	test("serves the profile by UUID with its textures property", async ({
		request,
	}) => {
		const response = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/profile/${seed.profileUuid}`,
		);
		expect(response.status()).toBe(200);

		const profile = (await response.json()) as YggdrasilProfile;
		expect(profile.id).toBe(seed.profileUuid);
		expect(profile.name).toBeTruthy();

		// Slot keys follow the authlib-injector uppercase convention.
		const decoded = decodeTexturesProperty(profile);
		expect(decoded.profileId).toBe(seed.profileUuid);
		expect(decoded.profileName).toBe(profile.name);
		expect(Object.keys(decoded.textures)).toContain("SKIN");
		expect(decoded.textures.SKIN?.url).toContain("/textures/");
	});

	test("signs the textures property when explicitly requested", async ({
		request,
	}) => {
		const response = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/profile/${seed.profileUuid}?unsigned=false`,
		);
		expect(response.status()).toBe(200);

		const profile = (await response.json()) as YggdrasilProfile;
		const textures = profile.properties?.find(
			(property) => property.name === "textures",
		);
		expect(textures).toBeTruthy();
		// The RSA signature over `name.value` is present and non-trivial.
		expect(textures?.signature?.length ?? 0).toBeGreaterThan(16);
	});

	test("omits signatures when requested unsigned", async ({ request }) => {
		const response = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/profile/${seed.profileUuid}?unsigned=true`,
		);
		expect(response.status()).toBe(200);
		const profile = (await response.json()) as YggdrasilProfile;
		const textures = profile.properties?.find(
			(property) => property.name === "textures",
		);
		expect(textures).toBeTruthy();
		expect(textures?.signature).toBeUndefined();
	});

	function decodeTexturesProperty(profile: YggdrasilProfile) {
		const texturesProperty = profile.properties?.find(
			(property) => property.name === "textures",
		);
		expect(texturesProperty).toBeTruthy();
		return JSON.parse(
			Buffer.from(texturesProperty?.value ?? "", "base64").toString("utf8"),
		) as {
			profileId: string;
			profileName: string;
			textures: Record<string, { url: string } | undefined>;
		};
	}

	test("answers 204 for unknown UUIDs and 400 for malformed ones", async ({
		request,
	}) => {
		const unknown = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/profile/${"0".repeat(32)}`,
		);
		expect(unknown.status()).toBe(204);
		expect((await unknown.text()).length).toBe(0);

		const malformed = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/profile/not-a-uuid`,
		);
		expect(malformed.status()).toBe(400);
		expect(((await malformed.json()) as { error: string }).error).toBe(
			"IllegalArgumentException",
		);
	});

	test("joins a server and verifies it through hasJoined", async ({
		request,
	}) => {
		const token = await authenticate(
			request,
			seed.profileName,
			PLAYER.password,
		);
		expect(token.selectedProfile?.id).toBe(seed.profileUuid);

		const serverId = "e2e-server-id-hash";
		const join = await request.post(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/join`,
			{
				data: {
					accessToken: token.accessToken,
					selectedProfile: seed.profileUuid,
					serverId,
				},
			},
		);
		expect(join.status()).toBe(204);

		const hasJoined = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/hasJoined`,
			{ params: { username: seed.profileName, serverId } },
		);
		expect(hasJoined.status()).toBe(200);
		const joined = (await hasJoined.json()) as YggdrasilProfile;
		expect(joined.id).toBe(seed.profileUuid);
		expect(joined.name).toBe(seed.profileName);

		// Wrong server id or unknown name yields 204 with no body.
		const wrongServer = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/hasJoined`,
			{ params: { username: seed.profileName, serverId: "other-server" } },
		);
		expect(wrongServer.status()).toBe(204);
		const wrongName = await request.get(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/hasJoined`,
			{ params: { username: "NobodyHere42", serverId } },
		);
		expect(wrongName.status()).toBe(204);
	});

	test("rejects a join for a profile the token does not own", async ({
		request,
	}) => {
		const token = await authenticate(
			request,
			seed.profileName,
			PLAYER.password,
		);
		const join = await request.post(
			`${YGGDRASIL_ROOT}/sessionserver/session/minecraft/join`,
			{
				data: {
					accessToken: token.accessToken,
					selectedProfile: "f".repeat(32),
					serverId: "foreign-profile",
				},
			},
		);
		// A foreign or unknown profile is a protocol-level 403.
		expect([403, 400]).toContain(join.status());
		const body = (await join.json()) as { error: string };
		expect([
			"ForbiddenOperationException",
			"IllegalArgumentException",
		]).toContain(body.error);
	});
});
