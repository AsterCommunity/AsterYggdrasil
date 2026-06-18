import { matchPath, Navigate, Outlet, useLocation } from "react-router-dom";
import {
	ADMIN_NAV_SCOPE_BY_PATH,
	firstAdminPathForScopes,
} from "@/lib/operatorScopes";
import { RouteAccessState } from "@/routes/guards/RouteAccessState";
import { accountPaths, adminPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";
import type { OperatorScope } from "@/types/api";

function scopeForAdminPath(pathname: string): OperatorScope | null {
	for (const [pattern, scope] of Object.entries(ADMIN_NAV_SCOPE_BY_PATH)) {
		if (matchPath({ path: pattern, end: true }, pathname)) {
			return scope;
		}
	}
	return null;
}

export function AdminOnlyGate() {
	const location = useLocation();
	const isAdmin = useAuthStore((state) => state.isAdmin);
	const canAccessAdminShell = useAuthStore(
		(state) => state.canAccessAdminShell,
	);
	const operatorScopes = useAuthStore((state) => state.operatorScopes);
	const hasOperatorScope = useAuthStore((state) => state.hasOperatorScope);
	const mustChangePassword = useAuthStore(
		(state) => state.user?.must_change_password ?? false,
	);
	if (mustChangePassword) {
		return <Navigate to={accountPaths.forcePasswordChange} replace />;
	}
	if (!canAccessAdminShell) {
		return (
			<RouteAccessState
				actionHref={accountPaths.home}
				actionLabelKey="shell.routeState.adminRequiredAction"
				descriptionKey="shell.routeState.adminRequiredDescription"
				icon="Shield"
				titleKey="shell.routeState.adminRequiredTitle"
			/>
		);
	}
	if (!isAdmin) {
		const requestedScope = scopeForAdminPath(location.pathname);
		const fallbackPath = firstAdminPathForScopes(operatorScopes);
		if (
			fallbackPath != null &&
			location.pathname === adminPaths.home &&
			fallbackPath !== location.pathname
		) {
			return <Navigate to={fallbackPath} replace />;
		}
		if (!requestedScope || !hasOperatorScope(requestedScope)) {
			return (
				<RouteAccessState
					actionHref={fallbackPath ?? accountPaths.home}
					actionLabelKey="shell.routeState.adminRequiredAction"
					descriptionKey="shell.routeState.adminRequiredDescription"
					icon="Shield"
					titleKey="shell.routeState.adminRequiredTitle"
				/>
			);
		}
	}
	return <Outlet />;
}
