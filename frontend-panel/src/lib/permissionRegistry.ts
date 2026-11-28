import { useCallback, useEffect, useState } from "react";
import type { useTranslation } from "react-i18next";
import { permissionService } from "@/services/permissionService";
import type {
	AdminPermissionInfo,
	PermissionId,
	PermissionKind,
} from "@/types/api";

let cachedPermissions: AdminPermissionInfo[] | null = null;
let pendingLoad: Promise<LoadResult> | null = null;

type LoadResult =
	| { ok: true; permissions: AdminPermissionInfo[] }
	| { ok: false };

/**
 * Loads the registry once per process, distinguishing a genuinely empty
 * registry from a failed request; failures are never cached, so a retry
 * always reaches the network again.
 */
function loadPermissions(): Promise<LoadResult> {
	if (cachedPermissions) {
		return Promise.resolve({ ok: true, permissions: cachedPermissions });
	}
	if (pendingLoad) return pendingLoad;
	const current: Promise<LoadResult> = permissionService
		.list()
		.then((data) => {
			// A superseded request must neither clear the newer pending load
			// nor overwrite an already-populated cache.
			if (pendingLoad === current) pendingLoad = null;
			cachedPermissions ??= data.permissions;
			return { ok: true, permissions: cachedPermissions } as LoadResult;
		})
		.catch(() => {
			// Registry introspection is display-only; a failed request must
			// not read as "no permissions are registered".
			if (pendingLoad === current) pendingLoad = null;
			return { ok: false } as LoadResult;
		});
	pendingLoad = current;
	return current;
}

type RegistryPermissionsState =
	| { status: "error" }
	| { status: "loading" }
	| {
			/** Descriptors of the requested kind. */
			permissions: AdminPermissionInfo[];
			status: "ready";
	  };

export type RegistryPermissions = {
	/** Retries the registry request after a failure. */
	retry: () => void;
} & RegistryPermissionsState;

/**
 * Loads the registered permissions of one semantic kind from the server
 * registry. The load phase travels with its outcome as a single union
 * state value, so a loading or failed registry cannot present itself as
 * an (empty) ready one. Synchronous callers (static nav matrices) keep
 * using the built-in constant lists; this hook is for editors and label
 * rendering.
 */
export function useRegistryPermissions(
	kind: PermissionKind,
): RegistryPermissions {
	const [attempt, setAttempt] = useState(0);
	const [state, setState] = useState<RegistryPermissionsState>(() =>
		cachedPermissions
			? {
					permissions: cachedPermissions.filter((item) => item.kind === kind),
					status: "ready",
				}
			: { status: "loading" },
	);

	// biome-ignore lint/correctness/useExhaustiveDependencies: attempt is an intentional retry trigger.
	useEffect(() => {
		let cancelled = false;
		void loadPermissions().then((result) => {
			if (cancelled) return;
			setState(
				result.ok
					? {
							permissions: result.permissions.filter(
								(item) => item.kind === kind,
							),
							status: "ready",
						}
					: { status: "error" },
			);
		});
		return () => {
			cancelled = true;
		};
	}, [kind, attempt]);

	const retry = useCallback(() => {
		pendingLoad = null;
		setAttempt((current) => current + 1);
		setState({ status: "loading" });
	}, []);

	return { ...state, retry };
}

/**
 * Resolves the display label for one descriptor: localized i18n message
 * when the bundle carries the key, otherwise the server-provided fallback.
 */
export function permissionLabel(
	permission: AdminPermissionInfo,
	t: ReturnType<typeof useTranslation>["t"],
): string {
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
