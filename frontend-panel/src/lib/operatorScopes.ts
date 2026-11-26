import { adminPaths } from "@/routes/routePaths";
import type { PermissionId } from "@/types/api";

export const OPERATOR_SCOPES = [
	"com.astercosm.yggdrasil.permission.admin.overview",
	"com.astercosm.yggdrasil.permission.admin.users",
	"com.astercosm.yggdrasil.permission.admin.profiles",
	"com.astercosm.yggdrasil.permission.admin.texture_library",
	"com.astercosm.yggdrasil.permission.admin.audit",
	"com.astercosm.yggdrasil.permission.admin.tasks",
	"com.astercosm.yggdrasil.permission.admin.settings",
	"com.astercosm.yggdrasil.permission.admin.external_auth",
] as const satisfies readonly PermissionId[];

export const ADMIN_NAV_SCOPE_BY_PATH = {
	[adminPaths.overview]: "com.astercosm.yggdrasil.permission.admin.overview",
	[adminPaths.users]: "com.astercosm.yggdrasil.permission.admin.users",
	[adminPaths.userInvitations]:
		"com.astercosm.yggdrasil.permission.admin.users",
	[adminPaths.externalAuth]:
		"com.astercosm.yggdrasil.permission.admin.external_auth",
	[adminPaths.yggdrasilForwarding]:
		"com.astercosm.yggdrasil.permission.admin.settings",
	[adminPaths.textureLibrary]:
		"com.astercosm.yggdrasil.permission.admin.texture_library",
	[adminPaths.textureLibraryReviews]:
		"com.astercosm.yggdrasil.permission.admin.texture_library",
	[adminPaths.textureLibraryReports]:
		"com.astercosm.yggdrasil.permission.admin.texture_library",
	[adminPaths.textureLibraryTags]:
		"com.astercosm.yggdrasil.permission.admin.texture_library",
	[adminPaths.audit]: "com.astercosm.yggdrasil.permission.admin.audit",
	[adminPaths.tasks]: "com.astercosm.yggdrasil.permission.admin.tasks",
	[adminPaths.settings]: "com.astercosm.yggdrasil.permission.admin.settings",
	[adminPaths.settingsCategory]:
		"com.astercosm.yggdrasil.permission.admin.settings",
	[adminPaths.about]: "com.astercosm.yggdrasil.permission.admin.overview",
	[adminPaths.userDetail]: "com.astercosm.yggdrasil.permission.admin.users",
	[adminPaths.minecraftProfile]:
		"com.astercosm.yggdrasil.permission.admin.profiles",
} as const satisfies Record<string, PermissionId>;

/**
 * Pre-migration short names for the built-in admin-area scopes. Cached
 * users written before the namespaced identifier rollout are upgraded to
 * the full ids instead of being silently dropped on load.
 */
export const LEGACY_OPERATOR_SCOPE_IDS: Readonly<Record<string, PermissionId>> =
	{
		overview: "com.astercosm.yggdrasil.permission.admin.overview",
		users: "com.astercosm.yggdrasil.permission.admin.users",
		profiles: "com.astercosm.yggdrasil.permission.admin.profiles",
		texture_library: "com.astercosm.yggdrasil.permission.admin.texture_library",
		audit: "com.astercosm.yggdrasil.permission.admin.audit",
		tasks: "com.astercosm.yggdrasil.permission.admin.tasks",
		settings: "com.astercosm.yggdrasil.permission.admin.settings",
		external_auth: "com.astercosm.yggdrasil.permission.admin.external_auth",
	};

export function hasOperatorScope(
	scopes: readonly PermissionId[] | null | undefined,
	scope: PermissionId,
) {
	return scopes?.includes(scope) ?? false;
}

export function firstAdminPathForScopes(scopes: readonly PermissionId[]) {
	for (const scope of OPERATOR_SCOPES) {
		if (!scopes.includes(scope)) continue;
		switch (scope) {
			case "com.astercosm.yggdrasil.permission.admin.overview":
				return adminPaths.overview;
			case "com.astercosm.yggdrasil.permission.admin.users":
				return adminPaths.users;
			case "com.astercosm.yggdrasil.permission.admin.profiles":
				break;
			case "com.astercosm.yggdrasil.permission.admin.texture_library":
				return adminPaths.textureLibrary;
			case "com.astercosm.yggdrasil.permission.admin.audit":
				return adminPaths.audit;
			case "com.astercosm.yggdrasil.permission.admin.tasks":
				return adminPaths.tasks;
			case "com.astercosm.yggdrasil.permission.admin.settings":
				return adminPaths.settings;
			case "com.astercosm.yggdrasil.permission.admin.external_auth":
				return adminPaths.externalAuth;
		}
	}
	return null;
}
