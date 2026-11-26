import { api } from "@/services/http";
import type {
	AdminPermissionInfo,
	PermissionLocalizationBundle,
} from "@/types/api";

/**
 * Reads the server permission registry: namespaced identifiers plus their
 * display metadata and localized labels. Available to every authenticated
 * account so player-facing capability labels render without admin
 * privileges.
 */
export const permissionService = {
	list: () => api.get<{ permissions: AdminPermissionInfo[] }>("/permissions"),
	localizations: (locale: string) =>
		api.get<PermissionLocalizationBundle>(
			`/permissions/localizations?locale=${encodeURIComponent(locale)}`,
		),
};
