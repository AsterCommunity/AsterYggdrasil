import type { RouteObject } from "react-router-dom";
import { lazyWithPreload } from "@/lib/lazyWithPreload";
import { GuestOnlyGate } from "@/routes/guards/GuestOnlyGate";
import { publicPaths } from "@/routes/routePaths";
import { authElement } from "@/routes/routeSuspense";

const LoginPage = lazyWithPreload(() => import("@/pages/LoginPage"));
const InvitePage = lazyWithPreload(() => import("@/pages/InvitePage"));
const ResetPasswordPage = lazyWithPreload(
	() => import("@/pages/ResetPasswordPage"),
);

export const authRoutes = [
	{
		path: publicPaths.login,
		element: <GuestOnlyGate />,
		children: [{ index: true, element: authElement(<LoginPage />) }],
	},
	{
		path: publicPaths.register,
		element: <GuestOnlyGate />,
		children: [{ index: true, element: authElement(<LoginPage />) }],
	},
	{
		path: publicPaths.invite,
		element: <GuestOnlyGate />,
		children: [{ index: true, element: authElement(<InvitePage />) }],
	},
	{
		path: publicPaths.resetPassword,
		element: <GuestOnlyGate />,
		children: [{ index: true, element: authElement(<ResetPasswordPage />) }],
	},
] satisfies RouteObject[];
