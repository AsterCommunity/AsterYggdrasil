import type { IconName } from "@/components/ui/icon";
import { hasOperatorScope } from "@/lib/operatorScopes";
import { accountPaths, adminPaths, publicPaths } from "@/routes/routePaths";
import type { PermissionId } from "@/types/api";

export type ShellScope = "account" | "admin";

export type ShellNavItem = {
	to: string;
	labelKey: string;
	icon: IconName;
	end?: boolean;
	operatorScope?: PermissionId;
	preload?: () => Promise<unknown>;
};

export type ShellNavSection = {
	id: "account" | "admin";
	labelKey: string;
	items: ShellNavItem[];
};

export const accountNavItems: ShellNavItem[] = [
	{
		to: accountPaths.home,
		labelKey: "nav.account",
		icon: "Gauge",
		end: true,
		preload: () => import("@/pages/account/AccountOverviewPage"),
	},
	{
		to: accountPaths.profiles,
		labelKey: "nav.profiles",
		icon: "User",
		preload: () => import("@/pages/account/MinecraftProfilesPage"),
	},
	{
		to: accountPaths.wardrobe,
		labelKey: "nav.wardrobe",
		icon: "FileImage",
		preload: () =>
			import("@/pages/account/texture-wardrobe-page/TextureWardrobePage"),
	},
	{
		to: publicPaths.textureLibrary,
		labelKey: "nav.textureLibrary",
		icon: "Images",
		preload: () =>
			import("@/pages/public-texture-library/PublicTextureLibraryPage"),
	},
	{
		to: accountPaths.audit,
		labelKey: "nav.accountAudit",
		icon: "ClipboardText",
		preload: () => import("@/pages/account/AccountAuditPage"),
	},
];

export const personalSettingsNavItem: ShellNavItem = {
	to: accountPaths.settings,
	labelKey: "nav.personalSettings",
	icon: "Gear",
	preload: () => import("@/pages/account/AccountSettingsPage"),
};

export const adminNavItems: ShellNavItem[] = [
	{
		to: adminPaths.overview,
		labelKey: "admin.nav.overview",
		icon: "Gauge",
		end: true,
		operatorScope: "com.astercosm.yggdrasil.permission.admin.overview",
		preload: () => import("@/pages/admin/AdminOverviewPage"),
	},
	{
		to: adminPaths.users,
		labelKey: "admin.nav.users",
		icon: "User",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.users",
		preload: () => import("@/pages/admin/AdminUsersPage"),
	},
	{
		to: adminPaths.externalAuth,
		labelKey: "admin.nav.externalAuth",
		icon: "SignIn",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.external_auth",
		preload: () =>
			import("@/pages/admin/admin-external-auth-page/AdminExternalAuthPage"),
	},
	{
		to: adminPaths.yggdrasilForwarding,
		labelKey: "admin.nav.yggdrasilForwarding",
		icon: "Shuffle",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.settings",
		preload: () =>
			import(
				"@/pages/admin/admin-yggdrasil-forwarding-page/AdminYggdrasilForwardingPage"
			),
	},
	{
		to: adminPaths.textureLibrary,
		labelKey: "admin.nav.textureLibrary",
		icon: "Images",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.texture_library",
		preload: () =>
			import(
				"@/pages/admin/admin-texture-library-textures-page/AdminTextureLibraryTexturesPage"
			),
	},
	{
		to: adminPaths.audit,
		labelKey: "admin.nav.audit",
		icon: "ClipboardText",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.audit",
		preload: () => import("@/pages/admin/AdminAuditPage"),
	},
	{
		to: adminPaths.tasks,
		labelKey: "admin.nav.tasks",
		icon: "Queue",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.tasks",
		preload: () => import("@/pages/admin/AdminTasksPage"),
	},
	{
		to: adminPaths.settings,
		labelKey: "admin.nav.settings",
		icon: "Gear",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.settings",
		preload: () =>
			import("@/pages/admin/admin-settings-page/AdminSettingsPage"),
	},
	{
		to: adminPaths.about,
		labelKey: "admin.nav.about",
		icon: "Info",
		operatorScope: "com.astercosm.yggdrasil.permission.admin.overview",
		preload: () => import("@/pages/admin/AdminAboutPage"),
	},
];

export function getShellNavSections({
	isAdmin,
	operatorScopes,
	textureLibraryEnabled = true,
}: {
	isAdmin: boolean;
	operatorScopes: readonly PermissionId[];
	textureLibraryEnabled?: boolean;
}): ShellNavSection[] {
	const visibleAccountItems = accountNavItems.filter(
		(item) => item.to !== publicPaths.textureLibrary || textureLibraryEnabled,
	);
	const sections: ShellNavSection[] = [
		{
			id: "account",
			labelKey: "shell.sections.account",
			items: [...visibleAccountItems, personalSettingsNavItem],
		},
	];

	const visibleAdminItems = isAdmin
		? adminNavItems
		: adminNavItems.filter((item) => {
				const scope = item.operatorScope;
				return scope ? hasOperatorScope(operatorScopes, scope) : false;
			});

	if (visibleAdminItems.length > 0) {
		sections.push({
			id: "admin",
			labelKey: "shell.sections.admin",
			items: visibleAdminItems,
		});
	}

	return sections;
}
