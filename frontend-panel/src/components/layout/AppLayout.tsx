import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Outlet, useLocation, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { ShellSidebar } from "@/components/shell/ShellSidebar";
import { ShellTopbar } from "@/components/shell/ShellTopbar";
import type { ShellScope } from "@/components/shell/shellNavigation";
import { readStorageItem, STORAGE_KEYS, writeStorageItem } from "@/lib/storage";
import { cn } from "@/lib/utils";
import { AdminRouteFallback, AppRouteFallback } from "@/router/RouteFallback";
import { adminPaths, publicPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

function getDefaultDesktopSidebarExpanded() {
	if (
		typeof window === "undefined" ||
		typeof window.matchMedia !== "function"
	) {
		return false;
	}

	return window.matchMedia("(min-width: 1280px)").matches;
}

function readStoredDesktopSidebarExpanded() {
	const stored = readStorageItem("local", STORAGE_KEYS.desktopSidebarExpanded);
	if (stored === "true") {
		return true;
	}
	if (stored === "false") {
		return false;
	}

	return null;
}

function writeStoredDesktopSidebarExpanded(expanded: boolean) {
	writeStorageItem(
		"local",
		STORAGE_KEYS.desktopSidebarExpanded,
		String(expanded),
	);
}

export function AppLayout({ scope }: { scope?: ShellScope }) {
	const { t } = useTranslation();
	const location = useLocation();
	const navigate = useNavigate();
	const user = useAuthStore((state) => state.user);
	const isAdmin = useAuthStore((state) => state.isAdmin);
	const logout = useAuthStore((state) => state.logout);
	const branding = useFrontendConfigStore((state) => state.branding);
	const pathname = location.pathname;
	const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
	const [desktopSidebarExpanded, setDesktopSidebarExpanded] = useState(
		() =>
			readStoredDesktopSidebarExpanded() ?? getDefaultDesktopSidebarExpanded(),
	);
	const previousPathnameRef = useRef(pathname);
	const resolvedScope = scope
		? scope
		: pathname.startsWith(adminPaths.home)
			? "admin"
			: "account";
	const isAdminScope = resolvedScope === "admin";

	const handleMobileSidebarToggle = useCallback(() => {
		setMobileSidebarOpen((current) => !current);
	}, []);

	const handleMobileSidebarClose = useCallback(() => {
		setMobileSidebarOpen(false);
	}, []);

	const handleDesktopSidebarToggle = useCallback(() => {
		setDesktopSidebarExpanded((current) => {
			const next = !current;
			writeStoredDesktopSidebarExpanded(next);
			return next;
		});
	}, []);

	const handleLogout = useCallback(async () => {
		await logout();
		toast.success(t("shell.logoutSuccess"));
		navigate(publicPaths.home, { replace: true });
	}, [logout, navigate, t]);

	useEffect(() => {
		if (previousPathnameRef.current === pathname) {
			return;
		}

		previousPathnameRef.current = pathname;
		setMobileSidebarOpen(false);
	}, [pathname]);

	useEffect(() => {
		if (!mobileSidebarOpen) {
			return;
		}

		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				setMobileSidebarOpen(false);
			}
		};

		window.addEventListener("keydown", handleKeyDown);
		return () => {
			window.removeEventListener("keydown", handleKeyDown);
		};
	}, [mobileSidebarOpen]);

	return (
		<div
			data-theme-surface="chrome"
			className={cn(
				"app-shell min-h-dvh text-foreground",
				isAdminScope ? "admin-shell" : "bg-background",
			)}
		>
			<div className="min-h-dvh lg:flex">
				<ShellSidebar
					branding={branding}
					desktopCollapsed={!desktopSidebarExpanded}
					isAdmin={isAdmin}
					mobileOpen={mobileSidebarOpen}
					onMobileClose={handleMobileSidebarClose}
				/>
				<div className="min-w-0 lg:flex-1">
					<ShellTopbar
						branding={branding}
						desktopSidebarExpanded={desktopSidebarExpanded}
						isAdminScope={isAdminScope}
						mobileSidebarOpen={mobileSidebarOpen}
						onDesktopSidebarToggle={handleDesktopSidebarToggle}
						onMobileSidebarToggle={handleMobileSidebarToggle}
						onLogout={() => void handleLogout()}
						user={user}
					/>
					<main>
						<div>
							<Suspense
								fallback={
									isAdminScope ? <AdminRouteFallback /> : <AppRouteFallback />
								}
							>
								<Outlet />
							</Suspense>
						</div>
					</main>
				</div>
			</div>
		</div>
	);
}
