const STORAGE_AREAS = {
	local: "local",
	session: "session",
} as const;

type StorageArea = (typeof STORAGE_AREAS)[keyof typeof STORAGE_AREAS];

export const STORAGE_KEYS = {
	authExpiresAt: "asteryggdrasil-auth-expires-at",
	cachedFrontendConfig: "asteryggdrasil-cached-frontend-config:v1",
	cachedUser: "asteryggdrasil-cached-user",
	desktopSidebarExpanded: "asteryggdrasil-desktop-sidebar-expanded",
	legacyAccessToken: "asteryggdrasil-access-token",
	legacyRefreshToken: "asteryggdrasil-refresh-token",
	legacyUser: "asteryggdrasil-user",
	refreshEvent: "aster-auth-refresh-event",
	refreshLock: "aster-auth-refresh-lock",
	themeMode: "asteryggdrasil-theme-mode",
} as const;

function getStorage(area: StorageArea): Storage | null {
	if (typeof window === "undefined") return null;

	try {
		return area === STORAGE_AREAS.local
			? window.localStorage
			: window.sessionStorage;
	} catch {
		return null;
	}
}

export function readStorageItem(area: StorageArea, key: string): string | null {
	try {
		return getStorage(area)?.getItem(key) ?? null;
	} catch {
		return null;
	}
}

export function writeStorageItem(
	area: StorageArea,
	key: string,
	value: string,
): boolean {
	try {
		const storage = getStorage(area);
		if (!storage) return false;
		storage.setItem(key, value);
		return true;
	} catch {
		return false;
	}
}

export function removeStorageItem(area: StorageArea, key: string): boolean {
	try {
		const storage = getStorage(area);
		if (!storage) return false;
		storage.removeItem(key);
		return true;
	} catch {
		return false;
	}
}

export function readJsonStorageItem<T>(
	area: StorageArea,
	key: string,
): T | null {
	const raw = readStorageItem(area, key);
	if (!raw) return null;

	try {
		return JSON.parse(raw) as T;
	} catch {
		return null;
	}
}

export function writeJsonStorageItem(
	area: StorageArea,
	key: string,
	value: unknown,
): boolean {
	return writeStorageItem(area, key, JSON.stringify(value));
}
