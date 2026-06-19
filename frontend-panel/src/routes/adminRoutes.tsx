import type { RouteObject } from "react-router-dom";
import { AppShell } from "@/components/shell/AppShell";
import { lazyWithPreload } from "@/lib/lazyWithPreload";
import { AdminOnlyGate } from "@/routes/guards/AdminOnlyGate";
import { adminPaths } from "@/routes/routePaths";

const AdminSettingsPage = lazyWithPreload(
	() => import("@/pages/admin/AdminSettingsPage"),
);
const AdminOverviewPage = lazyWithPreload(
	() => import("@/pages/admin/AdminOverviewPage"),
);
const AdminAuditPage = lazyWithPreload(
	() => import("@/pages/admin/AdminAuditPage"),
);
const AdminTasksPage = lazyWithPreload(
	() => import("@/pages/admin/AdminTasksPage"),
);
const AdminAboutPage = lazyWithPreload(
	() => import("@/pages/admin/AdminAboutPage"),
);
const AdminExternalAuthPage = lazyWithPreload(
	() => import("@/pages/admin/AdminExternalAuthPage"),
);
const AdminYggdrasilForwardingPage = lazyWithPreload(
	() => import("@/pages/admin/AdminYggdrasilForwardingPage"),
);
const AdminTextureLibraryPage = lazyWithPreload(
	() => import("@/pages/admin/AdminTextureLibraryPage"),
);
const AdminTextureLibraryTexturesPage = lazyWithPreload(
	() => import("@/pages/admin/AdminTextureLibraryTexturesPage"),
);
const AdminTextureLibraryReportsPage = lazyWithPreload(
	() => import("@/pages/admin/AdminTextureLibraryReportsPage"),
);
const AdminUsersPage = lazyWithPreload(
	() => import("@/pages/admin/AdminUsersPage"),
);
const AdminUserInvitationsPage = lazyWithPreload(
	() => import("@/pages/admin/AdminUserInvitationsPage"),
);
const AdminMinecraftProfilePage = lazyWithPreload(
	() => import("@/pages/admin/AdminMinecraftProfilePage"),
);
const AdminUserDetailPage = lazyWithPreload(
	() => import("@/pages/admin/AdminUserDetailPage"),
);

export const adminRoutes = [
	{
		element: <AdminOnlyGate />,
		children: [
			{
				element: <AppShell scope="admin" />,
				children: [
					{
						path: adminPaths.overview,
						element: <AdminOverviewPage />,
					},
					{
						path: adminPaths.audit,
						element: <AdminAuditPage />,
					},
					{
						path: adminPaths.users,
						element: <AdminUsersPage />,
					},
					{
						path: adminPaths.userInvitations,
						element: <AdminUserInvitationsPage />,
					},
					{
						path: adminPaths.minecraftProfile,
						element: <AdminMinecraftProfilePage />,
					},
					{
						path: adminPaths.userDetail,
						element: <AdminUserDetailPage />,
					},
					{
						path: adminPaths.externalAuth,
						element: <AdminExternalAuthPage />,
					},
					{
						path: adminPaths.yggdrasilForwarding,
						element: <AdminYggdrasilForwardingPage />,
					},
					{
						path: adminPaths.textureLibrary,
						element: <AdminTextureLibraryTexturesPage mode="all" />,
					},
					{
						path: adminPaths.textureLibraryDetail,
						element: <AdminTextureLibraryTexturesPage mode="detail" />,
					},
					{
						path: adminPaths.textureLibraryReviews,
						element: <AdminTextureLibraryTexturesPage mode="reviews" />,
					},
					{
						path: adminPaths.textureLibraryReports,
						element: <AdminTextureLibraryReportsPage />,
					},
					{
						path: adminPaths.textureLibraryTags,
						element: <AdminTextureLibraryPage />,
					},
					{
						path: adminPaths.tasks,
						element: <AdminTasksPage />,
					},
					{
						path: adminPaths.settings,
						element: <AdminSettingsPage />,
					},
					{
						path: adminPaths.settingsCategory,
						element: <AdminSettingsPage />,
					},
					{
						path: adminPaths.about,
						element: <AdminAboutPage />,
					},
				],
			},
		],
	},
] satisfies RouteObject[];
