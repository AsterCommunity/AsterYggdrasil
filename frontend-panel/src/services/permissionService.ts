import { api } from "@/services/http";
import type { AdminPermissionInfo } from "@/types/api";

/**
 * Reads the server permission registry: namespaced identifiers plus their
 * display metadata. Available to every authenticated account so
 * player-facing capability labels render without admin privileges.
 */
export const permissionService = {
	list: () => api.get<{ permissions: AdminPermissionInfo[] }>("/permissions"),
};
