import { create } from "zustand";
import {
	type AppliedBranding,
	applyBranding,
	DEFAULT_BRANDING,
	resolveBranding,
} from "@/lib/branding";
import {
	readStorageItem,
	removeStorageItem,
	STORAGE_KEYS,
	writeJsonStorageItem,
} from "@/lib/storage";
import { frontendConfigService } from "@/services/frontendConfigService";
import type {
	PublicBranding,
	PublicCaptchaConfig,
	PublicFrontendConfig,
	PublicYggdrasilConfig,
} from "@/types/api";

const FRONTEND_CONFIG_REVALIDATE_INTERVAL_MS = 30_000;
const DEFAULT_CAPTCHA_CONFIG: PublicCaptchaConfig = {
	enabled: false,
	invitation_accept_required: false,
	login_required: false,
	register_activation_resend_required: false,
	register_required: false,
};

interface CachedFrontendConfigPayload {
	config: PublicFrontendConfig;
	cachedAt?: number;
}

interface FrontendConfigState {
	allowUserRegistration: boolean;
	passkeyLoginEnabled: boolean;
	branding: AppliedBranding;
	captcha: PublicCaptchaConfig;
	config: PublicFrontendConfig | null;
	isLoaded: boolean;
	yggdrasil: PublicYggdrasilConfig | null;
	invalidate: () => void;
	load: (options?: { force?: boolean }) => Promise<void>;
}

let inFlightLoad: Promise<void> | null = null;
let lastRevalidationAttemptAt = 0;

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
	return (
		Array.isArray(value) && value.every((item) => typeof item === "string")
	);
}

function publicPasskeyLoginEnabled(branding: PublicBranding | null): boolean {
	if (!branding || !isRecord(branding)) return true;
	return branding.passkey_login_enabled !== false;
}

function isPublicBranding(value: unknown): value is PublicBranding {
	return (
		isRecord(value) &&
		typeof value.allow_user_registration === "boolean" &&
		(value.passkey_login_enabled === undefined ||
			typeof value.passkey_login_enabled === "boolean") &&
		typeof value.description === "string" &&
		typeof value.favicon_url === "string" &&
		isStringArray(value.site_urls) &&
		typeof value.title === "string" &&
		typeof value.wordmark_dark_url === "string" &&
		typeof value.wordmark_light_url === "string"
	);
}

function isPublicYggdrasilConfig(
	value: unknown,
): value is PublicYggdrasilConfig {
	return (
		isRecord(value) &&
		typeof value.allow_cape_upload === "boolean" &&
		typeof value.allow_profile_name_login === "boolean" &&
		typeof value.allow_skin_upload === "boolean" &&
		typeof value.max_texture_pixels === "number" &&
		Number.isFinite(value.max_texture_pixels) &&
		value.max_texture_pixels > 0 &&
		typeof value.max_texture_upload_bytes === "number" &&
		Number.isFinite(value.max_texture_upload_bytes) &&
		value.max_texture_upload_bytes > 0 &&
		isStringArray(value.public_base_urls) &&
		typeof value.server_name === "string" &&
		isStringArray(value.skin_domains)
	);
}

function isPublicCaptchaConfig(value: unknown): value is PublicCaptchaConfig {
	return (
		isRecord(value) &&
		typeof value.enabled === "boolean" &&
		typeof value.login_required === "boolean" &&
		typeof value.register_required === "boolean" &&
		typeof value.invitation_accept_required === "boolean" &&
		typeof value.register_activation_resend_required === "boolean"
	);
}

function isFrontendConfig(value: unknown): value is PublicFrontendConfig {
	return (
		isRecord(value) &&
		typeof value.version === "number" &&
		Number.isFinite(value.version) &&
		isPublicBranding(value.branding) &&
		(value.captcha === undefined || isPublicCaptchaConfig(value.captcha)) &&
		isPublicYggdrasilConfig(value.yggdrasil)
	);
}

function normalizeFrontendConfig(
	config: PublicFrontendConfig,
): PublicFrontendConfig {
	return {
		...config,
		captcha: isPublicCaptchaConfig(config.captcha)
			? config.captcha
			: DEFAULT_CAPTCHA_CONFIG,
	};
}

function readCachedFrontendConfig(): CachedFrontendConfigPayload | null {
	try {
		const raw = readStorageItem("local", STORAGE_KEYS.cachedFrontendConfig);
		if (!raw) return null;

		const parsed = JSON.parse(raw) as CachedFrontendConfigPayload | null;
		if (!isRecord(parsed) || !isFrontendConfig(parsed.config)) {
			removeStorageItem("local", STORAGE_KEYS.cachedFrontendConfig);
			return null;
		}

		return {
			config: normalizeFrontendConfig(parsed.config),
			cachedAt:
				typeof parsed.cachedAt === "number" && Number.isFinite(parsed.cachedAt)
					? parsed.cachedAt
					: 0,
		};
	} catch {
		removeStorageItem("local", STORAGE_KEYS.cachedFrontendConfig);
		return null;
	}
}

function writeCachedFrontendConfig(config: PublicFrontendConfig) {
	writeJsonStorageItem("local", STORAGE_KEYS.cachedFrontendConfig, {
		config,
		cachedAt: Date.now(),
	} satisfies CachedFrontendConfigPayload);
}

function clearCachedFrontendConfig() {
	removeStorageItem("local", STORAGE_KEYS.cachedFrontendConfig);
}

function applyFrontendConfig(config: PublicFrontendConfig) {
	const normalizedConfig = normalizeFrontendConfig(config);
	const branding = resolveBranding(normalizedConfig.branding);
	applyBranding(branding);
	return {
		allowUserRegistration: normalizedConfig.branding.allow_user_registration,
		passkeyLoginEnabled: publicPasskeyLoginEnabled(normalizedConfig.branding),
		branding,
		captcha: normalizedConfig.captcha,
		config: normalizedConfig,
		isLoaded: true,
		yggdrasil: normalizedConfig.yggdrasil,
	};
}

function fallbackState() {
	applyBranding(DEFAULT_BRANDING);
	return {
		allowUserRegistration: true,
		passkeyLoginEnabled: true,
		branding: DEFAULT_BRANDING,
		captcha: DEFAULT_CAPTCHA_CONFIG,
		config: null,
		isLoaded: true,
		yggdrasil: null,
	};
}

function shouldSkipRevalidation(force: boolean, isLoaded: boolean) {
	if (force || !isLoaded) return false;
	return (
		Date.now() - lastRevalidationAttemptAt <
		FRONTEND_CONFIG_REVALIDATE_INTERVAL_MS
	);
}

const initialCachedPayload = readCachedFrontendConfig();
const initialCachedConfig = initialCachedPayload?.config ?? null;
const initialBranding = resolveBranding(initialCachedConfig?.branding ?? null);

export const useFrontendConfigStore = create<FrontendConfigState>(
	(set, get) => ({
		allowUserRegistration:
			initialCachedConfig?.branding.allow_user_registration ?? true,
		passkeyLoginEnabled: publicPasskeyLoginEnabled(
			initialCachedConfig?.branding ?? null,
		),
		branding: initialBranding,
		captcha: initialCachedConfig?.captcha ?? DEFAULT_CAPTCHA_CONFIG,
		config: initialCachedConfig,
		isLoaded: initialCachedConfig !== null,
		yggdrasil: initialCachedConfig?.yggdrasil ?? null,

		invalidate: () => {
			clearCachedFrontendConfig();
			lastRevalidationAttemptAt = 0;
			set({
				allowUserRegistration: true,
				passkeyLoginEnabled: true,
				branding: DEFAULT_BRANDING,
				captcha: DEFAULT_CAPTCHA_CONFIG,
				config: null,
				isLoaded: false,
				yggdrasil: null,
			});
		},

		load: async ({ force = false } = {}) => {
			if (shouldSkipRevalidation(force, get().isLoaded)) return;
			if (inFlightLoad) return inFlightLoad;

			inFlightLoad = (async () => {
				lastRevalidationAttemptAt = Date.now();
				try {
					const config = await frontendConfigService.get();
					if (!isFrontendConfig(config)) {
						throw new Error("invalid frontend config response");
					}
					writeCachedFrontendConfig(config);
					set(applyFrontendConfig(config));
				} catch (error) {
					console.warn(
						"frontend config bootstrap failed, using cached/defaults",
						error,
					);
					if (get().isLoaded) return;
					set(fallbackState());
				} finally {
					inFlightLoad = null;
				}
			})();

			return inFlightLoad;
		},
	}),
);

export function initFrontendConfigRuntime() {
	if (typeof window === "undefined") return;
	applyBranding(initialBranding);
}
