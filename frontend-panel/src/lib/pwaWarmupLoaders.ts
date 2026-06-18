export interface WarmupLoaderEntry {
	key: string;
	label: string;
	load: () => Promise<unknown>;
}

const loginRouteWarmupLoader = {
	key: "route:login",
	label: "LoginPage",
	load: () => import("@/pages/LoginPage"),
} satisfies WarmupLoaderEntry;

const workbenchRouteWarmupLoader = {
	key: "route:workbench",
	label: "AccountOverviewPage",
	load: () => import("@/pages/account/AccountOverviewPage"),
} satisfies WarmupLoaderEntry;

export const loginSuccessPathWarmupLoaders = [
	workbenchRouteWarmupLoader,
	{
		key: "route:profiles",
		label: "MinecraftProfilesPage",
		load: () => import("@/pages/account/MinecraftProfilesPage"),
	},
	{
		key: "route:texture-library",
		label: "PublicTextureLibraryPage",
		load: () => import("@/pages/PublicTextureLibraryPage"),
	},
	{
		key: "route:wardrobe",
		label: "TextureWardrobePage",
		load: () => import("@/pages/account/TextureWardrobePage"),
	},
] satisfies WarmupLoaderEntry[];

export const userRouteWarmupLoaders = [
	loginRouteWarmupLoader,
	{
		key: "route:error",
		label: "ErrorPage",
		load: () => import("@/pages/ErrorPage"),
	},
	{
		key: "route:public-connect",
		label: "PublicConnectPage",
		load: () => import("@/pages/PublicConnectPage"),
	},
	workbenchRouteWarmupLoader,
	{
		key: "route:profiles",
		label: "MinecraftProfilesPage",
		load: () => import("@/pages/account/MinecraftProfilesPage"),
	},
	{
		key: "route:texture-library",
		label: "PublicTextureLibraryPage",
		load: () => import("@/pages/PublicTextureLibraryPage"),
	},
	{
		key: "route:wardrobe",
		label: "TextureWardrobePage",
		load: () => import("@/pages/account/TextureWardrobePage"),
	},
] satisfies WarmupLoaderEntry[];

export const adminRouteWarmupLoaders = [
	{
		key: "route:admin-audit",
		label: "AdminAuditPage",
		load: () => import("@/pages/admin/AdminAuditPage"),
	},
	{
		key: "route:admin-tasks",
		label: "AdminTasksPage",
		load: () => import("@/pages/admin/AdminTasksPage"),
	},
	{
		key: "route:admin-user-detail",
		label: "AdminUserDetailPage",
		load: () => import("@/pages/admin/AdminUserDetailPage"),
	},
] satisfies WarmupLoaderEntry[];
