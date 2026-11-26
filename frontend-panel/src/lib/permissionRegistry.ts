import type { TFunction } from "i18next";
import { useCallback, useEffect, useState } from "react";
import type { useTranslation } from "react-i18next";
import { i18next } from "@/i18n";
import { permissionService } from "@/services/permissionService";
import type {
	AdminPermissionInfo,
	PermissionId,
	PermissionKind,
} from "@/types/api";

let cachedPermissions: AdminPermissionInfo[] | null = null;
let pendingLoad: Promise<LoadResult> | null = null;
let loadGeneration = 0;
let languageTick = 0;
let cachedBackendT: TFunction | null = null;

type LoadResult =
	| { ok: true; permissions: AdminPermissionInfo[] }
	| { ok: false };

/**
 * Dedicated namespace holding the backend message map verbatim.
 *
 * Keys stay flat and are looked up with an empty key separator (probed
 * against i18next 27: flat keys, prefix pairs, and prototype-inherited
 * segments all resolve correctly this way), so no nested tree is built and
 * the installer cannot corrupt the store.
 */
export const PERMISSION_LOCALIZATION_NAMESPACE = "permissions_backend";

const FORBIDDEN_MESSAGE_KEY_SEGMENTS = new Set([
	"__proto__",
	"constructor",
	"prototype",
]);

/**
 * Installs the backend message map verbatim under the dedicated namespace.
 * Forbidden segments are dropped as defense in depth (the backend rejects
 * them at construction); the fixed translators look keys up flat.
 */
export function installPermissionMessages(
	locale: string,
	messages: Record<string, string>,
): void {
	const safe: Record<string, string> = {};
	for (const [key, value] of Object.entries(messages)) {
		const forbidden = key
			.split(".")
			.some((segment) => FORBIDDEN_MESSAGE_KEY_SEGMENTS.has(segment));
		if (!forbidden) {
			safe[key] = value;
		}
	}
	i18next.addResourceBundle(
		locale,
		PERMISSION_LOCALIZATION_NAMESPACE,
		safe,
		false,
		true,
	);
}

/**
 * Fixed translator bound to the backend namespace with flat-key lookup;
 * created once and refreshed when the language changes.
 */
function backendTranslator(): TFunction {
	if (!cachedBackendT) {
		cachedBackendT = i18next.getFixedT(null, PERMISSION_LOCALIZATION_NAMESPACE);
	}
	return cachedBackendT;
}

function activeLocale(): string {
	return i18next.resolvedLanguage ?? i18next.language ?? "en";
}

const cacheListeners = new Set<() => void>();

function notifyPermissionsUpdated(): void {
	for (const listener of cacheListeners) listener();
}

/**
 * Loads the registry once per process, distinguishing a genuinely empty
 * registry from a failed request; failures are never cached, so a retry
 * always reaches the network again. Descriptors and the localized label
 * bundle arrive together, and the bundle is installed into i18n before the
 * result resolves so the first render already has translated labels.
 *
 * Every invalidation (language switch, retry after failure) bumps the
 * generation; an in-flight request from a stale generation discards its
 * result entirely, so it can neither install stale messages, repopulate the
 * cache, nor clear a newer pending load.
 */
function loadPermissions(): Promise<LoadResult> {
	if (cachedPermissions) {
		return Promise.resolve({ ok: true, permissions: cachedPermissions });
	}
	if (pendingLoad) return pendingLoad;
	const startedGeneration = loadGeneration;
	const locale = activeLocale();
	const current: Promise<LoadResult> = (async () => {
		try {
			const [list, bundle] = await Promise.all([
				permissionService.list(),
				permissionService.localizations(locale),
			]);
			if (loadGeneration !== startedGeneration) {
				return { ok: false } as LoadResult;
			}
			// Install at the tier the backend resolved (its parent-tag
			// fallback already picked the best variant); the i18n language
			// hierarchy (en-US -> en) resolves into that tier on lookup.
			installPermissionMessages(bundle.resolved_locale, bundle.messages);
			cachedPermissions ??= list.permissions;
			notifyPermissionsUpdated();
			return { ok: true, permissions: cachedPermissions } as LoadResult;
		} catch {
			// Registry introspection is display-only; a failed request must
			// not read as "no permissions are registered".
			return { ok: false } as LoadResult;
		}
	})();
	pendingLoad = current;
	// Release the pending slot only when this exact request still owns it
	// and no invalidation has superseded it: an interleaved retry's failure
	// must not discard a newer request's slot, and a stale load leaves the
	// invalidation's cleanup intact.
	current.finally(() => {
		if (loadGeneration === startedGeneration && pendingLoad === current) {
			pendingLoad = null;
		}
	});
	return current;
}

export type RegistryPermissionStatus = "loading" | "error" | "ready";

export type RegistryPermissions = {
	/** Descriptors of the requested kind; empty while loading or on error. */
	permissions: AdminPermissionInfo[];
	/** Distinguishes an in-flight or failed load from an empty registry. */
	status: RegistryPermissionStatus;
	/** Retries the registry request after a failure. */
	retry: () => void;
};

/**
 * Loads the registered permissions of one semantic kind from the server
 * registry. Synchronous callers (static nav matrices) keep using the
 * built-in constant lists; this hook is for editors and label rendering.
 */
export function useRegistryPermissions(
	kind: PermissionKind,
): RegistryPermissions {
	const [attempt, setAttempt] = useState(0);
	const [tick, setTick] = useState(languageTick);
	const [permissions, setPermissions] = useState<AdminPermissionInfo[]>(
		() => cachedPermissions?.filter((item) => item.kind === kind) ?? [],
	);
	const [status, setStatus] = useState<RegistryPermissionStatus>(() =>
		cachedPermissions ? "ready" : "loading",
	);

	// The module-level listener bumps languageTick on a language switch;
	// syncing it into state re-runs the load effect below.
	useEffect(() => {
		const onLanguageChanged = () => setTick(languageTick);
		i18next.on("languageChanged", onLanguageChanged);
		return () => {
			i18next.off("languageChanged", onLanguageChanged);
		};
	}, []);

	// biome-ignore lint/correctness/useExhaustiveDependencies: attempt and tick are intentional reload triggers.
	useEffect(() => {
		let cancelled = false;
		void loadPermissions().then((result) => {
			if (cancelled) return;
			if (result.ok) {
				setPermissions(result.permissions.filter((item) => item.kind === kind));
				setStatus("ready");
			} else {
				setStatus("error");
			}
		});
		return () => {
			cancelled = true;
		};
	}, [kind, attempt, tick]);

	// Another consumer's retry or the first successful load updates the
	// shared cache; mounted instances sync from it instead of staying stale.
	useEffect(() => {
		const listener = () => {
			if (!cachedPermissions) return;
			setPermissions(cachedPermissions.filter((item) => item.kind === kind));
			setStatus("ready");
		};
		cacheListeners.add(listener);
		return () => {
			cacheListeners.delete(listener);
		};
	}, [kind]);

	const retry = useCallback(() => {
		pendingLoad = null;
		setStatus("loading");
		setAttempt((current) => current + 1);
	}, []);

	return { permissions, status, retry };
}

// A language switch invalidates the process cache, discards in-flight loads
// from the previous generation, drops the cached backend translator, and
// bumps the tick that remounted consumers watch, so labels refresh in the
// new language without a page reload.
i18next.on("languageChanged", () => {
	cachedPermissions = null;
	pendingLoad = null;
	cachedBackendT = null;
	loadGeneration += 1;
	languageTick += 1;
});

/**
 * Resolves the display label for one descriptor: the backend-installed
 * bundle first (flat lookup, probed against i18next 27), then the static
 * bundle's key, then the server-provided English fallback.
 */
export function permissionLabel(
	permission: AdminPermissionInfo,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	const MISS = `\u0000${permission.label_i18n_key}`;
	const backend = backendTranslator()(permission.label_i18n_key, {
		keySeparator: "",
		nsSeparator: "\u0000",
		defaultValue: MISS,
	});
	if (backend !== MISS) {
		return backend;
	}
	const translated = t(permission.label_i18n_key);
	return translated === permission.label_i18n_key
		? permission.label
		: translated;
}

/**
 * Indexes descriptors by id so scope arrays can render labels without
 * maintaining a frontend identifier-to-display mapping.
 */
export function permissionLabels(
	permissions: AdminPermissionInfo[],
): Map<PermissionId, AdminPermissionInfo> {
	return new Map(permissions.map((item) => [item.id, item]));
}
