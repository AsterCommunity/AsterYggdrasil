import type {
	AccountAuditLogPage,
	AdminTaskPage,
	AdminUserBanPage,
	AdminUserInfo,
	AdminUserInvitationInfo,
	AdminUserInvitationPage,
	AdminUserPage,
	ApiResponse,
	AuditLogEntry,
	AuditLogPage,
	AuthSessionInfo,
	AuthSessionPage,
	AuthUserInfo,
	AvatarInfo,
	PasskeyInfo,
	PasskeyPage,
	TaskInfo,
	UserBanEventInfo,
	UserBanInfo,
	UserProfileInfo,
} from "@/types/api";

/**
 * Shared typed fixtures for unit tests.
 *
 * Factories derive from the generated API types (`@/types/api`) so a schema
 * change surfaces as a type error here instead of N broken test literals.
 * The `apiResponse` envelope mirrors what `services/http.ts` unwraps:
 * `{ code: "success", msg, data }` with an optional `error` detail.
 */

export function apiResponse<T>(
	data: T,
	overrides: Partial<ApiResponse<T>> = {},
): ApiResponse<T> {
	return {
		code: "success",
		msg: "",
		data,
		...overrides,
	};
}

export function createAvatarInfo(
	overrides: Partial<AvatarInfo> = {},
): AvatarInfo {
	return {
		source: "none",
		url_512: null,
		url_1024: null,
		version: 0,
		...overrides,
	};
}

export function createUserProfileInfo(
	overrides: Partial<UserProfileInfo> = {},
): UserProfileInfo {
	return {
		avatar: createAvatarInfo(),
		display_name: null,
		...overrides,
	};
}

export function createAdminUser(
	overrides: Partial<AdminUserInfo> = {},
): AdminUserInfo {
	return {
		id: 7,
		username: "alex",
		email: "alex@example.com",
		pending_email: null,
		email_verified_at: "2026-06-18T00:00:00.000Z",
		role: "user",
		status: "active",
		must_change_password: false,
		session_version: 1,
		profile_count: 0,
		active_session_count: 0,
		operator_scopes: [],
		profile: createUserProfileInfo(),
		created_at: "2026-06-18T00:00:00.000Z",
		updated_at: "2026-06-18T00:00:00.000Z",
		...overrides,
	};
}

export function createAuthUser(
	overrides: Partial<AuthUserInfo> = {},
): AuthUserInfo {
	return {
		id: 7,
		username: "alex",
		email: "alex@example.com",
		pending_email: null,
		email_verified: true,
		must_change_password: false,
		role: "user",
		status: "active",
		operator_scopes: [],
		profile: createUserProfileInfo(),
		...overrides,
	};
}

export function createAdminUserPage(
	items: AdminUserInfo[] = [],
	overrides: Partial<AdminUserPage> = {},
): AdminUserPage {
	return {
		items,
		limit: 20,
		total: items.length,
		...overrides,
	};
}

export function createAuthSession(
	overrides: Partial<AuthSessionInfo> = {},
): AuthSessionInfo {
	const now = Date.now();
	return {
		id: "session-1",
		ip_address: "127.0.0.1",
		user_agent: "TestBrowser/1.0",
		is_current: false,
		revoked: false,
		created_at: new Date(now - 60_000).toISOString(),
		last_seen_at: new Date(now - 30_000).toISOString(),
		refresh_expires_at: new Date(now + 86_400_000).toISOString(),
		...overrides,
	};
}

export function createAuthSessionPage(
	items: AuthSessionInfo[] = [],
	overrides: Partial<AuthSessionPage> = {},
): AuthSessionPage {
	return {
		items,
		limit: 50,
		total: items.length,
		...overrides,
	};
}

export function createPasskey(
	overrides: Partial<PasskeyInfo> = {},
): PasskeyInfo {
	return {
		id: 1,
		name: "MacBook",
		backup_eligible: true,
		backed_up: true,
		sign_count: 0,
		transports: ["internal"],
		last_used_at: null,
		created_at: "2026-06-15T00:00:00.000Z",
		updated_at: "2026-06-15T00:00:00.000Z",
		...overrides,
	};
}

export function createPasskeyPage(
	items: PasskeyInfo[] = [],
	overrides: Partial<PasskeyPage> = {},
): PasskeyPage {
	return {
		items,
		limit: 20,
		total: items.length,
		...overrides,
	};
}

export function createUserBan(
	overrides: Partial<UserBanInfo> = {},
): UserBanInfo {
	return {
		id: 11,
		user_id: 7,
		created_by_user_id: 1,
		scopes: ["com.astercosm.yggdrasil.capability.texture.upload"],
		reason: "upload abuse",
		public_reason: "visible reason",
		admin_note: "internal note",
		revoke_note: null,
		revoked_by_user_id: null,
		revoked_at: null,
		status: "active",
		effective: true,
		effective_status: "active",
		starts_at: "2026-06-18T00:00:00.000Z",
		expires_at: null,
		created_at: "2026-06-18T00:00:00.000Z",
		updated_at: "2026-06-18T00:00:00.000Z",
		...overrides,
	};
}

export function createUserBanEvent(
	overrides: Partial<UserBanEventInfo> = {},
): UserBanEventInfo {
	return {
		id: 21,
		ban_id: 11,
		actor_user_id: 1,
		event_type: "created",
		note: "upload abuse",
		previous_status: null,
		previous_scopes: null,
		previous_expires_at: null,
		next_status: "active",
		next_scopes: ["com.astercosm.yggdrasil.capability.texture.upload"],
		next_expires_at: null,
		created_at: "2026-06-18T00:00:00.000Z",
		...overrides,
	};
}

export function createUserBanPage(
	items: UserBanInfo[] = [createUserBan()],
	overrides: Partial<AdminUserBanPage> = {},
): AdminUserBanPage {
	return {
		items,
		limit: 50,
		total: items.length,
		...overrides,
	};
}

export function createAdminInvitation(
	overrides: Partial<AdminUserInvitationInfo> = {},
): AdminUserInvitationInfo {
	return {
		id: 42,
		email: "invitee@example.com",
		status: "pending",
		invitation_url: "http://localhost:3300/invite/token",
		invited_by: 1,
		mail_queued: true,
		accepted_at: null,
		accepted_user_id: null,
		revoked_at: null,
		expires_at: "2026-06-25T00:00:00.000Z",
		created_at: "2026-06-18T00:00:00.000Z",
		updated_at: "2026-06-18T00:00:00.000Z",
		...overrides,
	};
}

export function createAdminInvitationPage(
	items: AdminUserInvitationInfo[] = [],
	overrides: Partial<AdminUserInvitationPage> = {},
): AdminUserInvitationPage {
	return {
		items,
		limit: 20,
		total: items.length,
		...overrides,
	};
}

export function createTask(overrides: Partial<TaskInfo> = {}): TaskInfo {
	const now = Date.now();
	return {
		id: 1,
		kind: "system_runtime",
		display_name: "Runtime cleanup",
		status: "succeeded",
		payload: { kind: "system_runtime", task_name: "system_cleanup" },
		steps: [],
		progress_current: 0,
		progress_total: 0,
		progress_percent: 100,
		attempt_count: 1,
		max_attempts: 3,
		can_retry: false,
		creator: null,
		creator_user_id: null,
		started_at: new Date(now - 30_000).toISOString(),
		finished_at: new Date(now - 10_000).toISOString(),
		lease_expires_at: null,
		last_error: null,
		status_text: null,
		result: null,
		presentation: null,
		created_at: new Date(now - 60_000).toISOString(),
		updated_at: new Date(now - 10_000).toISOString(),
		expires_at: new Date(now + 86_400_000).toISOString(),
		...overrides,
	};
}

export function createTaskPage(
	items: TaskInfo[] = [],
	overrides: Partial<AdminTaskPage> = {},
): AdminTaskPage {
	return {
		items,
		limit: 20,
		total: items.length,
		...overrides,
	};
}

export function createAuditEntry(
	overrides: Partial<AuditLogEntry> = {},
): AuditLogEntry {
	return {
		id: 1,
		user_id: 7,
		action: "user_login",
		entity_type: "auth_session",
		entity_id: null,
		entity_name: null,
		details: null,
		ip_address: "127.0.0.1",
		user_agent: "TestBrowser/1.0",
		user: null,
		presentation: null,
		created_at: "2026-06-18T00:00:00.000Z",
		...overrides,
	};
}

export function createAuditLogPage(
	items: AuditLogEntry[] = [],
	overrides: Partial<AuditLogPage> = {},
): AuditLogPage {
	return {
		items,
		limit: 50,
		total: items.length,
		...overrides,
	};
}

export function createAccountAuditLogPage(
	items: AuditLogEntry[] = [],
	overrides: Partial<AccountAuditLogPage> = {},
): AccountAuditLogPage {
	return {
		items,
		limit: 20,
		total: items.length,
		...overrides,
	};
}
