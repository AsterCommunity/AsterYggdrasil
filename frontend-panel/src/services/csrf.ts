export const DEFAULT_CSRF_COOKIE_NAME = "aster_yggdrasil_csrf";
export const DEFAULT_CSRF_HEADER_NAME = "x-aster-yggdrasil-csrf";
const CSRF_COOKIE_META_NAME = "asteryggdrasil-csrf-cookie-name";
const CSRF_HEADER_META_NAME = "asteryggdrasil-csrf-header-name";

type RuntimeCsrfConfig = {
	cookieName?: unknown;
	headerName?: unknown;
};

declare global {
	interface Window {
		__ASTER_YGGDRASIL_CSRF__?: RuntimeCsrfConfig;
	}
}

function configuredValue(value: unknown): string | null {
	if (typeof value !== "string") {
		return null;
	}

	const trimmed = value.trim();
	return trimmed.length > 0 ? trimmed : null;
}

function configuredMetaContent(name: string): string | null {
	if (typeof document === "undefined") {
		return null;
	}

	const element = document.querySelector<HTMLMetaElement>(
		`meta[name="${name}"]`,
	);
	return configuredValue(element?.content);
}

function runtimeCsrfConfig(): RuntimeCsrfConfig {
	if (typeof window === "undefined") {
		return {};
	}
	return window.__ASTER_YGGDRASIL_CSRF__ ?? {};
}

export function getCsrfCookieName(): string {
	return (
		configuredMetaContent(CSRF_COOKIE_META_NAME) ??
		configuredValue(runtimeCsrfConfig().cookieName) ??
		DEFAULT_CSRF_COOKIE_NAME
	);
}

export function getCsrfHeaderName(): string {
	return (
		configuredMetaContent(CSRF_HEADER_META_NAME) ??
		configuredValue(runtimeCsrfConfig().headerName) ??
		DEFAULT_CSRF_HEADER_NAME
	);
}

export function readCookie(name: string): string | null {
	if (typeof document === "undefined") {
		return null;
	}

	const encodedName = `${encodeURIComponent(name)}=`;
	for (const part of document.cookie.split(";")) {
		const trimmed = part.trim();
		if (!trimmed.startsWith(encodedName)) {
			continue;
		}

		const value = trimmed.slice(encodedName.length);
		return decodeURIComponent(value);
	}

	return null;
}

export function getCsrfToken(): string | null {
	return readCookie(getCsrfCookieName());
}
