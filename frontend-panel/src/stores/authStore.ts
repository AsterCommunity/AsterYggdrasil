import { create } from "zustand";
import { firstAdminPathForScopes, OPERATOR_SCOPES } from "@/lib/operatorScopes";
import {
	readStorageItem,
	removeStorageItem,
	STORAGE_KEYS,
	writeJsonStorageItem,
	writeStorageItem,
} from "@/lib/storage";
import { authService } from "@/services/authService";
import {
	type ApiClientErrorCode,
	ApiError,
	formatUnknownError,
	isApiConnectionError,
} from "@/services/http";
import type {
	AdminUserInfo,
	AuthTokenResponse,
	AuthUserInfo,
	OperatorScope,
	RegisterResponse,
	UpdateAvatarSourceRequest,
	UpdateProfileRequest,
	UserProfileInfo,
} from "@/types/api";

type CachedAuthUser = Pick<
	AuthUserInfo,
	| "email"
	| "email_verified"
	| "id"
	| "must_change_password"
	| "operator_scopes"
	| "pending_email"
	| "profile"
	| "role"
	| "status"
	| "username"
>;

type AuthState = {
	user: AuthUserInfo | null;
	checking: boolean;
	error: string | null;
	errorCode: ApiClientErrorCode | null;
	expiresAt: number | null;
	isAuthStale: boolean;
	isAuthenticated: boolean;
	isAdmin: boolean;
	isOperator: boolean;
	operatorScopes: OperatorScope[];
	canAccessAdminShell: boolean;
	hasOperatorScope: (scope: OperatorScope) => boolean;
	hydrate: () => Promise<void>;
	setup: (
		username: string,
		email: string,
		password: string,
		publicSiteUrl?: string,
	) => Promise<void>;
	register: (
		username: string,
		email: string,
		password: string,
		captcha?: CaptchaSubmission,
	) => Promise<RegisterResponse>;
	login: (
		identifier: string,
		password: string,
		captcha?: CaptchaSubmission,
	) => Promise<void>;
	changePassword: (
		currentPassword: string,
		newPassword: string,
	) => Promise<void>;
	acceptInvitation: (
		token: string,
		username: string,
		password: string,
		captcha?: CaptchaSubmission,
	) => Promise<AuthUserInfo>;
	loginWithPasskey: (flowId: string, credential: unknown) => Promise<void>;
	refreshUser: () => Promise<void>;
	updateProfile: (data: UpdateProfileRequest) => Promise<UserProfileInfo>;
	setAvatarSource: (
		data: UpdateAvatarSourceRequest,
	) => Promise<UserProfileInfo>;
	uploadAvatar: (file: File) => Promise<UserProfileInfo>;
	syncCurrentUserFromAdminUser: (user: AdminUserInfo) => void;
	refresh: () => Promise<void>;
	logout: () => Promise<void>;
	clear: () => void;
};

type CaptchaSubmission = {
	challengeId: string;
	answer: string;
};

let inFlightHydrate: Promise<void> | null = null;

function defaultUserProfile(): UserProfileInfo {
	return {
		display_name: null,
		avatar: {
			source: "none",
			url_1024: null,
			url_512: null,
			version: 0,
		},
	};
}

function sanitizeCachedUser(value: unknown): CachedAuthUser | null {
	if (!value || typeof value !== "object") return null;
	const source = value as Partial<AuthUserInfo>;
	if (
		typeof source.id !== "number" ||
		typeof source.username !== "string" ||
		typeof source.role !== "string" ||
		typeof source.status !== "string"
	) {
		return null;
	}
	return {
		email: typeof source.email === "string" ? source.email : "",
		email_verified:
			typeof source.email_verified === "boolean"
				? source.email_verified
				: false,
		id: source.id,
		must_change_password:
			typeof source.must_change_password === "boolean"
				? source.must_change_password
				: false,
		pending_email:
			typeof source.pending_email === "string" ? source.pending_email : null,
		operator_scopes: Array.isArray(source.operator_scopes)
			? (source.operator_scopes.filter(
					(value): value is OperatorScope =>
						typeof value === "string" &&
						OPERATOR_SCOPES.includes(value as OperatorScope),
				) as OperatorScope[])
			: [],
		profile: source.profile ?? defaultUserProfile(),
		username: source.username,
		role: source.role,
		status: source.status,
	} as CachedAuthUser;
}

function cachedUserToAuthUser(user: CachedAuthUser): AuthUserInfo {
	return {
		...user,
		profile: user.profile ?? defaultUserProfile(),
	};
}

function readStoredUser(): AuthUserInfo | null {
	try {
		const raw =
			readStorageItem("local", STORAGE_KEYS.cachedUser) ??
			readStorageItem("local", STORAGE_KEYS.legacyUser);
		if (!raw) return null;

		const cached = sanitizeCachedUser(JSON.parse(raw));
		if (!cached) {
			removeStorageItem("local", STORAGE_KEYS.cachedUser);
			removeStorageItem("local", STORAGE_KEYS.legacyUser);
			return null;
		}

		writeJsonStorageItem("local", STORAGE_KEYS.cachedUser, cached);
		removeStorageItem("local", STORAGE_KEYS.legacyUser);
		return cachedUserToAuthUser(cached);
	} catch {
		return null;
	}
}

function persistUser(user: AuthUserInfo | null) {
	try {
		const cached = sanitizeCachedUser(user);
		if (cached) {
			writeJsonStorageItem("local", STORAGE_KEYS.cachedUser, cached);
			removeStorageItem("local", STORAGE_KEYS.legacyUser);
			return;
		}
		removeStorageItem("local", STORAGE_KEYS.cachedUser);
		removeStorageItem("local", STORAGE_KEYS.legacyUser);
	} catch {
		// Storage can be unavailable in private contexts; auth still relies on cookies.
	}
}

function readStoredExpiresAt(): number | null {
	try {
		const raw = readStorageItem("session", STORAGE_KEYS.authExpiresAt);
		if (!raw) return null;
		const expiresAt = Number(raw);
		if (!Number.isFinite(expiresAt) || expiresAt <= Date.now()) {
			removeStorageItem("session", STORAGE_KEYS.authExpiresAt);
			return null;
		}
		return expiresAt;
	} catch {
		return null;
	}
}

function persistExpiresAt(expiresAt: number | null) {
	try {
		if (expiresAt === null) {
			removeStorageItem("session", STORAGE_KEYS.authExpiresAt);
			return;
		}
		writeStorageItem("session", STORAGE_KEYS.authExpiresAt, String(expiresAt));
	} catch {
		// Storage failures should not break cookie-backed auth.
	}
}

function expiresAtFromToken(
	response: Pick<AuthTokenResponse, "expires_in">,
): number | null {
	const expiresIn = Number(response.expires_in);
	if (!Number.isFinite(expiresIn) || expiresIn <= 0) {
		return null;
	}
	return Date.now() + expiresIn * 1000;
}

function persistSession(
	response: Pick<AuthTokenResponse, "expires_in">,
): number | null {
	const expiresAt = expiresAtFromToken(response);
	persistExpiresAt(expiresAt);
	return expiresAt;
}

function clearPersistedAuth() {
	clearLegacyTokenStorage();
	persistUser(null);
	persistExpiresAt(null);
}

function authStateFromUser(user: AuthUserInfo | null) {
	return {
		user,
		...deriveAuthFlags(user),
	};
}

function setAuthenticatedState(
	set: (state: Partial<AuthState>) => void,
	user: AuthUserInfo,
	expiresAt: number | null,
) {
	persistUser(user);
	if (expiresAt !== null) {
		persistExpiresAt(expiresAt);
	}
	set({
		...authStateFromUser(user),
		checking: false,
		error: null,
		errorCode: null,
		expiresAt,
		isAuthStale: false,
	});
}

async function syncUserAfterTokenResponse(
	set: (state: Partial<AuthState>) => void,
	response: Pick<AuthTokenResponse, "expires_in">,
) {
	const expiresAt = persistSession(response);
	const user = await authService.me();
	setAuthenticatedState(set, user, expiresAt);
}

function syncProfile(
	set: (state: Partial<AuthState>) => void,
	get: () => AuthState,
	profile: UserProfileInfo,
) {
	const currentUser = get().user;
	if (!currentUser) return;
	setAuthenticatedState(
		set,
		{ ...currentUser, profile },
		get().expiresAt ?? readStoredExpiresAt(),
	);
}

function syncAdminUser(
	set: (state: Partial<AuthState>) => void,
	get: () => AuthState,
	adminUser: AdminUserInfo,
) {
	const currentUser = get().user;
	if (!currentUser || currentUser.id !== adminUser.id) return;
	setAuthenticatedState(
		set,
		{
			...currentUser,
			email: adminUser.email,
			email_verified: Boolean(adminUser.email_verified_at),
			pending_email: adminUser.pending_email,
			profile: adminUser.profile,
			must_change_password: adminUser.must_change_password,
			operator_scopes: adminUser.operator_scopes,
			role: adminUser.role,
			status: adminUser.status,
			username: adminUser.username,
		},
		get().expiresAt ?? readStoredExpiresAt(),
	);
}

function clearLegacyTokenStorage() {
	try {
		removeStorageItem("local", STORAGE_KEYS.legacyAccessToken);
		removeStorageItem("local", STORAGE_KEYS.legacyRefreshToken);
	} catch {
		// ignore storage failures
	}
}

function deriveAuthFlags(user: AuthUserInfo | null) {
	const operatorScopes = user?.operator_scopes ?? [];
	const isAdmin = user?.role === "admin";
	const isOperator = user?.role === "operator";
	return {
		isAuthenticated: Boolean(user),
		isAdmin,
		isOperator,
		operatorScopes,
		canAccessAdminShell:
			isAdmin ||
			(isOperator && firstAdminPathForScopes(operatorScopes) != null),
	};
}

function apiErrorCode(error: unknown): ApiClientErrorCode | null {
	return error instanceof ApiError ? error.code : null;
}

const initialUser = readStoredUser();
const initialFlags = deriveAuthFlags(initialUser);
const initialExpiresAt = readStoredExpiresAt();
clearLegacyTokenStorage();

export const useAuthStore = create<AuthState>((set, get) => ({
	user: initialUser,
	checking: true,
	error: null,
	errorCode: null,
	expiresAt: initialExpiresAt,
	isAuthStale: Boolean(initialUser),
	isAuthenticated: initialFlags.isAuthenticated,
	isAdmin: initialFlags.isAdmin,
	isOperator: initialFlags.isOperator,
	operatorScopes: initialFlags.operatorScopes,
	canAccessAdminShell: initialFlags.canAccessAdminShell,
	hasOperatorScope(scope) {
		const state = get();
		return state.isAdmin || state.operatorScopes.includes(scope);
	},
	async hydrate() {
		if (inFlightHydrate) return inFlightHydrate;

		inFlightHydrate = (async () => {
			set({ checking: true, error: null, errorCode: null });
			try {
				const user = await authService.me();
				setAuthenticatedState(
					set,
					user,
					get().expiresAt ?? readStoredExpiresAt(),
				);
			} catch (error) {
				const errorCode = apiErrorCode(error);
				if (isApiConnectionError(error)) {
					const currentUser = get().user;
					set({
						...authStateFromUser(currentUser),
						checking: false,
						error: formatUnknownError(error),
						errorCode,
						expiresAt: get().expiresAt ?? readStoredExpiresAt(),
						isAuthStale: true,
					});
					return;
				}

				clearPersistedAuth();
				set({
					...authStateFromUser(null),
					checking: false,
					error: formatUnknownError(error),
					errorCode,
					expiresAt: null,
					isAuthStale: false,
				});
			} finally {
				inFlightHydrate = null;
			}
		})();

		return inFlightHydrate;
	},
	async setup(username, email, password, publicSiteUrl) {
		const response = await authService.setup({
			username,
			email,
			password,
			public_site_url: publicSiteUrl,
		});
		await syncUserAfterTokenResponse(set, response);
	},
	async register(username, email, password, captcha) {
		const response = await authService.register({
			username,
			email,
			password,
			captcha_challenge_id: captcha?.challengeId,
			captcha_answer: captcha?.answer,
		});
		if (response.requires_activation) {
			clearPersistedAuth();
			set({
				...authStateFromUser(null),
				checking: false,
				error: null,
				errorCode: null,
				expiresAt: null,
				isAuthStale: false,
			});
			return response;
		}
		await syncUserAfterTokenResponse(set, response);
		return response;
	},
	async login(identifier, password, captcha) {
		const response = await authService.login({
			identifier,
			password,
			captcha_challenge_id: captcha?.challengeId,
			captcha_answer: captcha?.answer,
		});
		await syncUserAfterTokenResponse(set, response);
	},
	async changePassword(currentPassword, newPassword) {
		const response = await authService.changePassword({
			current_password: currentPassword,
			new_password: newPassword,
		});
		await syncUserAfterTokenResponse(set, response);
	},
	async acceptInvitation(token, username, password, captcha) {
		return await authService.acceptInvitation(token, {
			username,
			password,
			captcha_challenge_id: captcha?.challengeId,
			captcha_answer: captcha?.answer,
		});
	},
	async loginWithPasskey(flowId, credential) {
		const response = await authService.finishPasskeyLogin(flowId, credential);
		await syncUserAfterTokenResponse(set, response);
	},
	async refreshUser() {
		const user = await authService.me();
		setAuthenticatedState(set, user, get().expiresAt ?? readStoredExpiresAt());
	},
	async updateProfile(data) {
		const profile = await authService.updateProfile(data);
		syncProfile(set, get, profile);
		return profile;
	},
	async setAvatarSource(data) {
		const profile = await authService.setAvatarSource(data);
		syncProfile(set, get, profile);
		return profile;
	},
	async uploadAvatar(file) {
		const profile = await authService.uploadAvatar(file);
		syncProfile(set, get, profile);
		return profile;
	},
	syncCurrentUserFromAdminUser(user) {
		syncAdminUser(set, get, user);
	},
	async refresh() {
		const response = await authService.refresh();
		const expiresAt = persistSession(response);
		const user = await authService.me();
		setAuthenticatedState(set, user, expiresAt);
	},
	async logout() {
		try {
			await authService.logout();
		} finally {
			get().clear();
		}
	},
	clear() {
		clearPersistedAuth();
		set({
			...authStateFromUser(null),
			error: null,
			errorCode: null,
			checking: false,
			expiresAt: null,
			isAuthStale: false,
		});
	},
}));
