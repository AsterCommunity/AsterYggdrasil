import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { AuthUserMenu } from "@/components/common/AuthUserMenu";
import { LanguageMenu } from "@/components/common/LanguageMenu";
import { BrandMark } from "@/components/layout/BrandMark";
import { ThemeToggleButton } from "@/components/layout/ThemeToggleButton";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import type { AppliedBranding } from "@/lib/branding";
import { cn } from "@/lib/utils";
import type { AuthUserInfo } from "@/types/api";

export function ShellTopbar({
	branding,
	desktopSidebarExpanded,
	isAdminScope,
	mobileSidebarOpen,
	onDesktopSidebarToggle,
	onMobileSidebarToggle,
	onLogout,
	user,
}: {
	branding: AppliedBranding;
	desktopSidebarExpanded: boolean;
	isAdminScope: boolean;
	mobileSidebarOpen: boolean;
	onDesktopSidebarToggle: () => void;
	onMobileSidebarToggle: () => void;
	onLogout: () => void;
	user: AuthUserInfo | null;
}) {
	const { t } = useTranslation();
	const mobileSidebarLabel = mobileSidebarOpen
		? t("common.close")
		: t("shell.openNavigation");
	const desktopSidebarLabel = desktopSidebarExpanded
		? t("shell.collapseNavigation")
		: t("shell.expandNavigation");

	return (
		<header
			data-theme-surface="chrome"
			className={cn(
				"sticky top-0 z-30 border-b backdrop-blur-xl",
				isAdminScope
					? "border-border/70 bg-background/88"
					: "border-border/60 bg-background/88",
			)}
		>
			<div className="flex min-h-16 items-center gap-3 px-4 sm:px-6 lg:px-7">
				<Button
					type="button"
					variant="outline"
					size="icon"
					className="relative size-10 shrink-0 overflow-hidden rounded-lg border-border/70 bg-card/70 shadow-xs lg:hidden"
					aria-label={mobileSidebarLabel}
					aria-expanded={mobileSidebarOpen}
					aria-controls="shell-mobile-sidebar"
					onClick={onMobileSidebarToggle}
				>
					<span className="relative inline-flex size-5 items-center justify-center">
						<span
							className={cn(
								"absolute inset-0 flex items-center justify-center transition-all duration-200 ease-out motion-reduce:transition-none",
								mobileSidebarOpen
									? "-rotate-90 scale-75 opacity-0"
									: "rotate-0 scale-100 opacity-100",
							)}
						>
							<Icon name="List" className="size-5" />
						</span>
						<span
							className={cn(
								"absolute inset-0 flex items-center justify-center transition-all duration-200 ease-out motion-reduce:transition-none",
								mobileSidebarOpen
									? "rotate-0 scale-100 opacity-100"
									: "rotate-90 scale-75 opacity-0",
							)}
						>
							<Icon name="X" className="size-5" />
						</span>
					</span>
				</Button>
				<Button
					type="button"
					variant="outline"
					size="icon"
					className="relative hidden size-10 shrink-0 overflow-hidden rounded-lg border-border/70 bg-card/70 shadow-xs lg:inline-flex"
					aria-label={desktopSidebarLabel}
					aria-pressed={desktopSidebarExpanded}
					onClick={onDesktopSidebarToggle}
				>
					<span className="relative inline-flex size-5 items-center justify-center">
						<span
							className={cn(
								"absolute inset-0 flex items-center justify-center transition-all duration-200 ease-out motion-reduce:transition-none",
								desktopSidebarExpanded
									? "-rotate-90 scale-75 opacity-0"
									: "rotate-0 scale-100 opacity-100",
							)}
						>
							<Icon name="List" className="size-5" />
						</span>
						<span
							className={cn(
								"absolute inset-0 flex items-center justify-center transition-all duration-200 ease-out motion-reduce:transition-none",
								desktopSidebarExpanded
									? "rotate-0 scale-100 opacity-100"
									: "rotate-90 scale-75 opacity-0",
							)}
						>
							<Icon name="CaretLeft" className="size-5" />
						</span>
					</span>
				</Button>
				{isAdminScope ? (
					<Link
						to="/"
						className="group flex min-w-0 items-center gap-3 lg:hidden"
					>
						<BrandMark
							branding={branding}
							className="size-9 shrink-0 object-contain transition-transform group-hover:-translate-y-0.5"
							wordmarkClassName="h-9 max-w-40"
						/>
						<span className="hidden min-w-0 sm:block">
							<span className="block truncate text-sm font-semibold">
								{branding.title || t("brand.name")}
							</span>
						</span>
					</Link>
				) : null}
				<div className="ml-auto flex items-center gap-2">
					<ThemeToggleButton className="inline-flex" />
					<LanguageMenu
						className={cn("h-9", isAdminScope && "bg-card/65")}
						compactOnMobile
					/>
					{user ? (
						<AuthUserMenu
							user={user}
							scope={isAdminScope ? "admin" : "account"}
							onLogout={onLogout}
						/>
					) : null}
				</div>
			</div>
		</header>
	);
}
