import { type APIRequestContext, expect, test } from "@playwright/test";
import { PLAYER } from "../support/fixtures";
import { seedProtocolFixtures, YGGDRASIL_ROOT } from "./support/seed";

interface AuthenticateResponse {
	accessToken: string;
	clientToken: string;
	availableProfiles: Array<{ id: string; name: string }>;
	selectedProfile?: { id: string; name: string };
	user?: { id: string; properties: unknown[] };
}

interface ProtocolError {
	error: string;
	errorMessage: string;
	cause?: string;
}

async function authenticate(
	request: APIRequestContext,
	body: Record<string, unknown>,
) {
	return request.post(`${YGGDRASIL_ROOT}/authserver/authenticate`, {
		data: body,
	});
}

test.describe("authserver token lifecycle", () => {
	test.beforeAll(async ({ request }) => {
		await seedProtocolFixtures(request);
	});

	test("authenticates with username and password and echoes the client token", async ({
		request,
	}) => {
		const clientToken = "e2e-client-token-0001";
		const response = await authenticate(request, {
			agent: { name: "Minecraft", version: 1 },
			username: PLAYER.username,
			password: PLAYER.password,
			clientToken,
			requestUser: true,
		});
		expect(response.status()).toBe(200);

		const body = (await response.json()) as AuthenticateResponse;
		// The client token is echoed verbatim; a fresh access token is issued.
		expect(body.clientToken).toBe(clientToken);
		expect(body.accessToken.length).toBeGreaterThan(16);
		expect(body.availableProfiles.length).toBeGreaterThanOrEqual(1);
		for (const profile of body.availableProfiles) {
			// Unsigned lowercase 32-hex UUID.
			expect(profile.id).toMatch(/^[0-9a-f]{32}$/);
			expect(profile.name.length).toBeGreaterThanOrEqual(3);
		}
		// A single-profile account gets the profile pre-selected.
		expect(body.selectedProfile).toBeTruthy();
		expect(body.selectedProfile?.id).toMatch(/^[0-9a-f]{32}$/);
		// requestUser=true returns the account with an unhyphenated UUID.
		expect(body.user?.id).toMatch(/^[0-9a-f]{32}$/);
		expect(Array.isArray(body.user?.properties)).toBe(true);
	});

	test("rejects wrong credentials with the protocol error shape", async ({
		request,
	}) => {
		const response = await authenticate(request, {
			agent: { name: "Minecraft", version: 1 },
			username: PLAYER.username,
			password: "not-the-password",
		});
		expect(response.status()).toBe(403);
		const body = (await response.json()) as ProtocolError;
		expect(body.error).toBe("ForbiddenOperationException");
		expect(body.errorMessage).toBe(
			"Invalid credentials. Invalid username or password.",
		);
	});

	test("rejects unsupported agents with a 400 IllegalArgumentException", async ({
		request,
	}) => {
		const response = await authenticate(request, {
			agent: { name: "Scrolls", version: 1 },
			username: PLAYER.username,
			password: PLAYER.password,
		});
		expect(response.status()).toBe(400);
		const body = (await response.json()) as ProtocolError;
		expect(body.error).toBe("IllegalArgumentException");
		// The exact message is a host-provided detail; only the error name is
		// a protocol contract.
		expect(body.errorMessage.toLowerCase()).toContain("agent");
	});

	test("validates and refreshes tokens, rotating the access token", async ({
		request,
	}) => {
		const login = await authenticate(request, {
			username: PLAYER.username,
			password: PLAYER.password,
		});
		const { accessToken, clientToken } =
			(await login.json()) as AuthenticateResponse;

		// A valid token answers 204 with no body.
		const valid = await request.post(`${YGGDRASIL_ROOT}/authserver/validate`, {
			data: { accessToken, clientToken },
		});
		expect(valid.status()).toBe(204);
		expect((await valid.text()).length).toBe(0);

		// A client-token mismatch is a hard 403, not a silent pass.
		const mismatch = await request.post(
			`${YGGDRASIL_ROOT}/authserver/validate`,
			{ data: { accessToken, clientToken: "wrong-client-token" } },
		);
		expect(mismatch.status()).toBe(403);
		expect(((await mismatch.json()) as ProtocolError).error).toBe(
			"ForbiddenOperationException",
		);

		// Refresh rotates the access token and keeps the client token.
		const refresh = await request.post(`${YGGDRASIL_ROOT}/authserver/refresh`, {
			data: { accessToken, clientToken },
		});
		expect(refresh.status()).toBe(200);
		const refreshed = (await refresh.json()) as AuthenticateResponse;
		expect(refreshed.clientToken).toBe(clientToken);
		expect(refreshed.accessToken).not.toBe(accessToken);
		expect(refreshed.selectedProfile).toBeTruthy();

		// The old token no longer validates; the new one does.
		const oldToken = await request.post(
			`${YGGDRASIL_ROOT}/authserver/validate`,
			{ data: { accessToken } },
		);
		expect(oldToken.status()).toBe(403);
		const newToken = await request.post(
			`${YGGDRASIL_ROOT}/authserver/validate`,
			{ data: { accessToken: refreshed.accessToken } },
		);
		expect(newToken.status()).toBe(204);
	});

	test("invalidates a token even when re-invalidated", async ({ request }) => {
		const login = await authenticate(request, {
			username: PLAYER.username,
			password: PLAYER.password,
		});
		const { accessToken } = (await login.json()) as AuthenticateResponse;

		const first = await request.post(
			`${YGGDRASIL_ROOT}/authserver/invalidate`,
			{
				data: { accessToken },
			},
		);
		// Invalidation succeeds regardless of prior state (idempotent-by-design).
		expect(first.status()).toBe(204);
		const second = await request.post(
			`${YGGDRASIL_ROOT}/authserver/invalidate`,
			{
				data: { accessToken },
			},
		);
		expect(second.status()).toBe(204);

		const valid = await request.post(`${YGGDRASIL_ROOT}/authserver/validate`, {
			data: { accessToken },
		});
		expect(valid.status()).toBe(403);
	});

	test("signout with password revokes every token for the account", async ({
		request,
	}) => {
		const first = await authenticate(request, {
			username: PLAYER.username,
			password: PLAYER.password,
		});
		const second = await authenticate(request, {
			username: PLAYER.username,
			password: PLAYER.password,
		});
		const firstToken = (await first.json()) as AuthenticateResponse;
		const secondToken = (await second.json()) as AuthenticateResponse;

		const wrongPassword = await request.post(
			`${YGGDRASIL_ROOT}/authserver/signout`,
			{
				data: {
					username: PLAYER.username,
					password: "not-the-password",
				},
			},
		);
		expect(wrongPassword.status()).toBe(403);

		const signout = await request.post(`${YGGDRASIL_ROOT}/authserver/signout`, {
			data: {
				username: PLAYER.username,
				password: PLAYER.password,
			},
		});
		expect(signout.status()).toBe(204);

		for (const token of [firstToken.accessToken, secondToken.accessToken]) {
			const valid = await request.post(
				`${YGGDRASIL_ROOT}/authserver/validate`,
				{ data: { accessToken: token } },
			);
			expect(valid.status()).toBe(403);
		}
	});
});
