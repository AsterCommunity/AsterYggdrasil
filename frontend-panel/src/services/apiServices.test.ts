import { beforeEach, describe, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => {
	const get = vi.fn();
	const post = vi.fn();
	const put = vi.fn();
	const patch = vi.fn();
	const deleteRequest = vi.fn();
	const rootClientGet = vi.fn();
	const rootClientRequest = vi.fn();
	const rootGet = vi.fn();

	return {
		deleteRequest,
		get,
		patch,
		post,
		put,
		rootClientGet,
		rootClientRequest,
		rootGet,
	};
});

vi.mock("./http", async () => {
	const actual = await vi.importActual<typeof import("./http")>("./http");
	return {
		...actual,
		api: {
			delete: apiMock.deleteRequest,
			get: apiMock.get,
			patch: apiMock.patch,
			post: apiMock.post,
			put: apiMock.put,
			rootClient: {
				get: apiMock.rootClientGet,
				request: apiMock.rootClientRequest,
			},
			root: {
				get: apiMock.rootGet,
			},
		},
	};
});

beforeEach(() => {
	vi.clearAllMocks();
	vi.resetModules();
});

function authUser(overrides: Partial<import("@/types/api").AuthUserInfo> = {}) {
	return {
		email: "steve@example.com",
		id: 1,
		profile: {
			avatar: {
				source: "none",
				url_1024: null,
				url_512: null,
				version: 0,
			},
			display_name: "Steve",
		},
		role: "user",
		status: "active",
		username: "steve",
		...overrides,
	} satisfies import("@/types/api").AuthUserInfo;
}

function authSession(
	overrides: Partial<import("@/types/api").AuthSessionInfo> = {},
) {
	return {
		created_at: "2026-06-15T00:00:00Z",
		id: "session-1",
		ip_address: "127.0.0.1",
		is_current: true,
		last_seen_at: "2026-06-15T00:00:00Z",
		refresh_expires_at: "2026-06-16T00:00:00Z",
		revoked: false,
		user_agent: "Chrome",
		...overrides,
	} satisfies import("@/types/api").AuthSessionInfo;
}

function passkey(overrides: Partial<import("./authService").PasskeyInfo> = {}) {
	return {
		backed_up: false,
		backup_eligible: true,
		created_at: "2026-06-15T00:00:00Z",
		id: 7,
		last_used_at: null,
		name: "MacBook",
		sign_count: 0,
		transports: ["internal"],
		updated_at: "2026-06-15T00:00:00Z",
		...overrides,
	} satisfies import("./authService").PasskeyInfo;
}

type OffsetPageFixture<T> = {
	items: T[];
	limit: number;
	offset: number;
	total: number;
};

function offsetPage<T>(
	items: T[],
	limit = 50,
	offset = 0,
	total = items.length,
): OffsetPageFixture<T> {
	return { items, limit, offset, total };
}

function externalAuthProvider(
	overrides: Partial<import("@/types/api").ExternalAuthPublicProvider> = {},
) {
	return {
		display_name: "GitHub",
		icon_url: "/static/external-auth/github.svg",
		key: "github",
		kind: "github",
		...overrides,
	} satisfies import("@/types/api").ExternalAuthPublicProvider;
}

function externalAuthLink(
	overrides: Partial<import("@/types/api").ExternalAuthLinkInfo> = {},
) {
	return {
		created_at: "2026-06-15T00:00:00Z",
		display_name_snapshot: "Steve",
		email_snapshot: "steve@example.com",
		id: 11,
		issuer: "https://github.com",
		last_login_at: null,
		provider_display_name: "GitHub",
		provider_icon_url: "/static/external-auth/github.svg",
		provider_id: 3,
		provider_kind: "github",
		provider_key: "github",
		subject: "subject-1",
		updated_at: "2026-06-15T00:00:00Z",
		...overrides,
	} satisfies import("@/types/api").ExternalAuthLinkInfo;
}

describe("authService", () => {
	it("sends typed auth request bodies through the shared API client", async () => {
		apiMock.post.mockResolvedValue({ expires_in: 3600 });
		const { authService } = await import("./authService");

		await authService.login({ identifier: "cat", password: "secret" });

		expect(apiMock.post).toHaveBeenCalledWith("/auth/login", {
			identifier: "cat",
			password: "secret",
		});
	});

	it("uses generated response shapes for session revocation counts", async () => {
		apiMock.deleteRequest.mockResolvedValue({ removed: 2 });
		const { authService } = await import("./authService");

		await expect(authService.revokeOtherSessions()).resolves.toEqual({
			removed: 2,
		});
		expect(apiMock.deleteRequest).toHaveBeenCalledWith("/auth/sessions/others");
	});

	it("updates profile and avatar endpoints through the shared API client", async () => {
		apiMock.patch.mockResolvedValue({
			avatar: { source: "none", url_1024: null, url_512: null, version: 0 },
			display_name: "Aster",
		});
		apiMock.put.mockResolvedValue({
			avatar: {
				source: "gravatar",
				url_1024: "https://www.gravatar.com/avatar/hash?s=1024",
				url_512: "https://www.gravatar.com/avatar/hash?s=512",
				version: 1,
			},
			display_name: "Aster",
		});
		apiMock.post.mockResolvedValue({
			avatar: {
				source: "upload",
				url_1024: "/auth/profile/avatar/1024?v=2",
				url_512: "/auth/profile/avatar/512?v=2",
				version: 2,
			},
			display_name: "Aster",
		});
		const { authService } = await import("./authService");
		const file = new File(["avatar"], "avatar.webp", { type: "image/webp" });

		await authService.updateProfile({ display_name: "Aster" });
		await authService.setAvatarSource({ source: "gravatar" });
		await authService.uploadAvatar(file);

		expect(apiMock.patch).toHaveBeenCalledWith("/auth/profile", {
			display_name: "Aster",
		});
		expect(apiMock.put).toHaveBeenCalledWith("/auth/profile/avatar/source", {
			source: "gravatar",
		});
		expect(apiMock.post).toHaveBeenCalledWith(
			"/auth/profile/avatar/upload",
			expect.any(FormData),
		);
		const formData = apiMock.post.mock.calls.at(-1)?.[1] as FormData;
		expect(formData.get("file")).toBe(file);
	});

	it("uses the passkey management endpoints with explicit request bodies", async () => {
		apiMock.get.mockResolvedValue(offsetPage([], 20));
		apiMock.post.mockResolvedValue({
			flow_id: "flow-1",
			public_key: { challenge: "challenge-1" },
		});
		apiMock.patch.mockResolvedValue({
			backed_up: false,
			backup_eligible: true,
			created_at: "2026-06-15T00:00:00Z",
			id: 7,
			last_used_at: null,
			name: "MacBook",
			sign_count: 0,
			transports: ["internal"],
			updated_at: "2026-06-15T00:00:00Z",
		});
		apiMock.deleteRequest.mockResolvedValue(undefined);
		const { authService } = await import("./authService");

		await authService.listPasskeys();
		await authService.startPasskeyRegistration({ name: "MacBook" });
		await authService.finishPasskeyRegistration("flow-1", {
			id: "credential-1",
		});
		await authService.renamePasskey(7, { name: "Desk Mac" });
		await authService.deletePasskey(7);

		expect(apiMock.get).toHaveBeenCalledWith("/auth/passkeys?limit=20");
		expect(apiMock.post).toHaveBeenCalledWith("/auth/passkeys/register/start", {
			name: "MacBook",
		});
		expect(apiMock.post).toHaveBeenCalledWith(
			"/auth/passkeys/register/finish",
			{
				credential: { id: "credential-1" },
				flow_id: "flow-1",
				name: undefined,
			},
		);
		expect(apiMock.patch).toHaveBeenCalledWith("/auth/passkeys/7", {
			name: "Desk Mac",
		});
		expect(apiMock.deleteRequest).toHaveBeenCalledWith("/auth/passkeys/7");
	});

	it("starts and finishes passkey login through the auth API", async () => {
		apiMock.post.mockResolvedValueOnce({
			flow_id: "flow-2",
			public_key: { challenge: "challenge-2" },
		});
		apiMock.post.mockResolvedValueOnce({ expires_in: 3600 });
		const { authService } = await import("./authService");

		await authService.startPasskeyLogin({
			conditional: false,
			identifier: "steve",
		});
		await authService.finishPasskeyLogin("flow-2", { id: "credential-2" });

		expect(apiMock.post).toHaveBeenCalledWith("/auth/passkeys/login/start", {
			conditional: false,
			identifier: "steve",
		});
		expect(apiMock.post).toHaveBeenCalledWith("/auth/passkeys/login/finish", {
			credential: { id: "credential-2" },
			flow_id: "flow-2",
		});
	});

	it("caches the current auth user and returns cloned results", async () => {
		apiMock.get
			.mockResolvedValueOnce(authUser())
			.mockResolvedValueOnce(authUser({ username: "alex" }));
		const { authService } = await import("./authService");

		const first = await authService.me();
		first.profile.display_name = "Mutated";
		const cached = await authService.me();
		const refreshed = await authService.me({ force: true });

		expect(apiMock.get).toHaveBeenCalledTimes(2);
		expect(apiMock.get).toHaveBeenNthCalledWith(1, "/auth/me");
		expect(apiMock.get).toHaveBeenNthCalledWith(2, "/auth/me");
		expect(cached.profile.display_name).toBe("Steve");
		expect(refreshed.username).toBe("alex");
	});

	it("deduplicates pending session requests and invalidates them after revocation", async () => {
		let resolveSessions:
			| ((sessions: OffsetPageFixture<ReturnType<typeof authSession>>) => void)
			| undefined;
		apiMock.get.mockReturnValueOnce(
			new Promise((resolve) => {
				resolveSessions = resolve;
			}),
		);
		const { authService } = await import("./authService");

		const first = authService.sessions();
		const second = authService.sessions();
		resolveSessions?.(offsetPage([authSession()]));

		await expect(first).resolves.toHaveLength(1);
		await expect(second).resolves.toHaveLength(1);
		expect(apiMock.get).toHaveBeenCalledTimes(1);

		apiMock.deleteRequest.mockResolvedValueOnce(undefined);
		apiMock.get.mockResolvedValueOnce(
			offsetPage([authSession({ id: "session-2" })]),
		);
		await authService.revokeSession("session-1");

		await expect(authService.sessions()).resolves.toEqual([
			authSession({ id: "session-2" }),
		]);
		expect(apiMock.get).toHaveBeenCalledTimes(2);
	});

	it("caches passkeys, supports forced refresh, and syncs mutations", async () => {
		apiMock.get
			.mockResolvedValueOnce(offsetPage([passkey()], 20))
			.mockResolvedValueOnce(
				offsetPage([passkey({ id: 8, name: "Phone" })], 20),
			);
		const { authService } = await import("./authService");

		const first = await authService.listPasskeys();
		first[0].name = "Mutated";
		await expect(authService.listPasskeys()).resolves.toEqual([passkey()]);
		await expect(authService.listPasskeys({ force: true })).resolves.toEqual([
			passkey({ id: 8, name: "Phone" }),
		]);

		apiMock.post.mockResolvedValueOnce(
			passkey({ id: 9, name: "Security Key" }),
		);
		await authService.finishPasskeyRegistration("flow-1", { id: "cred" });
		await expect(authService.listPasskeys()).resolves.toEqual([
			passkey({ id: 9, name: "Security Key" }),
			passkey({ id: 8, name: "Phone" }),
		]);

		apiMock.patch.mockResolvedValueOnce(passkey({ id: 8, name: "Desk Mac" }));
		await authService.renamePasskey(8, { name: "Desk Mac" });
		await expect(authService.listPasskeys()).resolves.toEqual([
			passkey({ id: 9, name: "Security Key" }),
			passkey({ id: 8, name: "Desk Mac" }),
		]);

		apiMock.deleteRequest.mockResolvedValueOnce(undefined);
		await authService.deletePasskey(9);
		await expect(authService.listPasskeys()).resolves.toEqual([
			passkey({ id: 8, name: "Desk Mac" }),
		]);
		expect(apiMock.get).toHaveBeenCalledTimes(2);
	});
});

describe("admin services", () => {
	it("loads authenticated system info from the admin endpoint", async () => {
		apiMock.get.mockResolvedValue({
			build_time: "2026-06-15T08:30:00.000Z",
			uptime_seconds: 3723,
			version: "0.0.0-alpha.1",
		});
		const { adminSystemService } = await import("./adminService");

		await expect(adminSystemService.getInfo()).resolves.toEqual({
			build_time: "2026-06-15T08:30:00.000Z",
			uptime_seconds: 3723,
			version: "0.0.0-alpha.1",
		});
		expect(apiMock.get).toHaveBeenCalledWith("/admin/system-info");
	});

	it("builds config list queries from operation query parameters", async () => {
		apiMock.get.mockResolvedValue({
			items: [],
			limit: 25,
			offset: 50,
			total: 0,
		});
		const { adminConfigService } = await import("./adminService");

		await adminConfigService.list({ limit: 25, offset: 50 });

		expect(apiMock.get).toHaveBeenCalledWith(
			"/admin/config?limit=25&offset=50",
		);
	});

	it("uses admin user invitation endpoints", async () => {
		const invitation = {
			accepted_at: null,
			accepted_user_id: null,
			created_at: "2026-06-18T00:00:00Z",
			email: "invitee@example.com",
			expires_at: "2026-06-25T00:00:00Z",
			id: 42,
			invitation_url: "http://localhost:3300/invite/token",
			invited_by: 1,
			mail_queued: true,
			revoked_at: null,
			status: "pending" as const,
			updated_at: "2026-06-18T00:00:00Z",
		} satisfies import("@/types/api").AdminUserInvitationInfo;
		apiMock.get.mockResolvedValueOnce(offsetPage([invitation], 10, 0, 1));
		apiMock.post
			.mockResolvedValueOnce(invitation)
			.mockResolvedValueOnce({ ...invitation, status: "revoked" });
		const { adminUserService } = await import("./adminService");

		await adminUserService.listInvitations({
			after_created_at: "2026-06-18T00:00:00Z",
			after_id: 42,
			limit: 10,
		});
		await adminUserService.createInvitation({ email: "invitee@example.com" });
		await adminUserService.revokeInvitation(42);

		expect(apiMock.get).toHaveBeenCalledWith(
			"/admin/users/invitations?limit=10&after_created_at=2026-06-18T00%3A00%3A00Z&after_id=42",
		);
		expect(apiMock.post).toHaveBeenNthCalledWith(
			1,
			"/admin/users/invitations",
			{ email: "invitee@example.com" },
		);
		expect(apiMock.post).toHaveBeenNthCalledWith(
			2,
			"/admin/users/invitations/42/revoke",
		);
	});

	it("uses admin user account endpoints including deletion", async () => {
		const user = {
			active_session_count: 0,
			created_at: "2026-06-18T00:00:00Z",
			email: "alex@example.com",
			email_verified_at: null,
			id: 7,
			must_change_password: false,
			pending_email: null,
			profile: {
				display_name: "Alex",
				avatar: {
					source: "none",
					url_512: null,
					url_1024: null,
					version: 0,
				},
			},
			profile_count: 0,
			role: "user",
			session_version: 1,
			status: "active",
			updated_at: "2026-06-18T00:00:00Z",
			username: "alex",
		} satisfies import("@/types/api").AdminUserInfo;
		apiMock.get.mockResolvedValueOnce(offsetPage([user], 0, 20, 1));
		apiMock.get.mockResolvedValueOnce(user);
		apiMock.post.mockResolvedValueOnce({
			user,
			generated_password: "temporary",
		});
		apiMock.patch.mockResolvedValueOnce({ ...user, status: "disabled" });
		apiMock.post.mockResolvedValueOnce({ removed: 2 });
		apiMock.deleteRequest.mockResolvedValueOnce(undefined);
		const { adminUserService } = await import("./adminService");

		await adminUserService.list({
			after_created_at: "2026-06-18T00:00:00Z",
			after_id: 7,
			limit: 20,
			keyword: "alex",
		});
		await adminUserService.get(7);
		await adminUserService.create({
			username: "alex",
			email: "alex@example.com",
			password: null,
			must_change_password: false,
		});
		await adminUserService.update(7, { status: "disabled" });
		await adminUserService.revokeSessions(7);
		await adminUserService.delete(7);

		expect(apiMock.get).toHaveBeenNthCalledWith(
			1,
			"/admin/users?limit=20&keyword=alex&after_created_at=2026-06-18T00%3A00%3A00Z&after_id=7",
		);
		expect(apiMock.get).toHaveBeenNthCalledWith(2, "/admin/users/7");
		expect(apiMock.post).toHaveBeenNthCalledWith(1, "/admin/users", {
			username: "alex",
			email: "alex@example.com",
			password: null,
			must_change_password: false,
		});
		expect(apiMock.patch).toHaveBeenCalledWith("/admin/users/7", {
			status: "disabled",
		});
		expect(apiMock.post).toHaveBeenNthCalledWith(
			2,
			"/admin/users/7/sessions/revoke",
		);
		expect(apiMock.deleteRequest).toHaveBeenCalledWith("/admin/users/7");
	});

	it("passes config updates as generated request bodies", async () => {
		const payload = { value: "Aster", visibility: "public" as const };
		apiMock.put.mockResolvedValue({
			config: {
				category: "general",
				description: "",
				id: 1,
				is_sensitive: false,
				key: "site.name",
				namespace: "system",
				requires_restart: false,
				source: "custom",
				updated_at: "2026-06-15T00:00:00Z",
				updated_by: null,
				value: "Aster",
				value_type: "string",
				visibility: "public",
			},
			warnings: [],
		});
		const { adminConfigService } = await import("./adminService");

		await expect(adminConfigService.set("site.name", payload)).resolves.toEqual(
			{
				config: expect.objectContaining({
					key: "site.name",
					value: "Aster",
				}),
				warnings: [],
			},
		);

		expect(apiMock.put).toHaveBeenCalledWith(
			"/admin/config/site.name",
			payload,
		);
	});

	it("loads config template variables", async () => {
		apiMock.get.mockResolvedValue([
			{
				category: "mail.template",
				label_i18n_key: "settings_mail_template_group_password_reset",
				template_code: "password_reset",
				variables: [],
			},
		]);
		const { adminConfigService } = await import("./adminService");

		await adminConfigService.templateVariables();

		expect(apiMock.get).toHaveBeenCalledWith(
			"/admin/config/template-variables",
		);
	});

	it("executes config actions through the generated action endpoint", async () => {
		apiMock.post.mockResolvedValue({ message: "done", value: null });
		const { adminConfigService } = await import("./adminService");

		await adminConfigService.action("mail", {
			action: "send_test_email",
			values: {
				target_email: "admin@example.com",
			},
		});

		expect(apiMock.post).toHaveBeenCalledWith("/admin/config/mail/action", {
			action: "send_test_email",
			values: {
				target_email: "admin@example.com",
			},
		});
	});

	it("sends test email actions and normalizes an empty target", async () => {
		apiMock.post.mockResolvedValue({ message: "sent", value: null });
		const { adminConfigService } = await import("./adminService");

		await adminConfigService.sendTestEmail("   ");

		expect(apiMock.post).toHaveBeenCalledWith("/admin/config/mail/action", {
			action: "send_test_email",
			values: {
				target_email: "",
			},
		});
	});

	it("previews captcha through config action values", async () => {
		apiMock.post.mockResolvedValue({ message: "preview", value: "data:image" });
		const { adminConfigService } = await import("./adminService");

		await adminConfigService.previewCaptcha({
			auth_captcha_length: "6",
			auth_captcha_preset: ["hardened"],
		});

		expect(apiMock.post).toHaveBeenCalledWith(
			"/admin/config/auth_captcha/action",
			{
				action: "preview_captcha",
				values: {
					auth_captcha_length: "6",
					auth_captcha_preset: ["hardened"],
				},
			},
		);
	});

	it("rotates the Yggdrasil signature key through a config action", async () => {
		apiMock.post.mockResolvedValue({ message: "rotated", value: null });
		const { adminConfigService } = await import("./adminService");

		await adminConfigService.rotateYggdrasilSignatureKey();

		expect(apiMock.post).toHaveBeenCalledWith(
			"/admin/config/yggdrasil/action",
			{
				action: "rotate_yggdrasil_signature_key",
			},
		);
	});

	it("uses the retry task operation response instead of paging item inference", async () => {
		apiMock.post.mockResolvedValue({ id: 42, status: "pending" });
		const { adminTaskService } = await import("./adminService");

		await adminTaskService.retry(42);

		expect(apiMock.post).toHaveBeenCalledWith("/admin/tasks/42/retry");
	});

	it("manages texture library tags through administrator APIs", async () => {
		apiMock.get.mockResolvedValue(offsetPage([], 20, 40, 60));
		apiMock.post.mockResolvedValue({
			color: "#228855",
			created_at: "2026-06-15T00:00:00Z",
			id: 3,
			name: "Featured",
			sort_order: 10,
			updated_at: "2026-06-15T00:00:00Z",
		});
		apiMock.patch.mockResolvedValue({
			color: "#334455",
			created_at: "2026-06-15T00:00:00Z",
			id: 3,
			name: "Classic",
			sort_order: 5,
			updated_at: "2026-06-15T00:00:00Z",
		});
		const { adminTextureLibraryService } = await import("./adminService");

		await adminTextureLibraryService.listTags({ limit: 20, offset: 40 });
		await adminTextureLibraryService.createTag({
			color: "#228855",
			name: "Featured",
			sort_order: 10,
		});
		await adminTextureLibraryService.updateTag(3, {
			color: "#334455",
			name: "Classic",
			sort_order: 5,
		});
		await adminTextureLibraryService.deleteTag(3);

		expect(apiMock.get).toHaveBeenCalledWith(
			"/admin/texture-library/tags?limit=20&offset=40",
		);
		expect(apiMock.post).toHaveBeenCalledWith("/admin/texture-library/tags", {
			color: "#228855",
			name: "Featured",
			sort_order: 10,
		});
		expect(apiMock.patch).toHaveBeenCalledWith(
			"/admin/texture-library/tags/3",
			{
				color: "#334455",
				name: "Classic",
				sort_order: 5,
			},
		);
		expect(apiMock.deleteRequest).toHaveBeenCalledWith(
			"/admin/texture-library/tags/3",
		);
	});

	it("manages Yggdrasil session forwarding servers through administrator APIs", async () => {
		apiMock.get.mockResolvedValueOnce(offsetPage([], 20, 40, 0));
		apiMock.get.mockResolvedValueOnce({
			base_url: "https://remote.example.test/yggdrasil",
			created_at: "2026-06-20T00:00:00Z",
			deletable: true,
			display_name: "Remote",
			enabled: true,
			id: 7,
			last_checked_at: null,
			last_failure_at: null,
			last_failure_message: null,
			last_success_at: null,
			local: false,
			priority: 50,
			provider_kind: "remote",
			texture_forward_enabled: true,
			timeout_ms: 1500,
			updated_at: "2026-06-20T00:00:00Z",
			weight: 2,
		});
		const { adminYggdrasilSessionForwardService } = await import(
			"./adminService"
		);

		await adminYggdrasilSessionForwardService.list({
			limit: 20,
			offset: 40,
		});
		await adminYggdrasilSessionForwardService.get(7);
		await adminYggdrasilSessionForwardService.create({
			base_url: "https://remote.example.test/yggdrasil",
			display_name: "Remote",
			enabled: true,
			priority: 50,
			texture_forward_enabled: true,
			timeout_ms: 1500,
			weight: 2,
		});
		await adminYggdrasilSessionForwardService.update(7, {
			display_name: "Remote Primary",
			priority: 20,
		});
		await adminYggdrasilSessionForwardService.delete(7);

		expect(apiMock.get).toHaveBeenCalledWith(
			"/admin/yggdrasil/session-forward-servers?limit=20&offset=40",
		);
		expect(apiMock.get).toHaveBeenCalledWith(
			"/admin/yggdrasil/session-forward-servers/7",
		);
		expect(apiMock.post).toHaveBeenCalledWith(
			"/admin/yggdrasil/session-forward-servers",
			{
				base_url: "https://remote.example.test/yggdrasil",
				display_name: "Remote",
				enabled: true,
				priority: 50,
				texture_forward_enabled: true,
				timeout_ms: 1500,
				weight: 2,
			},
		);
		expect(apiMock.patch).toHaveBeenCalledWith(
			"/admin/yggdrasil/session-forward-servers/7",
			{
				display_name: "Remote Primary",
				priority: 20,
			},
		);
		expect(apiMock.deleteRequest).toHaveBeenCalledWith(
			"/admin/yggdrasil/session-forward-servers/7",
		);
	});

	it("manages texture library moderation textures through administrator APIs", async () => {
		apiMock.get.mockResolvedValueOnce(offsetPage([], 20, 0, 0));
		apiMock.get.mockResolvedValueOnce({
			created_at: "2026-06-15T00:00:00Z",
			display_name: "Review Skin",
			file_size: 3,
			hash: "hash-review",
			height: 64,
			id: 12,
			library_review_note: null,
			library_reviewed_at: "2026-06-15T01:00:00Z",
			library_status: "pending_review",
			library_submitted_at: "2026-06-15T00:30:00Z",
			mime_type: "image/png",
			name: "Review Skin",
			tags: [],
			texture_model: "slim",
			texture_type: "skin",
			updated_at: "2026-06-15T00:00:00Z",
			uploader: {
				avatar: {
					source: "none",
					url_1024: null,
					url_512: null,
					version: 0,
				},
				id: 1,
				name: "Steve",
				public_uuid: "user-public-uuid",
				username: "steve",
			},
			url: "/textures/hash-review.png",
			visibility: "public",
			width: 64,
		});
		apiMock.post.mockResolvedValue({
			created_at: "2026-06-15T00:00:00Z",
			display_name: "Review Skin",
			file_size: 3,
			hash: "hash-review",
			height: 64,
			id: 12,
			library_review_note: "ok",
			library_reviewed_at: "2026-06-15T01:00:00Z",
			library_status: "published",
			library_submitted_at: "2026-06-15T00:30:00Z",
			mime_type: "image/png",
			name: "Review Skin",
			tags: [],
			texture_model: "slim",
			texture_type: "skin",
			updated_at: "2026-06-15T00:00:00Z",
			uploader: {
				avatar: {
					source: "none",
					url_1024: null,
					url_512: null,
					version: 0,
				},
				id: 1,
				name: "Steve",
				public_uuid: "user-public-uuid",
				username: "steve",
			},
			url: "/textures/hash-review.png",
			visibility: "public",
			width: 64,
		});
		const { adminTextureLibraryService } = await import("./adminService");

		await adminTextureLibraryService.listTextures({
			after_id: 42,
			after_updated_at: "2026-06-15T00:00:00Z",
			keyword: "Review",
			library_status: "pending_review",
			limit: 20,
			published: false,
			tag_ids: [3, 4],
			tag_search_method: "any",
			texture_type: "skin",
			visibility: "public",
		});
		await adminTextureLibraryService.getTexture(12);
		await adminTextureLibraryService.deleteTexture(12);
		await adminTextureLibraryService.approveTexture(12, {
			review_note: "ok",
			tag_ids: [3],
		});
		await adminTextureLibraryService.rejectTexture(12, {
			review_note: "bad dimensions",
		});
		await adminTextureLibraryService.unpublishTexture(12, {
			review_note: null,
		});
		await adminTextureLibraryService.listReports({
			after_created_at: "2026-06-15T00:00:00Z",
			after_id: 7,
			limit: 20,
			reason: "copyright",
			status: "pending",
			texture_id: 12,
		});
		await adminTextureLibraryService.getReport(7);
		await adminTextureLibraryService.acceptReport(7, {
			admin_note: "confirmed",
		});
		await adminTextureLibraryService.rejectReport(8, {
			admin_note: "not enough evidence",
		});

		expect(apiMock.get).toHaveBeenNthCalledWith(
			1,
			"/admin/texture-library/textures?limit=20&after_updated_at=2026-06-15T00%3A00%3A00Z&after_id=42&keyword=Review&texture_type=skin&visibility=public&library_status=pending_review&published=false&tag_ids=3%2C4&tag_search_method=any",
		);
		expect(apiMock.get).toHaveBeenNthCalledWith(
			2,
			"/admin/texture-library/textures/12",
		);
		expect(apiMock.deleteRequest).toHaveBeenCalledWith(
			"/admin/texture-library/textures/12",
		);
		expect(apiMock.get).toHaveBeenNthCalledWith(
			3,
			"/admin/texture-library/reports?limit=20&status=pending&reason=copyright&texture_id=12&after_created_at=2026-06-15T00%3A00%3A00Z&after_id=7",
		);
		expect(apiMock.get).toHaveBeenNthCalledWith(
			4,
			"/admin/texture-library/reports/7",
		);
		expect(apiMock.post).toHaveBeenNthCalledWith(
			1,
			"/admin/texture-library/textures/12/approve",
			{ review_note: "ok", tag_ids: [3] },
		);
		expect(apiMock.post).toHaveBeenNthCalledWith(
			2,
			"/admin/texture-library/textures/12/reject",
			{ review_note: "bad dimensions" },
		);
		expect(apiMock.post).toHaveBeenNthCalledWith(
			3,
			"/admin/texture-library/textures/12/unpublish",
			{ review_note: null },
		);
		expect(apiMock.post).toHaveBeenNthCalledWith(
			4,
			"/admin/texture-library/reports/7/accept",
			{ admin_note: "confirmed" },
		);
		expect(apiMock.post).toHaveBeenNthCalledWith(
			5,
			"/admin/texture-library/reports/8/reject",
			{ admin_note: "not enough evidence" },
		);
	});
});

describe("systemService", () => {
	it("loads public health as a raw root response and readiness as an API envelope", async () => {
		const signal = new AbortController().signal;
		apiMock.rootClientGet.mockResolvedValue({ data: { status: "ok" } });
		apiMock.rootGet.mockResolvedValue({ status: "ready" });
		const { systemService } = await import("./systemService");

		await expect(systemService.health(signal)).resolves.toEqual({
			status: "ok",
		});
		await expect(systemService.ready(signal)).resolves.toEqual({
			status: "ready",
		});

		expect(apiMock.rootClientGet).toHaveBeenCalledWith("/health", { signal });
		expect(apiMock.rootGet).toHaveBeenCalledWith("/health/ready", { signal });
	});
});

describe("externalAuthService", () => {
	it("caches public providers, clones results, and supports forced refresh", async () => {
		apiMock.get
			.mockResolvedValueOnce(offsetPage([externalAuthProvider()], 20))
			.mockResolvedValueOnce(
				offsetPage(
					[
						externalAuthProvider({
							display_name: "Microsoft",
							key: "microsoft",
						}),
					],
					20,
				),
			);
		const { externalAuthService } = await import("./externalAuthService");

		const first = await externalAuthService.listPublic();
		first[0].display_name = "Mutated";
		await expect(externalAuthService.listAuthAliases()).resolves.toEqual([
			externalAuthProvider(),
		]);
		await expect(
			externalAuthService.listPublic({ force: true }),
		).resolves.toEqual([
			externalAuthProvider({ display_name: "Microsoft", key: "microsoft" }),
		]);

		expect(apiMock.get).toHaveBeenCalledTimes(2);
		expect(apiMock.get).toHaveBeenNthCalledWith(
			1,
			"/auth/external-auth/providers?limit=20&offset=0",
			{
				signal: undefined,
			},
		);
		expect(apiMock.get).toHaveBeenNthCalledWith(
			2,
			"/auth/external-auth/providers?limit=20&offset=0",
			{
				signal: undefined,
			},
		);
	});

	it("deduplicates public provider requests without sharing abortable calls", async () => {
		let resolveProviders:
			| ((
					providers: OffsetPageFixture<ReturnType<typeof externalAuthProvider>>,
			  ) => void)
			| undefined;
		apiMock.get.mockReturnValueOnce(
			new Promise((resolve) => {
				resolveProviders = resolve;
			}),
		);
		const { externalAuthService } = await import("./externalAuthService");

		const first = externalAuthService.listPublic();
		const second = externalAuthService.listAuthAliases();
		resolveProviders?.(offsetPage([externalAuthProvider()], 20));

		await expect(first).resolves.toHaveLength(1);
		await expect(second).resolves.toHaveLength(1);
		expect(apiMock.get).toHaveBeenCalledTimes(1);

		const signal = new AbortController().signal;
		apiMock.get.mockResolvedValueOnce(
			offsetPage([externalAuthProvider({ key: "oidc" })], 20),
		);
		await externalAuthService.listPublic(signal);

		expect(apiMock.get).toHaveBeenCalledTimes(2);
		expect(apiMock.get).toHaveBeenLastCalledWith(
			"/auth/external-auth/providers?limit=20&offset=0",
			{ signal },
		);
	});

	it("loads public providers by kind through paginated endpoints", async () => {
		apiMock.get.mockResolvedValueOnce(
			offsetPage([externalAuthProvider({ kind: "oidc", key: "oidc" })], 20),
		);
		const { externalAuthService } = await import("./externalAuthService");

		await expect(
			externalAuthService.listAuthAliasesByKind("oidc"),
		).resolves.toEqual([externalAuthProvider({ kind: "oidc", key: "oidc" })]);

		expect(apiMock.get).toHaveBeenCalledWith(
			"/auth/external-auth/oidc/providers?limit=20&offset=0",
			{ signal: undefined },
		);
	});

	it("caches external auth links and syncs deletes", async () => {
		apiMock.get
			.mockResolvedValueOnce(offsetPage([externalAuthLink()], 20))
			.mockResolvedValueOnce(
				offsetPage([externalAuthLink({ id: 12, subject: "subject-2" })], 20),
			);
		const { externalAuthService } = await import("./externalAuthService");

		const first = await externalAuthService.listLinks();
		first[0].subject = "mutated";
		await expect(externalAuthService.listLinks()).resolves.toEqual([
			externalAuthLink(),
		]);
		await expect(
			externalAuthService.listLinks({ force: true }),
		).resolves.toEqual([externalAuthLink({ id: 12, subject: "subject-2" })]);

		apiMock.deleteRequest.mockResolvedValueOnce(undefined);
		await externalAuthService.deleteLink(12);
		await expect(externalAuthService.listLinks()).resolves.toEqual([]);
		expect(apiMock.get).toHaveBeenCalledTimes(2);
	});

	it("sends auth namespace start requests with generated body types", async () => {
		apiMock.post.mockResolvedValue({
			authorization_url: "https://provider.example/start",
		});
		const { externalAuthService } = await import("./externalAuthService");

		await externalAuthService.startAuthAlias("github", "github", {
			return_path: "/account",
		});

		expect(apiMock.post).toHaveBeenCalledWith(
			"/auth/external-auth/github/github/start",
			{
				return_path: "/account",
			},
		);
	});

	it("reflects the auth namespace callback operation", async () => {
		apiMock.get.mockResolvedValue(undefined);
		const { externalAuthService } = await import("./externalAuthService");

		await expect(
			externalAuthService.finishAuthAlias(
				"github",
				"github",
				"state-1",
				"code-1",
			),
		).resolves.toBeUndefined();

		expect(apiMock.get).toHaveBeenCalledWith(
			"/auth/external-auth/github/github/callback?state=state-1&code=code-1",
		);
	});
});

describe("yggdrasilService", () => {
	it("uses configured public Yggdrasil API root before the current page host", async () => {
		const { yggdrasilApiRoot } = await import("./yggdrasilService");

		expect(
			yggdrasilApiRoot({
				public_base_urls: ["https://skin.example.test/api/yggdrasil"],
			}),
		).toBe("https://skin.example.test/api/yggdrasil");
	});

	it("loads protocol metadata using the generated protocol response type", async () => {
		const metadata = {
			meta: {
				feature: { non_email_login: true },
				implementationName: "AsterYggdrasil",
				implementationVersion: "0.1.0",
				serverName: "Aster",
			},
			signaturePublickey: "public-key",
			skinDomains: ["textures.example"],
		};
		apiMock.rootClientGet.mockResolvedValue({ data: metadata });
		const { yggdrasilService } = await import("./yggdrasilService");

		await expect(yggdrasilService.metadata()).resolves.toEqual(metadata);
		expect(apiMock.rootClientGet).toHaveBeenCalledWith("/", {
			signal: undefined,
		});
	});

	it("uploads textures through the protocol endpoint with bearer auth", async () => {
		const file = new File(["png"], "skin.png", { type: "image/png" });
		apiMock.rootClientRequest.mockResolvedValue({ status: 204 });
		const { yggdrasilService } = await import("./yggdrasilService");

		await yggdrasilService.uploadTexture({
			accessToken: "token-1",
			file,
			model: "slim",
			textureType: "skin",
			uuid: "profile-uuid",
		});

		expect(apiMock.rootClientRequest).toHaveBeenCalledWith(
			expect.objectContaining({
				data: expect.any(FormData),
				headers: { Authorization: "Bearer token-1" },
				method: "put",
				url: "/api/user/profile/profile-uuid/skin",
			}),
		);
		const request = apiMock.rootClientRequest.mock.calls[0]?.[0] as {
			data: FormData;
		};
		expect(request.data.get("model")).toBe("slim");
		expect(request.data.get("file")).toBe(file);
	});

	it("renames current-user Minecraft profiles through the project API", async () => {
		apiMock.put.mockResolvedValue({ id: "profile-uuid", name: "NewName" });
		const { yggdrasilService } = await import("./yggdrasilService");

		await expect(
			yggdrasilService.renameProfile("profile-uuid", { name: "NewName" }),
		).resolves.toEqual({ id: "profile-uuid", name: "NewName" });

		expect(apiMock.put).toHaveBeenCalledWith(
			"/profiles/minecraft/profile-uuid/name",
			{ name: "NewName" },
		);
	});

	it("deletes current-user Minecraft profiles through the project API", async () => {
		apiMock.deleteRequest.mockResolvedValue(undefined);
		const { yggdrasilService } = await import("./yggdrasilService");

		await expect(
			yggdrasilService.deleteProfile("profile-uuid"),
		).resolves.toBeUndefined();

		expect(apiMock.deleteRequest).toHaveBeenCalledWith(
			"/profiles/minecraft/profile-uuid",
		);
	});

	it("loads current-page profile skin avatar URLs through project texture APIs", async () => {
		apiMock.get.mockImplementation((path: string) => {
			if (path === "/profiles/minecraft/profile-one/textures") {
				return Promise.resolve([
					{
						source: "bound",
						texture_type: "skin",
						url: "/textures/profile-one-skin.png",
					},
					{
						source: "bound",
						texture_type: "cape",
						url: "/textures/profile-one-cape.png",
					},
				]);
			}
			if (path === "/profiles/minecraft/profile-two/textures") {
				return Promise.resolve([
					{
						source: "default",
						texture_type: "skin",
						url: "/textures/default-skin.png",
					},
				]);
			}
			return Promise.reject(new Error(`unexpected path: ${path}`));
		});
		const { yggdrasilService } = await import("./yggdrasilService");

		await expect(
			yggdrasilService.listProfileSkinTextureUrls([
				"profile-one",
				"profile-two",
			]),
		).resolves.toEqual({
			"profile-one": "/textures/profile-one-skin.png",
			"profile-two": null,
		});

		expect(apiMock.get).toHaveBeenCalledWith(
			"/profiles/minecraft/profile-one/textures",
		);
		expect(apiMock.get).toHaveBeenCalledWith(
			"/profiles/minecraft/profile-two/textures",
		);
		expect(apiMock.rootClientGet).not.toHaveBeenCalled();
		expect(apiMock.rootClientRequest).not.toHaveBeenCalled();
	});

	it("reuses concurrent current-user profile texture requests for the same uuid", async () => {
		apiMock.get.mockImplementation((path: string) => {
			if (path === "/profiles/minecraft/profile-one/textures") {
				return Promise.resolve([
					{
						source: "bound",
						texture_type: "skin",
						url: "/textures/profile-one-skin.png",
					},
				]);
			}
			if (path === "/profiles/minecraft/profile-two/textures") {
				return Promise.resolve([]);
			}
			return Promise.reject(new Error(`unexpected path: ${path}`));
		});
		const { yggdrasilService } = await import("./yggdrasilService");

		await Promise.all([
			yggdrasilService.listProfileTextures("profile-one"),
			yggdrasilService.listProfileSkinTextureUrls([
				"profile-one",
				"profile-two",
			]),
		]);

		expect(apiMock.get).toHaveBeenCalledTimes(2);
		expect(
			apiMock.get.mock.calls.filter(
				([path]) => path === "/profiles/minecraft/profile-one/textures",
			),
		).toHaveLength(1);
		expect(apiMock.get).toHaveBeenCalledWith(
			"/profiles/minecraft/profile-two/textures",
		);
	});

	it("renames admin Minecraft profiles through the admin API", async () => {
		apiMock.put.mockResolvedValue({
			created_at: "2026-06-15T00:00:00Z",
			id: 7,
			name: "AdminRenamed",
			texture_model: "default",
			updated_at: "2026-06-15T00:00:00Z",
			uploadable_textures: "skin,cape",
			user_id: 1,
			uuid: "profile-uuid",
		});
		const { adminMinecraftProfileService } = await import("./adminService");

		await expect(
			adminMinecraftProfileService.rename("profile-uuid", {
				name: "AdminRenamed",
			}),
		).resolves.toMatchObject({ name: "AdminRenamed", uuid: "profile-uuid" });

		expect(apiMock.put).toHaveBeenCalledWith(
			"/admin/minecraft-profiles/profile-uuid/name",
			{ name: "AdminRenamed" },
		);
	});

	it("uploads wardrobe textures as multipart FormData", async () => {
		const file = new File(["png"], "skin.png", { type: "image/png" });
		apiMock.post.mockResolvedValue({
			created_at: "2026-06-15T00:00:00Z",
			display_name: "Blue Jacket",
			file_size: 3,
			hash: "hash-1",
			height: 64,
			id: 9,
			library_status: "private",
			mime_type: "image/png",
			name: "Blue Jacket",
			tags: [],
			texture_model: "slim",
			texture_type: "skin",
			updated_at: "2026-06-15T00:00:00Z",
			url: "/textures/hash-1.png",
			width: 64,
		});
		const { yggdrasilService } = await import("./yggdrasilService");

		await yggdrasilService.uploadWardrobeTexture({
			file,
			model: "slim",
			name: " Blue Jacket ",
			textureType: "skin",
			visibility: "public",
		});

		expect(apiMock.post).toHaveBeenCalledWith(
			"/wardrobe/textures/skin",
			expect.any(FormData),
		);
		const form = apiMock.post.mock.calls[0]?.[1] as FormData;
		expect(form.get("model")).toBe("slim");
		expect(form.get("name")).toBe("Blue Jacket");
		expect(form.get("visibility")).toBe("public");
		expect(form.get("file")).toBe(file);
	});

	it("omits blank wardrobe texture upload names and updates texture metadata", async () => {
		const file = new File(["png"], "cape.png", { type: "image/png" });
		apiMock.post.mockResolvedValue({
			created_at: "2026-06-15T00:00:00Z",
			display_name: null,
			file_size: 3,
			hash: "hash-2",
			height: 32,
			id: 10,
			library_status: "private",
			mime_type: "image/png",
			name: "hash-2",
			tags: [],
			texture_model: "default",
			texture_type: "cape",
			updated_at: "2026-06-15T00:00:00Z",
			url: "/textures/hash-2.png",
			visibility: "private",
			width: 64,
		});
		apiMock.patch.mockResolvedValue({
			created_at: "2026-06-15T00:00:00Z",
			display_name: null,
			file_size: 3,
			hash: "hash-2",
			height: 32,
			id: 10,
			library_status: "private",
			mime_type: "image/png",
			name: "hash-2",
			tags: [],
			texture_model: "default",
			texture_type: "cape",
			updated_at: "2026-06-15T00:00:00Z",
			url: "/textures/hash-2.png",
			visibility: "public",
			width: 64,
		});
		const { yggdrasilService } = await import("./yggdrasilService");

		await yggdrasilService.uploadWardrobeTexture({
			file,
			name: "   ",
			textureType: "cape",
		});
		await yggdrasilService.updateWardrobeTexture(10, {
			display_name: null,
			visibility: "public",
		});

		const form = apiMock.post.mock.calls[0]?.[1] as FormData;
		expect(form.has("name")).toBe(false);
		expect(apiMock.patch).toHaveBeenCalledWith("/wardrobe/textures/10", {
			display_name: null,
			visibility: "public",
		});
	});

	it("loads and copies public texture library items through project APIs", async () => {
		apiMock.get.mockResolvedValue(offsetPage([], 12, 24, 36));
		apiMock.post.mockResolvedValue({
			created_at: "2026-06-15T00:00:00Z",
			display_name: "Shared Skin",
			file_size: 3,
			hash: "hash-3",
			height: 64,
			id: 12,
			library_status: "private",
			mime_type: "image/png",
			name: "Shared Skin",
			tags: [],
			texture_model: "slim",
			texture_type: "skin",
			updated_at: "2026-06-15T00:00:00Z",
			url: "/textures/hash-3.png",
			visibility: "private",
			width: 64,
		});
		const { yggdrasilService } = await import("./yggdrasilService");

		await yggdrasilService.listPublicTextureLibraryTextures({
			after_id: 31,
			after_updated_at: "2026-06-15T00:00:00Z",
			keyword: "Shared",
			limit: 12,
			tag_ids: [3],
			tag_search_method: "all",
			texture_type: "skin",
		});
		await yggdrasilService.listPublicTextureLibraryTags({
			offset: 0,
			keyword: "Featured",
		});
		await yggdrasilService.getPublicTextureLibraryTexture(12);
		await yggdrasilService.copyPublicTextureToWardrobe(12, {
			display_name: "Shared Copy",
		});
		await yggdrasilService.createTextureReport(12, {
			message: "copied from another site",
			reason: "copyright",
		});
		await yggdrasilService.submitTextureLibraryReview(12);
		await yggdrasilService.withdrawTextureLibrarySubmission(12);

		expect(apiMock.get).toHaveBeenCalledWith(
			"/texture-library/textures?limit=12&after_updated_at=2026-06-15T00%3A00%3A00Z&after_id=31&keyword=Shared&texture_type=skin&tag_ids=3&tag_search_method=all",
		);
		expect(apiMock.get).toHaveBeenCalledWith(
			"/texture-library/tags?limit=30&offset=0&keyword=Featured",
		);
		expect(apiMock.get).toHaveBeenCalledWith("/texture-library/textures/12");
		expect(apiMock.post).toHaveBeenCalledWith(
			"/texture-library/textures/12/copy",
			{ display_name: "Shared Copy" },
		);
		expect(apiMock.post).toHaveBeenCalledWith(
			"/texture-library/textures/12/reports",
			{ message: "copied from another site", reason: "copyright" },
		);
		expect(apiMock.post).toHaveBeenCalledWith(
			"/wardrobe/textures/12/library-submission",
		);
		expect(apiMock.deleteRequest).toHaveBeenCalledWith(
			"/wardrobe/textures/12/library-submission",
		);
	});

	it("lists current user texture tags and replaces wardrobe texture tags", async () => {
		apiMock.get.mockResolvedValueOnce(
			offsetPage(
				[
					{
						color: "#228855",
						created_at: "2026-06-15T00:00:00Z",
						id: 3,
						name: "Featured",
						sort_order: 10,
						updated_at: "2026-06-15T00:00:00Z",
					},
				],
				50,
				0,
				1,
			),
		);
		apiMock.put.mockResolvedValue({
			created_at: "2026-06-15T00:00:00Z",
			display_name: "Tagged Skin",
			file_size: 3,
			hash: "hash-4",
			height: 64,
			id: 12,
			library_status: "private",
			mime_type: "image/png",
			name: "Tagged Skin",
			tags: [],
			texture_model: "slim",
			texture_type: "skin",
			updated_at: "2026-06-15T00:00:00Z",
			url: "/textures/hash-4.png",
			visibility: "private",
			width: 64,
		});
		const { yggdrasilService } = await import("./yggdrasilService");

		await expect(yggdrasilService.listTextureLibraryTags()).resolves.toEqual([
			{
				color: "#228855",
				created_at: "2026-06-15T00:00:00Z",
				id: 3,
				name: "Featured",
				sort_order: 10,
				updated_at: "2026-06-15T00:00:00Z",
			},
		]);
		await yggdrasilService.listTextureLibraryTags();
		await yggdrasilService.replaceWardrobeTextureTags(12, {
			tag_ids: [3, 4],
		});

		expect(apiMock.get).toHaveBeenCalledTimes(1);
		expect(apiMock.get).toHaveBeenCalledWith(
			"/wardrobe/tags?limit=30&offset=0",
		);
		expect(apiMock.put).toHaveBeenCalledWith("/wardrobe/textures/12/tags", {
			tag_ids: [3, 4],
		});
	});

	it("loads current user texture tags page by page when cache is refreshed", async () => {
		apiMock.get
			.mockResolvedValueOnce(
				offsetPage(
					Array.from({ length: 30 }, (_, index) => ({
						color: "#228855",
						created_at: "2026-06-15T00:00:00Z",
						id: index + 1,
						name: `Tag ${index + 1}`,
						sort_order: index,
						updated_at: "2026-06-15T00:00:00Z",
					})),
					30,
					0,
					51,
				),
			)
			.mockResolvedValueOnce(
				offsetPage(
					Array.from({ length: 21 }, (_, index) => ({
						color: "#334455",
						created_at: "2026-06-15T00:00:00Z",
						id: index + 31,
						name: `Tag ${index + 31}`,
						sort_order: index + 31,
						updated_at: "2026-06-15T00:00:00Z",
					})),
					30,
					30,
					51,
				),
			);
		const { yggdrasilService } = await import("./yggdrasilService");

		const tags = await yggdrasilService.listTextureLibraryTags({ force: true });

		expect(tags).toHaveLength(51);
		expect(apiMock.get).toHaveBeenNthCalledWith(
			1,
			"/wardrobe/tags?limit=30&offset=0",
		);
		expect(apiMock.get).toHaveBeenNthCalledWith(
			2,
			"/wardrobe/tags?limit=30&offset=30",
		);
	});

	it("maps generated Yggdrasil error bodies to protocol errors", async () => {
		apiMock.rootClientRequest.mockRejectedValue({
			response: {
				data: {
					cause: "texture",
					error: "ForbiddenOperationException",
					errorMessage: "Invalid token",
				},
				status: 403,
			},
		});
		const { YggdrasilProtocolError, yggdrasilService } = await import(
			"./yggdrasilService"
		);

		await expect(
			yggdrasilService.deleteTexture({
				accessToken: "bad-token",
				textureType: "cape",
				uuid: "profile-uuid",
			}),
		).rejects.toMatchObject({
			cause: "texture",
			error: "ForbiddenOperationException",
			message: "Invalid token",
			status: 403,
		});

		await expect(
			yggdrasilService.deleteTexture({
				accessToken: "bad-token",
				textureType: "cape",
				uuid: "profile-uuid",
			}),
		).rejects.toBeInstanceOf(YggdrasilProtocolError);
	});
});
