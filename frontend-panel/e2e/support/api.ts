import {
	type APIRequestContext,
	expect,
	request as playwrightRequest,
} from "@playwright/test";
import type {
	AdminUserInfo,
	CheckResp,
	MinecraftWardrobeTextureMetadata,
} from "@/types/api";
import { E2E_API_SUCCESS_CODE, type E2eApiResponse } from "./api-response";
import { ADMIN } from "./fixtures";

export const CSRF_COOKIE_NAME = "aster_csrf";
export const CSRF_HEADER_NAME = "X-CSRF-Token";
export const ACCESS_COOKIE_NAME = "aster_access";
export const REFRESH_COOKIE_NAME = "aster_refresh";

type EnvelopeResult<T> = { status: number; payload: E2eApiResponse<T> };

/**
 * Extracts the CSRF double-submit token from the request context's cookie jar
 * so unsafe API calls can carry the `X-CSRF-Token` header the backend requires
 * whenever an access cookie is present.
 */
export async function csrfHeaders(
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

async function parseEnvelope<T>(
	response: Awaited<ReturnType<APIRequestContext["get"]>>,
): Promise<EnvelopeResult<T>> {
	const text = await response.text();
	const payload = JSON.parse(text) as E2eApiResponse<T>;
	return { status: response.status(), payload };
}

/** Performs an authenticated unsafe request, injecting the CSRF header. */
async function apiUnsafe<T>(
	request: APIRequestContext,
	method: "delete" | "patch" | "post" | "put",
	url: string,
	body?: unknown,
): Promise<EnvelopeResult<T>> {
	const response = await request[method](url, {
		headers: await csrfHeaders(request),
		...(body === undefined ? {} : { data: body }),
	});
	return parseEnvelope<T>(response);
}

export async function apiGet<T>(
	request: APIRequestContext,
	url: string,
): Promise<EnvelopeResult<T>> {
	return parseEnvelope<T>(await request.get(url));
}

export function apiPost<T>(
	request: APIRequestContext,
	url: string,
	body?: unknown,
) {
	return apiUnsafe<T>(request, "post", url, body);
}

export function apiPut<T>(
	request: APIRequestContext,
	url: string,
	body?: unknown,
) {
	return apiUnsafe<T>(request, "put", url, body);
}

export function apiPatch<T>(
	request: APIRequestContext,
	url: string,
	body?: unknown,
) {
	return apiUnsafe<T>(request, "patch", url, body);
}

export function apiDelete<T>(
	request: APIRequestContext,
	url: string,
): Promise<EnvelopeResult<T>> {
	return apiUnsafe<T>(request, "delete", url);
}

/** Asserts an envelope response succeeded and returns `data`. */
export function expectOk<T>(
	result: EnvelopeResult<T>,
	expectedStatus = 200,
): T {
	expect(
		result.status,
		(result.payload.msg as string) ?? "API call status",
	).toBe(expectedStatus);
	expect(result.payload.code).toBe(E2E_API_SUCCESS_CODE);
	return result.payload.data as T;
}

/**
 * Logs in through the API (no browser). The request context keeps the session
 * cookies for later `storageState()` reuse or direct API calls.
 */
export async function loginViaApi(
	request: APIRequestContext,
	identifier: string,
	password: string,
) {
	const response = await request.post("/api/v1/auth/login", {
		data: { identifier, password },
	});
	const { status, payload } = await parseEnvelope<unknown>(response);
	expect(status, payload.msg ?? "login status").toBe(200);
	expect(payload.code).toBe(E2E_API_SUCCESS_CODE);
	return payload;
}

/**
 * Opens an admin-scoped API session (fresh request context + API login) for
 * specs whose own `request` fixture intentionally carries no storageState.
 */
export async function adminApiSession(baseURL: string) {
	const context = await playwrightRequest.newContext({ baseURL });
	await loginViaApi(context, ADMIN.username, ADMIN.password);
	return context;
}

/**
 * Opens a bare API session sharing nothing with the spec's own context.
 * Returns the context with `baseURL` preset so relative URLs work.
 */
export async function apiSession(
	baseURL: string,
	options?: { storageState?: string },
) {
	return playwrightRequest.newContext({
		baseURL,
		...(options?.storageState ? { storageState: options.storageState } : {}),
	});
}

/** Persists the request context's session cookies as a Playwright storageState. */
export async function saveStorageState(
	request: APIRequestContext,
	file: string,
) {
	await request.storageState({ path: file });
}

/** Reads the instance initialization flag (`GET /api/v1/auth/check`). */
export async function hasUsers(request: APIRequestContext) {
	const result = await apiGet<CheckResp>(request, "/api/v1/auth/check");
	return result.payload.data?.initialized ?? false;
}

export interface SeedOutcome {
	initializedNow: boolean;
}

/**
 * Ensures the instance is initialized by creating the first admin account via
 * the setup endpoint. Safe to call from multiple places: the endpoint rejects
 * re-setup once any account exists and that rejection is tolerated here.
 *
 * `publicSiteUrl` is stored as the instance's public origin; the Yggdrasil
 * protocol needs it to build texture URLs, so suites pass their own base URL.
 */
export async function ensureInitializedViaApi(
	request: APIRequestContext,
	publicSiteUrl?: string,
): Promise<SeedOutcome> {
	if (await hasUsers(request)) {
		return { initializedNow: false };
	}
	const response = await request.post("/api/v1/auth/setup", {
		data: {
			email: ADMIN.email,
			password: ADMIN.password,
			...(publicSiteUrl ? { public_site_url: publicSiteUrl } : {}),
			username: ADMIN.username,
		},
	});
	if (response.status() === 200) {
		return { initializedNow: true };
	}
	// Another setup already completed.
	expect(response.status()).toBe(400);
	return { initializedNow: false };
}

export interface SeedAccountSpec {
	email: string;
	mustChangePassword?: boolean;
	operatorScopes?: string[];
	password?: string;
	role?: "admin" | "operator" | "user";
	username: string;
}

/**
 * Creates a site account as the admin. When `password` is omitted the backend
 * generates a temporary password; the response carries it and it is returned
 * so specs can sign that account in.
 */
export async function createUserViaApi(
	adminRequest: APIRequestContext,
	spec: SeedAccountSpec,
) {
	const data = expectOk<AdminUserInfo>(
		await apiPost<AdminUserInfo>(adminRequest, "/api/v1/admin/users", {
			email: spec.email,
			must_change_password: spec.mustChangePassword ?? false,
			operator_scopes: spec.operatorScopes ?? null,
			password: spec.password ?? null,
			role: spec.role ?? "user",
			status: "active",
			username: spec.username,
		}),
		201,
	);
	return { user: data, generatedPassword: null as string | null };
}

/** Finds an existing account by username, or null when it does not exist. */
export async function findUserByUsername(
	adminRequest: APIRequestContext,
	username: string,
) {
	const result = await apiGet<{ items: AdminUserInfo[] }>(
		adminRequest,
		`/api/v1/admin/users?keyword=${encodeURIComponent(username)}`,
	);
	const data = expectOk<{ items: AdminUserInfo[] }>(result);
	return data.items.find((item) => item.username === username) ?? null;
}

/**
 * Ensures a stable account exists (used by the setup project so every run has
 * the same well-known player/operator identities for storageState reuse).
 */
export async function ensureAccountViaApi(
	adminRequest: APIRequestContext,
	spec: SeedAccountSpec,
) {
	const existing = await findUserByUsername(adminRequest, spec.username);
	if (existing) {
		return existing;
	}
	return (await createUserViaApi(adminRequest, spec)).user;
}

export interface CreatedMinecraftProfile {
	/** Unsigned (dash-less) profile UUID, as in the Yggdrasil wire model. */
	id: string;
	name: string;
}

/** Creates a Minecraft profile for the authenticated user. */
export async function createMinecraftProfileViaApi(
	request: APIRequestContext,
	name: string,
): Promise<CreatedMinecraftProfile> {
	const result = await apiPost<CreatedMinecraftProfile>(
		request,
		"/api/v1/profiles/minecraft",
		{ name },
	);
	return expectOk(result);
}

/** Uploads a PNG into the authenticated user's wardrobe. */
export async function uploadWardrobeTextureViaApi(
	request: APIRequestContext,
	textureType: "cape" | "skin",
	png: Buffer,
	options?: { displayName?: string; visibility?: "private" | "public" },
) {
	const fields: Record<
		string,
		string | { name: string; mimeType: string; buffer: Buffer }
	> = {
		file: {
			buffer: png,
			mimeType: "image/png",
			name: "e2e-texture.png",
		},
	};
	if (options?.displayName) {
		fields.name = options.displayName;
	}
	if (options?.visibility) {
		fields.visibility = options.visibility;
	}
	const response = await request.post(
		`/api/v1/wardrobe/textures/${textureType}`,
		{
			headers: await csrfHeaders(request),
			multipart: fields,
		},
	);
	const result =
		await parseEnvelope<MinecraftWardrobeTextureMetadata>(response);
	expect(result.status, result.payload.msg ?? "wardrobe upload").toBe(200);
	expect(result.payload.code).toBe(E2E_API_SUCCESS_CODE);
	return result.payload.data as MinecraftWardrobeTextureMetadata;
}

/** Binds a wardrobe texture to a profile slot on behalf of the owner. */
export async function bindProfileTextureViaApi(
	request: APIRequestContext,
	profileUuid: string,
	textureType: "cape" | "skin",
	textureId: number,
) {
	await expectOk(
		await apiPut(
			request,
			`/api/v1/profiles/minecraft/${profileUuid}/textures/${textureType}`,
			{ texture_id: textureId },
		),
	);
}

/** Submits one of the authenticated user's wardrobe textures for review. */
export async function submitLibraryReviewViaApi(
	request: APIRequestContext,
	textureId: number,
) {
	await expectOk(
		await apiPost(
			request,
			`/api/v1/wardrobe/textures/${textureId}/library-submission`,
		),
	);
}

/** Revokes every active site session owned by the given user id. */
export async function revokeUserSessionsViaApi(
	adminRequest: APIRequestContext,
	userId: number,
) {
	const response = await adminRequest.post(
		`/api/v1/admin/users/${userId}/sessions/revoke`,
		{ headers: await csrfHeaders(adminRequest) },
	);
	expect(response.status(), (await response.text()).slice(0, 200)).toBe(200);
}

/** Patches a site account (role, scopes, status, ...) as the admin. */
export async function updateUserViaApi(
	adminRequest: APIRequestContext,
	userId: number,
	patch: {
		email?: string;
		must_change_password?: boolean;
		operator_scopes?: string[] | null;
		password?: string | null;
		role?: "admin" | "operator" | "user";
		status?: "active" | "disabled";
		username?: string;
	},
) {
	return expectOk<AdminUserInfo>(
		await apiPatch<AdminUserInfo>(
			adminRequest,
			`/api/v1/admin/users/${userId}`,
			patch,
		),
	);
}

/** Deletes a site account as the admin. */
export async function deleteUserViaApi(
	adminRequest: APIRequestContext,
	userId: number,
) {
	await expectOk(
		await apiDelete(adminRequest, `/api/v1/admin/users/${userId}`),
	);
}

/**
 * Logs a set of accounts in through the API, one request context per account,
 * and persists each session as a storageState file for browser reuse.
 */
export async function seedAccountStates(
	baseURL: string,
	states: Array<{
		identifier: string;
		password: string;
		path: string;
	}>,
) {
	for (const state of states) {
		const context = await playwrightRequest.newContext({ baseURL });
		try {
			await loginViaApi(context, state.identifier, state.password);
			await saveStorageState(context, state.path);
		} finally {
			await context.dispose();
		}
	}
}
