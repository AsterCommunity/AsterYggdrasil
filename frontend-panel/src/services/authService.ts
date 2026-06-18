import { withQuery } from "@/lib/query";
import type {
	AuthSessionInfo,
	AuthSessionPage,
	AuthSessionQuery,
	AuthTokenResponse,
	AuthUserInfo,
	ChangePasswordRequest,
	CheckResp,
	LoginRequest,
	OperationData,
	OperationPath,
	OperationRequestBody,
	PasskeyInfo,
	PasskeyLoginStartRequest,
	PasskeyLoginStartResponse,
	PasskeyPage,
	PasskeyQuery,
	PasskeyRegisterFinishRequest,
	PasskeyRegisterStartRequest,
	PasskeyRegisterStartResponse,
	PasswordResetConfirmRequest,
	PasswordResetRequest,
	PatchPasskeyRequest,
	PublicUserInvitationInfo,
	RegisterRequest,
	RegisterResponse,
	RequestEmailChangeRequest,
	RevokeOtherAuthSessionsResponse,
	SetupRequest,
	UpdateAvatarSourceRequest,
	UpdateProfileRequest,
	UserProfileInfo,
} from "@/types/api";
import { api } from "./http";

type RevokeAuthSessionPath = OperationPath<"revoke_auth_session">;

type CachedRequestOptions = {
	force?: boolean;
};

export type { PasskeyInfo };

let cachedMe: AuthUserInfo | null = null;
let pendingMeRequest: Promise<AuthUserInfo> | null = null;
let meCacheSerial = 0;

let cachedSessions: AuthSessionInfo[] | null = null;
let pendingSessionsRequest: Promise<AuthSessionInfo[]> | null = null;
let sessionsCacheSerial = 0;

let cachedPasskeys: PasskeyInfo[] | null = null;
let pendingPasskeysRequest: Promise<PasskeyInfo[]> | null = null;
let passkeysCacheSerial = 0;

function cloneProfile(profile: UserProfileInfo): UserProfileInfo {
	return {
		...profile,
		avatar: { ...profile.avatar },
	};
}

function cloneUser(user: AuthUserInfo): AuthUserInfo {
	return {
		...user,
		profile: user.profile ? cloneProfile(user.profile) : user.profile,
	};
}

function cloneSessions(sessions: AuthSessionInfo[]) {
	return sessions.map((session) => ({ ...session }));
}

function clonePasskeys(passkeys: PasskeyInfo[]) {
	return passkeys.map((passkey) => ({ ...passkey }));
}

function primeMeCache(user: AuthUserInfo) {
	cachedMe = cloneUser(user);
}

function primeSessionsCache(sessions: AuthSessionInfo[]) {
	cachedSessions = cloneSessions(sessions);
}

function primePasskeysCache(passkeys: PasskeyInfo[]) {
	cachedPasskeys = clonePasskeys(passkeys);
}

export function invalidateMeCache() {
	cachedMe = null;
	pendingMeRequest = null;
	meCacheSerial += 1;
}

export function invalidateSessionsCache() {
	cachedSessions = null;
	pendingSessionsRequest = null;
	sessionsCacheSerial += 1;
}

export function invalidatePasskeysCache() {
	cachedPasskeys = null;
	pendingPasskeysRequest = null;
	passkeysCacheSerial += 1;
}

export function invalidateAuthServiceCaches() {
	invalidateMeCache();
	invalidateSessionsCache();
	invalidatePasskeysCache();
}

function me(options?: CachedRequestOptions) {
	const force = options?.force ?? false;
	if (!force && cachedMe !== null) {
		return Promise.resolve(cloneUser(cachedMe));
	}
	if (!force && pendingMeRequest !== null) {
		return pendingMeRequest.then(cloneUser);
	}

	const requestSerial = ++meCacheSerial;
	const request = api
		.get<AuthUserInfo>("/auth/me")
		.then((user) => {
			if (requestSerial === meCacheSerial) {
				primeMeCache(user);
			}
			return cloneUser(user);
		})
		.finally(() => {
			if (pendingMeRequest === request) {
				pendingMeRequest = null;
			}
		});
	pendingMeRequest = request;
	return request.then(cloneUser);
}

function sessions(options?: CachedRequestOptions) {
	const force = options?.force ?? false;
	if (!force && cachedSessions !== null) {
		return Promise.resolve(cloneSessions(cachedSessions));
	}
	if (!force && pendingSessionsRequest !== null) {
		return pendingSessionsRequest.then(cloneSessions);
	}

	const requestSerial = ++sessionsCacheSerial;
	const request = api
		.get<AuthSessionPage>(withQuery("/auth/sessions", { limit: 50, offset: 0 }))
		.then((page) => {
			const nextSessions = page.items;
			if (requestSerial === sessionsCacheSerial) {
				primeSessionsCache(nextSessions);
			}
			return cloneSessions(nextSessions);
		})
		.finally(() => {
			if (pendingSessionsRequest === request) {
				pendingSessionsRequest = null;
			}
		});
	pendingSessionsRequest = request;
	return request.then(cloneSessions);
}

function sessionsPage(params: AuthSessionQuery = {}) {
	return api.get<AuthSessionPage>(withQuery("/auth/sessions", params));
}

function listPasskeys(options?: CachedRequestOptions) {
	const force = options?.force ?? false;
	if (!force && cachedPasskeys !== null) {
		return Promise.resolve(clonePasskeys(cachedPasskeys));
	}
	if (!force && pendingPasskeysRequest !== null) {
		return pendingPasskeysRequest.then(clonePasskeys);
	}

	const requestSerial = ++passkeysCacheSerial;
	const request = api
		.get<PasskeyPage>(withQuery("/auth/passkeys", { limit: 20, offset: 0 }))
		.then((page) => {
			const passkeys = page.items;
			if (requestSerial === passkeysCacheSerial) {
				primePasskeysCache(passkeys);
			}
			return clonePasskeys(passkeys);
		})
		.finally(() => {
			if (pendingPasskeysRequest === request) {
				pendingPasskeysRequest = null;
			}
		});
	pendingPasskeysRequest = request;
	return request.then(clonePasskeys);
}

function listPasskeysPage(params: PasskeyQuery = {}) {
	return api.get<PasskeyPage>(withQuery("/auth/passkeys", params));
}

function upsertCachedPasskey(passkey: PasskeyInfo) {
	pendingPasskeysRequest = null;
	passkeysCacheSerial += 1;
	if (cachedPasskeys === null) return;
	cachedPasskeys = [
		{ ...passkey },
		...cachedPasskeys.filter((item) => item.id !== passkey.id),
	];
}

function replaceCachedPasskey(passkey: PasskeyInfo) {
	pendingPasskeysRequest = null;
	passkeysCacheSerial += 1;
	if (cachedPasskeys === null) return;
	cachedPasskeys = cachedPasskeys.map((item) =>
		item.id === passkey.id ? { ...passkey } : item,
	);
}

function removeCachedPasskey(id: number) {
	pendingPasskeysRequest = null;
	passkeysCacheSerial += 1;
	if (cachedPasskeys === null) return;
	cachedPasskeys = cachedPasskeys.filter((item) => item.id !== id);
}

export const authService = {
	check: () => api.get<CheckResp>("/auth/check"),
	setup: async (data: SetupRequest) => {
		invalidateAuthServiceCaches();
		return api.post<
			AuthTokenResponse,
			OperationRequestBody<"setup_first_admin">
		>("/auth/setup", data);
	},
	register: async (data: RegisterRequest) => {
		invalidateAuthServiceCaches();
		return api.post<RegisterResponse, OperationRequestBody<"register">>(
			"/auth/register",
			data,
		);
	},
	resendRegisterActivation: (identifier: string) =>
		api.post<void, OperationRequestBody<"resend_register_activation">>(
			"/auth/register/resend",
			{ identifier },
		),
	requestPasswordReset: (data: PasswordResetRequest) =>
		api.post<
			OperationData<"request_password_reset">,
			OperationRequestBody<"request_password_reset">
		>("/auth/password/reset/request", data),
	confirmPasswordReset: (data: PasswordResetConfirmRequest) =>
		api.post<
			OperationData<"confirm_password_reset">,
			OperationRequestBody<"confirm_password_reset">
		>("/auth/password/reset/confirm", data),
	verifyInvitation: (token: string) =>
		api.get<PublicUserInvitationInfo>(
			`/auth/invitations/${encodeURIComponent(token)}`,
		),
	acceptInvitation: (
		token: string,
		data: OperationRequestBody<"accept_user_invitation">,
	) => {
		invalidateAuthServiceCaches();
		return api.post<
			AuthUserInfo,
			OperationRequestBody<"accept_user_invitation">
		>(`/auth/invitations/${encodeURIComponent(token)}/accept`, data);
	},
	login: async (data: LoginRequest) => {
		invalidateAuthServiceCaches();
		return api.post<AuthTokenResponse, OperationRequestBody<"login">>(
			"/auth/login",
			data,
		);
	},
	refresh: async () => {
		invalidateMeCache();
		return api.post<AuthTokenResponse>("/auth/refresh");
	},
	changePassword: async (data: ChangePasswordRequest) => {
		invalidateAuthServiceCaches();
		return api.put<AuthTokenResponse, ChangePasswordRequest>(
			"/auth/password",
			data,
		);
	},
	logout: async () => {
		invalidateAuthServiceCaches();
		return api.post<void>("/auth/logout");
	},
	me,
	requestEmailChange: async (data: RequestEmailChangeRequest) => {
		const user = await api.post<
			AuthUserInfo,
			OperationRequestBody<"request_email_change">
		>("/auth/email/change", data);
		primeMeCache(user);
		return cloneUser(user);
	},
	resendEmailChange: () =>
		api.post<OperationData<"resend_email_change">>("/auth/email/change/resend"),
	updateProfile: async (data: UpdateProfileRequest) => {
		const profile = await api.patch<UserProfileInfo, UpdateProfileRequest>(
			"/auth/profile",
			data,
		);
		invalidateMeCache();
		return profile;
	},
	setAvatarSource: async (data: UpdateAvatarSourceRequest) => {
		const profile = await api.put<UserProfileInfo, UpdateAvatarSourceRequest>(
			"/auth/profile/avatar/source",
			data,
		);
		invalidateMeCache();
		return profile;
	},
	uploadAvatar: async (file: File) => {
		const formData = new FormData();
		formData.append("file", file);
		const profile = await api.post<UserProfileInfo, FormData>(
			"/auth/profile/avatar/upload",
			formData,
		);
		invalidateMeCache();
		return profile;
	},
	sessions,
	sessionsPage,
	revokeSession: async (id: RevokeAuthSessionPath["id"]) => {
		await api.delete<void>(`/auth/sessions/${id}`);
		invalidateSessionsCache();
	},
	revokeOtherSessions: async () => {
		const response = await api.delete<RevokeOtherAuthSessionsResponse>(
			"/auth/sessions/others",
		);
		invalidateSessionsCache();
		return response;
	},
	listPasskeys,
	listPasskeysPage,
	startPasskeyRegistration: (data: PasskeyRegisterStartRequest) =>
		api.post<PasskeyRegisterStartResponse, PasskeyRegisterStartRequest>(
			"/auth/passkeys/register/start",
			data,
		),
	finishPasskeyRegistration: async (
		flowId: string,
		credential: unknown,
		name?: string | null,
	) => {
		const passkey = await api.post<PasskeyInfo, PasskeyRegisterFinishRequest>(
			"/auth/passkeys/register/finish",
			{
				flow_id: flowId,
				credential,
				name,
			},
		);
		upsertCachedPasskey(passkey);
		return passkey;
	},
	renamePasskey: async (id: number, data: PatchPasskeyRequest) => {
		const passkey = await api.patch<PasskeyInfo, PatchPasskeyRequest>(
			`/auth/passkeys/${id}`,
			data,
		);
		replaceCachedPasskey(passkey);
		return passkey;
	},
	deletePasskey: async (id: number) => {
		await api.delete<void>(`/auth/passkeys/${id}`);
		removeCachedPasskey(id);
	},
	startPasskeyLogin: (data: PasskeyLoginStartRequest = {}) =>
		api.post<PasskeyLoginStartResponse, PasskeyLoginStartRequest>(
			"/auth/passkeys/login/start",
			data,
		),
	finishPasskeyLogin: async (flowId: string, credential: unknown) => {
		invalidateAuthServiceCaches();
		return api.post<
			AuthTokenResponse,
			{ flow_id: string; credential: unknown }
		>("/auth/passkeys/login/finish", { flow_id: flowId, credential });
	},
};
