import { beforeEach, describe, expect, it, vi } from "vitest";

const authServiceMock = vi.hoisted(() => ({
	me: vi.fn(),
	login: vi.fn(),
	register: vi.fn(),
	setup: vi.fn(),
	finishPasskeyLogin: vi.fn(),
	refresh: vi.fn(),
	logout: vi.fn(),
	updateProfile: vi.fn(),
	setAvatarSource: vi.fn(),
	uploadAvatar: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

const fullUser = {
	id: 7,
	username: "steve",
	email: "steve@example.com",
	role: "admin",
	status: "active",
	profile: {
		display_name: null,
		avatar: {
			source: "none",
			url_512: null,
			url_1024: null,
			version: 0,
		},
	},
} as const;

async function loadStore() {
	vi.resetModules();
	return await import("@/stores/authStore");
}

describe("authStore persistence", () => {
	beforeEach(() => {
		localStorage.clear();
		sessionStorage.clear();
		authServiceMock.me.mockReset();
		authServiceMock.login.mockReset();
		authServiceMock.register.mockReset();
		authServiceMock.setup.mockReset();
		authServiceMock.finishPasskeyLogin.mockReset();
		authServiceMock.refresh.mockReset();
		authServiceMock.logout.mockReset();
		authServiceMock.updateProfile.mockReset();
		authServiceMock.setAvatarSource.mockReset();
		authServiceMock.uploadAvatar.mockReset();
		authServiceMock.me.mockResolvedValue(fullUser);
		authServiceMock.login.mockResolvedValue({ expires_in: 3600 });
		authServiceMock.register.mockResolvedValue({ expires_in: 3600 });
		authServiceMock.setup.mockResolvedValue({ expires_in: 3600 });
		authServiceMock.finishPasskeyLogin.mockResolvedValue({ expires_in: 3600 });
		authServiceMock.refresh.mockResolvedValue({ expires_in: 1800 });
		authServiceMock.logout.mockResolvedValue(undefined);
		authServiceMock.updateProfile.mockResolvedValue(fullUser.profile);
		authServiceMock.setAvatarSource.mockResolvedValue(fullUser.profile);
		authServiceMock.uploadAvatar.mockResolvedValue(fullUser.profile);
	});

	it("persists only non-sensitive display fields after login", async () => {
		const { useAuthStore } = await loadStore();

		await useAuthStore.getState().login("steve", "password");

		const raw = localStorage.getItem("asteryggdrasil-cached-user");
		expect(raw).not.toBeNull();
		expect(JSON.parse(raw ?? "{}")).toEqual({
			id: 7,
			username: "steve",
			role: "admin",
			status: "active",
			profile: {
				display_name: null,
				avatar: {
					source: "none",
					url_512: null,
					url_1024: null,
					version: 0,
				},
			},
		});
		expect(raw).not.toContain("access");
		expect(raw).not.toContain("refresh");
		expect(sessionStorage.getItem("asteryggdrasil-auth-expires-at")).toMatch(
			/^\d+$/,
		);
	});

	it("migrates legacy cached users by stripping sensitive fields", async () => {
		localStorage.setItem(
			"asteryggdrasil-user",
			JSON.stringify({
				...fullUser,
				access_token: "plain-access-token",
				refresh_token: "plain-refresh-token",
			}),
		);

		const { useAuthStore } = await loadStore();

		expect(useAuthStore.getState().user).toMatchObject({
			id: 7,
			username: "steve",
			role: "admin",
			status: "active",
			email: "",
			profile: {
				display_name: null,
				avatar: {
					source: "none",
					url_512: null,
					url_1024: null,
					version: 0,
				},
			},
		});
		expect(localStorage.getItem("asteryggdrasil-user")).toBeNull();
		const raw = localStorage.getItem("asteryggdrasil-cached-user");
		expect(raw).not.toContain("plain-access-token");
		expect(raw).not.toContain("plain-refresh-token");
	});

	it("drops malformed cached users during startup", async () => {
		localStorage.setItem(
			"asteryggdrasil-cached-user",
			JSON.stringify({
				id: "not-a-number",
				username: "steve",
				role: "admin",
				status: "active",
			}),
		);

		const { useAuthStore } = await loadStore();

		expect(useAuthStore.getState().user).toBeNull();
		expect(localStorage.getItem("asteryggdrasil-cached-user")).toBeNull();
	});

	it("drops expired session expiry values during startup", async () => {
		sessionStorage.setItem("asteryggdrasil-auth-expires-at", "1");

		const { useAuthStore } = await loadStore();

		expect(useAuthStore.getState().expiresAt).toBeNull();
		expect(sessionStorage.getItem("asteryggdrasil-auth-expires-at")).toBeNull();
	});

	it("starts without cached auth when storage reads fail", async () => {
		const getItemSpy = vi
			.spyOn(Storage.prototype, "getItem")
			.mockImplementation(() => {
				throw new Error("storage reads blocked");
			});

		try {
			const { useAuthStore } = await loadStore();

			expect(useAuthStore.getState().user).toBeNull();
			expect(useAuthStore.getState().expiresAt).toBeNull();
		} finally {
			getItemSpy.mockRestore();
		}
	});

	it("keeps the in-memory session usable when storage writes fail", async () => {
		const setItemSpy = vi
			.spyOn(Storage.prototype, "setItem")
			.mockImplementation(() => {
				throw new Error("storage writes blocked");
			});
		const removeItemSpy = vi
			.spyOn(Storage.prototype, "removeItem")
			.mockImplementation(() => {
				throw new Error("storage removes blocked");
			});

		try {
			const { useAuthStore } = await loadStore();

			await useAuthStore.getState().login("steve", "password");

			expect(useAuthStore.getState().user).toEqual(fullUser);
			expect(useAuthStore.getState().isAuthenticated).toBe(true);
			expect(useAuthStore.getState().expiresAt).toEqual(expect.any(Number));
		} finally {
			setItemSpy.mockRestore();
			removeItemSpy.mockRestore();
		}
	});

	it("keeps cached auth stale when session hydration hits a network error", async () => {
		localStorage.setItem(
			"asteryggdrasil-cached-user",
			JSON.stringify({
				id: 7,
				username: "steve",
				role: "admin",
				status: "active",
				profile: fullUser.profile,
			}),
		);
		const { useAuthStore } = await loadStore();
		const { ApiError } = await import("@/services/http");
		authServiceMock.me.mockRejectedValue(
			new ApiError("network_error", "Network error", { retryable: true }),
		);

		await useAuthStore.getState().hydrate();

		expect(useAuthStore.getState()).toMatchObject({
			checking: false,
			error: "Network error",
			errorCode: "network_error",
			isAdmin: true,
			isAuthenticated: true,
			isAuthStale: true,
			user: {
				id: 7,
				username: "steve",
			},
		});
		expect(localStorage.getItem("asteryggdrasil-cached-user")).toContain(
			"steve",
		);
	});

	it("keeps network errors distinct from unauthenticated state without cached auth", async () => {
		const { useAuthStore } = await loadStore();
		const { ApiError } = await import("@/services/http");
		authServiceMock.me.mockRejectedValue(
			new ApiError("network_error", "Network error", { retryable: true }),
		);

		await useAuthStore.getState().hydrate();

		expect(useAuthStore.getState()).toMatchObject({
			checking: false,
			error: "Network error",
			errorCode: "network_error",
			isAuthenticated: false,
			isAuthStale: true,
			user: null,
		});
	});

	it("clears cached auth when session hydration returns a real auth failure", async () => {
		localStorage.setItem(
			"asteryggdrasil-cached-user",
			JSON.stringify({
				id: 7,
				username: "steve",
				role: "admin",
				status: "active",
				profile: fullUser.profile,
			}),
		);
		const { useAuthStore } = await loadStore();
		const { ApiError } = await import("@/services/http");
		authServiceMock.me.mockRejectedValue(
			new ApiError("auth.token_invalid", "token invalid"),
		);

		await useAuthStore.getState().hydrate();

		expect(useAuthStore.getState()).toMatchObject({
			checking: false,
			error: "token invalid",
			errorCode: "auth.token_invalid",
			isAuthenticated: false,
			isAuthStale: false,
			user: null,
		});
		expect(localStorage.getItem("asteryggdrasil-cached-user")).toBeNull();
	});

	it("clears cached user, expiry, and legacy token keys on logout", async () => {
		localStorage.setItem("asteryggdrasil-access-token", "legacy-access");
		localStorage.setItem("asteryggdrasil-refresh-token", "legacy-refresh");
		const { useAuthStore } = await loadStore();

		await useAuthStore.getState().login("steve", "password");
		await useAuthStore.getState().logout();

		expect(localStorage.getItem("asteryggdrasil-cached-user")).toBeNull();
		expect(localStorage.getItem("asteryggdrasil-access-token")).toBeNull();
		expect(localStorage.getItem("asteryggdrasil-refresh-token")).toBeNull();
		expect(sessionStorage.getItem("asteryggdrasil-auth-expires-at")).toBeNull();
		expect(useAuthStore.getState().isAuthenticated).toBe(false);
	});

	it("persists the authenticated user after passkey login", async () => {
		const { useAuthStore } = await loadStore();
		const credential = { id: "credential-1" };

		await useAuthStore.getState().loginWithPasskey("flow-1", credential);

		expect(authServiceMock.finishPasskeyLogin).toHaveBeenCalledWith(
			"flow-1",
			credential,
		);
		expect(authServiceMock.me).toHaveBeenCalled();
		expect(useAuthStore.getState().user).toEqual(fullUser);
		expect(useAuthStore.getState().isAuthenticated).toBe(true);
		expect(localStorage.getItem("asteryggdrasil-cached-user")).toContain(
			"steve",
		);
		expect(sessionStorage.getItem("asteryggdrasil-auth-expires-at")).toMatch(
			/^\d+$/,
		);
	});

	it("updates the current user profile without reloading /auth/me", async () => {
		const { useAuthStore } = await loadStore();
		const nextProfile = {
			display_name: "Builder Steve",
			avatar: {
				source: "upload",
				url_512: "/auth/profile/avatar/512?v=4",
				url_1024: "/auth/profile/avatar/1024?v=4",
				version: 4,
			},
		} as const;
		authServiceMock.updateProfile.mockResolvedValue(nextProfile);

		await useAuthStore.getState().login("steve", "password");
		authServiceMock.me.mockClear();
		await useAuthStore.getState().updateProfile({
			display_name: "Builder Steve",
		});

		expect(authServiceMock.updateProfile).toHaveBeenCalledWith({
			display_name: "Builder Steve",
		});
		expect(authServiceMock.me).not.toHaveBeenCalled();
		expect(useAuthStore.getState().user?.profile).toEqual(nextProfile);
		expect(localStorage.getItem("asteryggdrasil-cached-user")).toContain(
			"Builder Steve",
		);
		expect(localStorage.getItem("asteryggdrasil-cached-user")).toContain(
			"/auth/profile/avatar/512?v=4",
		);
	});

	it("syncs current account fields after an admin self-update", async () => {
		const { useAuthStore } = await loadStore();

		await useAuthStore.getState().login("steve", "password");
		useAuthStore.getState().syncCurrentUserFromAdminUser({
			active_session_count: 2,
			created_at: "2026-06-15T00:00:00.000Z",
			email: "builder@example.com",
			email_verified_at: null,
			id: fullUser.id,
			profile: {
				display_name: "Builder",
				avatar: {
					source: "upload",
					url_512: "/auth/profile/avatar/512?v=5",
					url_1024: "/auth/profile/avatar/1024?v=5",
					version: 5,
				},
			},
			profile_count: 1,
			role: "admin",
			session_version: 3,
			status: "active",
			updated_at: "2026-06-15T00:01:00.000Z",
			username: "builder",
		});

		expect(useAuthStore.getState().user).toMatchObject({
			id: fullUser.id,
			email: "builder@example.com",
			username: "builder",
			profile: {
				display_name: "Builder",
				avatar: {
					url_512: "/auth/profile/avatar/512?v=5",
				},
			},
		});
		expect(localStorage.getItem("asteryggdrasil-cached-user")).toContain(
			"builder",
		);
	});

	it("ignores admin updates for other users", async () => {
		const { useAuthStore } = await loadStore();

		await useAuthStore.getState().login("steve", "password");
		useAuthStore.getState().syncCurrentUserFromAdminUser({
			active_session_count: 0,
			created_at: "2026-06-15T00:00:00.000Z",
			email: "other@example.com",
			email_verified_at: null,
			id: 99,
			profile: fullUser.profile,
			profile_count: 0,
			role: "user",
			session_version: 1,
			status: "active",
			updated_at: "2026-06-15T00:01:00.000Z",
			username: "other",
		});

		expect(useAuthStore.getState().user).toEqual(fullUser);
	});
});
