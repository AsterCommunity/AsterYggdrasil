import { useEffect } from "react";
import { Outlet } from "react-router-dom";
import { Loading } from "@/router/Loading";
import { RouteAccessState } from "@/routes/guards/RouteAccessState";
import { accountPaths, publicPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";

export function AuthenticatedGate() {
	const hydrate = useAuthStore((state) => state.hydrate);
	const checking = useAuthStore((state) => state.checking);
	const errorCode = useAuthStore((state) => state.errorCode);
	const isAuthStale = useAuthStore((state) => state.isAuthStale);
	const isAuthenticated = useAuthStore((state) => state.isAuthenticated);

	useEffect(() => {
		void hydrate();
	}, [hydrate]);

	if (checking) {
		return <Loading />;
	}
	if (
		isAuthStale &&
		(errorCode === "network_error" || errorCode === "request_timeout")
	) {
		return (
			<RouteAccessState
				actionLabelKey="shell.routeState.networkErrorAction"
				actionOnClick={() => void hydrate()}
				descriptionKey="shell.routeState.networkErrorDescription"
				icon="WifiX"
				titleKey="shell.routeState.networkErrorTitle"
			/>
		);
	}
	if (!isAuthenticated) {
		return (
			<RouteAccessState
				actionHref={publicPaths.login}
				actionLabelKey="shell.routeState.loginRequiredAction"
				descriptionKey="shell.routeState.loginRequiredDescription"
				icon="Lock"
				titleKey="shell.routeState.loginRequiredTitle"
			/>
		);
	}
	return <Outlet />;
}

export function authenticatedFallbackPath() {
	return accountPaths.home;
}
