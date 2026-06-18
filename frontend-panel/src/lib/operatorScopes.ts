import { adminPaths } from "@/routes/routePaths";
import type { OperatorScope } from "@/types/api";

export const OPERATOR_SCOPES = [
	"overview",
	"users",
	"profiles",
	"texture_library",
	"audit",
	"tasks",
	"settings",
	"external_auth",
] as const satisfies readonly OperatorScope[];

export const ADMIN_NAV_SCOPE_BY_PATH = {
	[adminPaths.overview]: "overview",
	[adminPaths.users]: "users",
	[adminPaths.userInvitations]: "users",
	[adminPaths.externalAuth]: "external_auth",
	[adminPaths.textureLibrary]: "texture_library",
	[adminPaths.audit]: "audit",
	[adminPaths.tasks]: "tasks",
	[adminPaths.settings]: "settings",
	[adminPaths.about]: "overview",
	[adminPaths.userDetail]: "users",
	[adminPaths.minecraftProfile]: "profiles",
} as const satisfies Record<string, OperatorScope>;

export function hasOperatorScope(
	scopes: readonly OperatorScope[] | null | undefined,
	scope: OperatorScope,
) {
	return scopes?.includes(scope) ?? false;
}

export function firstAdminPathForScopes(scopes: readonly OperatorScope[]) {
	for (const scope of OPERATOR_SCOPES) {
		if (!scopes.includes(scope)) continue;
		switch (scope) {
			case "overview":
				return adminPaths.overview;
			case "users":
				return adminPaths.users;
			case "profiles":
				break;
			case "texture_library":
				return adminPaths.textureLibrary;
			case "audit":
				return adminPaths.audit;
			case "tasks":
				return adminPaths.tasks;
			case "settings":
				return adminPaths.settings;
			case "external_auth":
				return adminPaths.externalAuth;
		}
	}
	return null;
}
