import type { RouteObject } from "react-router-dom";
import { lazyWithPreload } from "@/lib/lazyWithPreload";
import { publicPaths } from "@/routes/routePaths";
import { publicElement } from "@/routes/routeSuspense";

const PublicConnectPage = lazyWithPreload(
	() => import("@/pages/PublicConnectPage"),
);
const TermsPage = lazyWithPreload(() => import("@/pages/TermsPage"));
const PrivacyPage = lazyWithPreload(() => import("@/pages/PrivacyPage"));

export const publicRoutes = [
	{
		path: publicPaths.home,
		element: publicElement(<PublicConnectPage />),
	},
	{
		path: publicPaths.tos,
		element: publicElement(<TermsPage />),
	},
	{
		path: publicPaths.privacy,
		element: publicElement(<PrivacyPage />),
	},
] satisfies RouteObject[];
