import {
	type APIRequestContext,
	expect,
	request as playwrightRequest,
} from "@playwright/test";
import { CSRF_COOKIE_NAME, CSRF_HEADER_NAME } from "../../support/api";
import {
	ADMIN,
	createSkinPng,
	PLAYER,
	protocolBaseURL,
	uniqueProfileName,
} from "../../support/fixtures";

export const YGGDRASIL_ROOT = "/api/yggdrasil";

export interface SeedResult {
	/** Unsigned lowercase UUID (32 hex chars) of the created profile. */
	profileUuid: string;
	profileName: string;
}

let seeded: SeedResult | null = null;

/** Reads the CSRF cookie from a request context's cookie jar. */
export async function csrfHeadersFor(
	request: APIRequestContext,
): Promise<Record<string, string>> {
	const state = await request.storageState();
	const headers: Record<string, string> = {};
	const token = state.cookies.find(
		(cookie) => cookie.name === CSRF_COOKIE_NAME,
	)?.value;
	if (token) {
		headers[CSRF_HEADER_NAME] = token;
	}
	return headers;
}

/**
 * Seeds the protocol fixtures once per worker:
 *
 * 1. initializes the instance via `/api/v1/auth/setup` when necessary,
 * 2. logs the admin in through the site API and creates the player account,
 * 3. creates a Minecraft profile and uploads a skin for it via the wardrobe,
 *    then binds it so protocol profile/texture endpoints have real data.
 *
 * Protocol specs must not rely on browser storageState projects; they only
 * need the resulting profile UUID and site credentials for authenticate.
 */
export async function seedProtocolFixtures(
	request: APIRequestContext,
): Promise<SeedResult> {
	if (seeded) {
		return seeded;
	}

	// 1. Setup when the instance is fresh.
	const check = await request.get("/api/v1/auth/check");
	const checkBody = (await check.json()) as {
		data?: { initialized?: boolean };
	};
	if (!checkBody.data?.initialized) {
		// The public origin is required: Yggdrasil texture properties embed
		// absolute texture URLs built from it.
		const setupResponse = await request.post("/api/v1/auth/setup", {
			data: {
				email: ADMIN.email,
				password: ADMIN.password,
				public_site_url: protocolBaseURL(),
				username: ADMIN.username,
			},
		});
		expect(setupResponse.status(), "instance setup").toBe(200);
	}

	// The Yggdrasil signature keypair is explicit runtime config. Rotate it
	// through the product's own admin action so signed texture properties and
	// the metadata public key are real, exactly like operator setup would do.
	const rotateResponse = await request.post("/api/v1/auth/login", {
		data: { identifier: ADMIN.username, password: ADMIN.password },
	});
	expect(rotateResponse.status(), "admin login").toBe(200);
	const rotateAction = await request.post(
		"/api/v1/admin/config/yggdrasil/action",
		{
			headers: await csrfHeadersFor(request),
			data: { action: "rotate_yggdrasil_signature_key" },
		},
	);
	expect(rotateAction.status(), "signature key rotation").toBe(200);

	// 2. Admin session: create the player when it does not exist yet.
	const baseURL = protocolBaseURL();
	const admin = await playwrightRequest.newContext({ baseURL });
	const player = await playwrightRequest.newContext({ baseURL });
	try {
		const loginResponse = await admin.post("/api/v1/auth/login", {
			data: { identifier: ADMIN.username, password: ADMIN.password },
		});
		expect(loginResponse.status(), "admin login").toBe(200);

		const search = await admin.get(
			`/api/v1/admin/users?keyword=${encodeURIComponent(PLAYER.username)}`,
		);
		const searchBody = (await search.json()) as {
			data?: { items?: Array<{ id: number; username: string }> };
		};
		if (!searchBody.data?.items?.length) {
			const createResponse = await admin.post("/api/v1/admin/users", {
				headers: await csrfHeadersFor(admin),
				data: {
					email: PLAYER.email,
					must_change_password: false,
					password: PLAYER.password,
					role: "user",
					status: "active",
					username: PLAYER.username,
				},
			});
			expect(
				[200, 201].includes(createResponse.status()),
				"player creation",
			).toBe(true);
		}

		const playerLogin = await player.post("/api/v1/auth/login", {
			data: { identifier: PLAYER.username, password: PLAYER.password },
		});
		expect(playerLogin.status(), "player login").toBe(200);

		// 3. Profile + skin so protocol endpoints serve real data. The create
		// endpoint returns the Yggdrasil profile summary, whose `id` is the
		// unsigned (dash-less) profile UUID used across protocol endpoints.
		const profileName = uniqueProfileName();
		const profileResponse = await player.post("/api/v1/profiles/minecraft", {
			headers: await csrfHeadersFor(player),
			data: { name: profileName },
		});
		const profileBody = (await profileResponse.json()) as {
			data?: { id?: string; name?: string };
		};
		expect(profileResponse.status(), "profile creation").toBe(200);
		const profileUuid = profileBody.data?.id;
		expect(profileUuid, "profile uuid").toBeTruthy();

		const uploadResponse = await player.post("/api/v1/wardrobe/textures/skin", {
			headers: await csrfHeadersFor(player),
			multipart: {
				file: {
					buffer: createSkinPng(),
					mimeType: "image/png",
					name: "protocol-seed.png",
				},
			},
		});
		const uploadBody = (await uploadResponse.json()) as {
			data?: { id?: number };
		};
		expect(uploadResponse.status(), "wardrobe upload").toBe(200);
		const textureId = uploadBody.data?.id;
		expect(textureId, "texture id").toBeTruthy();

		const bindResponse = await player.put(
			`/api/v1/profiles/minecraft/${profileUuid}/textures/skin`,
			{
				headers: await csrfHeadersFor(player),
				data: { texture_id: textureId },
			},
		);
		expect(bindResponse.status(), "texture binding").toBe(200);

		seeded = { profileName, profileUuid: profileUuid as string };
	} finally {
		await admin.dispose();
		await player.dispose();
	}

	return seeded;
}
