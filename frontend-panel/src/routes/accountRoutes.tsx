import type { RouteObject } from "react-router-dom";
import { AppShell } from "@/components/shell/AppShell";
import { lazyWithPreload } from "@/lib/lazyWithPreload";
import { accountPaths } from "@/routes/routePaths";

const AccountOverviewPage = lazyWithPreload(
	() => import("@/pages/account/AccountOverviewPage"),
);
const MinecraftProfilesPage = lazyWithPreload(
	() => import("@/pages/account/MinecraftProfilesPage"),
);
const AccountSettingsPage = lazyWithPreload(
	() => import("@/pages/account/AccountSettingsPage"),
);
const AccountAuditPage = lazyWithPreload(
	() => import("@/pages/account/AccountAuditPage"),
);
const TextureWardrobePage = lazyWithPreload(
	() => import("@/pages/account/TextureWardrobePage"),
);

export const accountRoutes = [
	{
		element: <AppShell scope="account" />,
		children: [
			{ path: accountPaths.home, element: <AccountOverviewPage /> },
			{ path: accountPaths.profiles, element: <MinecraftProfilesPage /> },
			{ path: accountPaths.wardrobe, element: <TextureWardrobePage /> },
			{ path: accountPaths.audit, element: <AccountAuditPage /> },
			{ path: accountPaths.settings, element: <AccountSettingsPage /> },
			{
				path: accountPaths.settingsSecurityCompat,
				element: <AccountSettingsPage />,
			},
		],
	},
] satisfies RouteObject[];
