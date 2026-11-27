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
let bundleLoaded = false;
let pendingLoad: Promise<LoadResult> | null = null;
let loadGeneration = 0;
let languageTick = 0;
let cachedBackendT: TFunction | null = null;

type LoadResult =
	| {
			ok: true;
			permissions: AdminPermissionInfo[];
			/** In-flight bundle install; resolves true when it landed. */
			bundlePending?: Promise<boolean>;
	  }
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
 * Fetches and installs the localization bundle for one locale. Best-effort
 * by design: a bundle failure must not block the descriptor flow, because
 * labels degrade to the static bundle and descriptor labels.
 *
 * The bundle binds to the generation it started under: a response that
 * lands after an invalidation installs nothing and never marks the bundle
 * loaded, so the current locale's own request stays authoritative.
 */
async function loadBundle(
	locale: string,
	startedGeneration: number,
): Promise<boolean> {
	try {
		const bundle = await permissionService.localizations(locale);
		if (loadGeneration !== startedGeneration) {
			return false;
		}
		installPermissionMessages(bundle.resolved_locale, bundle.messages);
		bundleLoaded = true;
		return true;
	} catch {
		return false;
	}
}

/**
 * Loads the registry once per process, distinguishing a genuinely empty
 * registry from a failed request; failures are never cached, so a retry
 * always reaches the network again. The descriptor list is the required
 * success path; the localization bundle is best-effort so a bundle failure
 * degrades labels instead of hiding the editor.
 *
 * Every invalidation (language switch, retry after failure) bumps the
 * generation; an in-flight request from a stale generation discards its
 * result entirely, so it can neither install stale messages, repopulate the
 * cache, nor clear a newer pending load.
 */
function loadPermissions(): Promise<LoadResult> {
	if (cachedPermissions) {
		if (bundleLoaded) {
			return Promise.resolve({ ok: true, permissions: cachedPermissions });
		}
		// Descriptors are cached but the bundle never arrived: retry it.
		const bundleRetry = loadBundle(activeLocale(), loadGeneration).then(
			(ok) => {
				if (ok) notifyPermissionsUpdated();
				return { ok: true, permissions: cachedPermissions } as LoadResult;
			},
		);
		return bundleRetry;
	}
	if (pendingLoad) return pendingLoad;
	const startedGeneration = loadGeneration;
	const locale = activeLocale();
	const current: Promise<LoadResult> = (async () => {
		try {
			const list = await permissionService.list();
			if (loadGeneration !== startedGeneration) {
				return { ok: false } as LoadResult;
			}
			cachedPermissions ??= list.permissions;
			notifyPermissionsUpdated();
			// The bundle is best-effort: publish the descriptors now and load
			// the bundle without blocking the editor. A hung bundle request
			// delays labels, never the switches themselves.
			const bundlePending = loadBundle(locale, startedGeneration);
			return {
				ok: true,
				permissions: cachedPermissions,
				bundlePending,
			} as LoadResult;
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

export type RegistryBundleStatus = "loading" | "error" | "ready";

export type RegistryPermissions = {
	/** Descriptors of the requested kind; empty while loading or on error. */
	permissions: AdminPermissionInfo[];
	/** Distinguishes an in-flight or failed load from an empty registry. */
	status: RegistryPermissionStatus;
	/** Retries the registry request after a failure. */
	retry: () => void;
	/** Tracks the localization bundle independently: a bundle failure
	 * degrades labels but must not block the descriptor flow. */
	bundleStatus: RegistryBundleStatus;
	/** Retries a failed localization bundle without touching descriptors. */
	retryBundle: () => void;
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
	const [bundleStatus, setBundleStatus] = useState<RegistryBundleStatus>(() =>
		bundleLoaded ? "ready" : "loading",
	);
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
				// The bundle resolves independently: descriptors are already
				// published, and the pending promise only moves bundleStatus.
				if (result.bundlePending) {
					setBundleStatus("loading");
					void result.bundlePending.then((ok) => {
						if (cancelled) return;
						setBundleStatus(ok ? "ready" : "error");
					});
				} else {
					// No in-flight bundle: a cached-descriptor hit with the
					// bundle already loaded is ready, not a failure.
					setBundleStatus(bundleLoaded ? "ready" : "error");
				}
			} else {
				setStatus("error");
			}
		});
		return () => {
			cancelled = true;
		};
	}, [kind, attempt, tick]);

	const retryBundle = useCallback(() => {
		if (bundleStatus === "loading") return;
		setBundleStatus("loading");
		void loadBundle(activeLocale(), loadGeneration).then((ok) => {
			setBundleStatus(ok ? "ready" : "error");
			if (ok) notifyPermissionsUpdated();
		});
	}, [bundleStatus]);

	// Another consumer's retry or the first successful load updates the
	// shared cache; mounted instances sync from it instead of staying stale.
	useEffect(() => {
		const listener = () => {
			if (!cachedPermissions) return;
			setPermissions(cachedPermissions.filter((item) => item.kind === kind));
			setStatus("ready");
			// A sibling consumer's successful bundle retry clears this
			// instance's failure strip without refetching descriptors.
			if (bundleLoaded) {
				setBundleStatus("ready");
			}
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

	return { permissions, status, retry, bundleStatus, retryBundle };
}

// A language switch invalidates the process cache, discards in-flight loads
// from the previous generation, drops the cached backend translator, and
// bumps the tick that remounted consumers watch, so labels refresh in the
// new language without a page reload.
i18next.on("languageChanged", () => {
	cachedPermissions = null;
	pendingLoad = null;
	cachedBackendT = null;
	bundleLoaded = false;
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
