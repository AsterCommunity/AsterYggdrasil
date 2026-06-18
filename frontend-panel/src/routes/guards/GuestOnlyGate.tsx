import { useEffect } from "react";
import { Outlet } from "react-router-dom";
import { AuthRouteFallback } from "@/router/RouteFallback";
import { RouteAccessState } from "@/routes/guards/RouteAccessState";
import { accountPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";

export function GuestOnlyGate() {
	const hydrate = useAuthStore((state) => state.hydrate);
	const checking = useAuthStore((state) => state.checking);
	const isAuthStale = useAuthStore((state) => state.isAuthStale);
	const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
	const mustChangePassword = useAuthStore(
		(state) => state.user?.must_change_password ?? false,
	);

	useEffect(() => {
		if (!isAuthenticated && !isAuthStale) return;
		void hydrate();
	}, [hydrate, isAuthenticated, isAuthStale]);

	if (isAuthenticated) {
		return (
			<RouteAccessState
				actionHref={
					mustChangePassword
						? accountPaths.forcePasswordChange
						: accountPaths.home
				}
				actionLabelKey={
					mustChangePassword
						? "shell.routeState.forcePasswordChangeAction"
						: "shell.routeState.alreadySignedInAction"
				}
				descriptionKey={
					mustChangePassword
						? "shell.routeState.forcePasswordChangeDescription"
						: "shell.routeState.alreadySignedInDescription"
				}
				icon="Lock"
				titleKey={
					mustChangePassword
						? "shell.routeState.forcePasswordChangeTitle"
						: "shell.routeState.alreadySignedInTitle"
				}
			/>
		);
	}
	if (checking && isAuthStale) {
		return <AuthRouteFallback />;
	}
	return <Outlet />;
}
