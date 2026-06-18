import { Navigate, Outlet } from "react-router-dom";
import { RouteAccessState } from "@/routes/guards/RouteAccessState";
import { accountPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";

export function AdminOnlyGate() {
	const isAdmin = useAuthStore((state) => state.isAdmin);
	const mustChangePassword = useAuthStore(
		(state) => state.user?.must_change_password ?? false,
	);
	if (mustChangePassword) {
		return <Navigate to={accountPaths.forcePasswordChange} replace />;
	}
	if (!isAdmin) {
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
	return <Outlet />;
}
