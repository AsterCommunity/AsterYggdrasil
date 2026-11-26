import type { AdminPermissionInfo } from "@/types/api";

/**
 * Mirrors the backend built-in permission registry for frontend tests.
 * Identifier strings, kinds, and i18n keys agree with the Rust
 * BUILTIN_DESCRIPTORS table.
 */
export const REGISTRY_PERMISSIONS: AdminPermissionInfo[] = [
	{
		id: "com.astercosm.yggdrasil.permission.admin.overview",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.overview",
		label: "Overview",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.permission.admin.users",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.users",
		label: "Users",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.permission.admin.profiles",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.profiles",
		label: "Profiles",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.permission.admin.texture_library",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.texture_library",
		label: "Texture library",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.permission.admin.audit",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.audit",
		label: "Audit",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.permission.admin.tasks",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.tasks",
		label: "Tasks",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.permission.admin.settings",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.settings",
		label: "Settings",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.permission.admin.external_auth",
		kind: "admin_area",
		label_i18n_key: "admin.users.operatorScope.external_auth",
		label: "External auth",
		group: "admin_panel",
	},
	{
		id: "com.astercosm.yggdrasil.capability.yggdrasil.access",
		kind: "capability",
		label_i18n_key: "admin.users.bans.scope.yggdrasil_access",
		label: "Yggdrasil access",
		group: "yggdrasil",
	},
	{
		id: "com.astercosm.yggdrasil.capability.yggdrasil.join",
		kind: "capability",
		label_i18n_key: "admin.users.bans.scope.yggdrasil_join",
		label: "Multiplayer join",
		group: "yggdrasil",
	},
	{
		id: "com.astercosm.yggdrasil.capability.profile.manage",
		kind: "capability",
		label_i18n_key: "admin.users.bans.scope.minecraft_profile_manage",
		label: "Profile management",
		group: "profile",
	},
	{
		id: "com.astercosm.yggdrasil.capability.texture.upload",
		kind: "capability",
		label_i18n_key: "admin.users.bans.scope.texture_upload",
		label: "Texture upload",
		group: "texture",
	},
	{
		id: "com.astercosm.yggdrasil.capability.texture_library.interact",
		kind: "capability",
		label_i18n_key: "admin.users.bans.scope.texture_library_interact",
		label: "Texture library interaction",
		group: "texture_library",
	},
];

/**
 * The localized label texts the backend localization endpoint serves for
 * the default locale; mirrors BUILTIN_PERMISSION_MESSAGES.
 */
export const REGISTRY_MESSAGES: Record<string, string> = {
	"admin.users.bans.scope.minecraft_profile_manage": "Profile management",
	"admin.users.bans.scope.texture_library_interact":
		"Texture library interaction",
	"admin.users.bans.scope.texture_upload": "Texture upload and wardrobe",
	"admin.users.bans.scope.yggdrasil_access": "Yggdrasil login access",
	"admin.users.bans.scope.yggdrasil_join": "Yggdrasil server join",
	"admin.users.operatorScope.audit": "Audit logs",
	"admin.users.operatorScope.external_auth": "External auth",
	"admin.users.operatorScope.overview": "Admin overview",
	"admin.users.operatorScope.profiles": "Profiles",
	"admin.users.operatorScope.settings": "System settings",
	"admin.users.operatorScope.tasks": "Background tasks",
	"admin.users.operatorScope.texture_library": "Texture library",
	"admin.users.operatorScope.users": "Users",
};
