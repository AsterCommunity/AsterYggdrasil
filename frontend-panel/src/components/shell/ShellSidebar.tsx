import { useTranslation } from "react-i18next";
import { Link, NavLink } from "react-router-dom";
import { BrandMark } from "@/components/layout/BrandMark";
import {
	getShellNavSections,
	type ShellNavItem,
} from "@/components/shell/shellNavigation";
import { Icon } from "@/components/ui/icon";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import type { AppliedBranding } from "@/lib/branding";
import { cn } from "@/lib/utils";

export function ShellSidebar({
	branding,
	desktopCollapsed,
	isAdmin,
	mobileOpen,
	onMobileClose,
}: {
	branding: AppliedBranding;
	desktopCollapsed: boolean;
	isAdmin: boolean;
	mobileOpen: boolean;
	onMobileClose: () => void;
}) {
	const { t } = useTranslation();
	const navSections = getShellNavSections(isAdmin);

	return (
		<>
			<button
				type="button"
				className={cn(
					"fixed inset-0 z-40 cursor-default bg-black/42 transition-opacity duration-200 ease-out lg:hidden motion-reduce:transition-none",
					mobileOpen ? "opacity-100" : "pointer-events-none opacity-0",
				)}
				aria-label={t("common.close")}
				tabIndex={mobileOpen ? 0 : -1}
				onClick={onMobileClose}
			/>
			<aside
				id="shell-mobile-sidebar"
				data-slot="shell-mobile-drawer"
				data-theme-surface="chrome"
				className={cn(
					"fixed inset-y-0 left-0 z-50 grid h-dvh w-[min(22rem,calc(100vw-2.5rem))] grid-rows-[auto_minmax(0,1fr)] overflow-hidden border-white/10 bg-[#0b271b] text-white transition-[translate,box-shadow] duration-200 ease-out will-change-transform lg:hidden motion-reduce:transition-none",
					mobileOpen
						? "translate-x-0 shadow-2xl shadow-black/35"
						: "pointer-events-none -translate-x-[calc(100%+1rem)] shadow-none",
				)}
				aria-hidden={!mobileOpen}
				inert={mobileOpen ? undefined : true}
			>
				<ShellSidebarBrand branding={branding} className="min-h-20 pr-14" />
				<ShellSidebarNavigation
					navSections={navSections}
					className="min-h-0 overflow-y-auto px-4 pt-2 pb-6"
					onNavigate={onMobileClose}
				/>
				<button
					type="button"
					className="absolute top-4 right-4 inline-flex size-9 items-center justify-center rounded-lg text-emerald-50/82 transition-colors hover:bg-white/10 hover:text-white focus-visible:ring-3 focus-visible:ring-white/25 focus-visible:outline-none"
					aria-label={t("common.close")}
					onClick={onMobileClose}
				>
					<Icon name="X" className="size-4" />
				</button>
			</aside>
			<aside
				data-slot="shell-desktop-sidebar"
				data-theme-surface="chrome"
				data-state={desktopCollapsed ? "collapsed" : "expanded"}
				className={cn(
					"hidden shrink-0 border-white/10 bg-[#0b271b] text-white shadow-2xl shadow-emerald-950/20 transition-[width] duration-200 ease-out lg:sticky lg:top-0 lg:flex lg:h-dvh lg:self-start lg:flex-col lg:overflow-y-auto motion-reduce:transition-none",
					desktopCollapsed ? "lg:w-[6.5rem]" : "lg:w-[17rem]",
				)}
			>
				<ShellSidebarBrand branding={branding} collapsed={desktopCollapsed} />
				<ShellSidebarNavigation
					navSections={navSections}
					className={cn("flex-1 py-6", desktopCollapsed ? "px-0" : "px-4")}
					collapsed={desktopCollapsed}
				/>
			</aside>
		</>
	);
}

export function ShellSidebarBrand({
	branding,
	className,
	collapsed = false,
}: {
	branding: AppliedBranding;
	className?: string;
	collapsed?: boolean;
}) {
	const { t } = useTranslation();

	return (
		<div
			data-theme-surface="chrome"
			className={cn(
				"flex min-h-24 items-center gap-3 transition-[padding] duration-200 ease-out motion-reduce:transition-none",
				collapsed ? "justify-center px-0" : "px-6",
				className,
			)}
		>
			<Link
				to="/"
				className={cn(
					"group flex min-w-0 items-center gap-3",
					collapsed && "justify-center gap-0",
				)}
				aria-label={branding.title || t("brand.name")}
			>
				{collapsed ? (
					<img
						src="/favicon.svg"
						alt=""
						className="size-13 shrink-0 object-contain"
					/>
				) : (
					<BrandMark
						branding={branding}
						className="size-11 shrink-0 object-contain"
						wordmarkClassName="h-10 max-w-44"
					/>
				)}
				<span
					className={cn(
						"min-w-0 transition-[opacity,translate] duration-150 ease-out motion-reduce:transition-none",
						collapsed
							? "pointer-events-none w-0 overflow-hidden -translate-x-1 opacity-0"
							: "translate-x-0 opacity-100",
					)}
				>
					<span className="block truncate text-lg font-semibold">
						{branding.title || t("brand.name")}
					</span>
				</span>
			</Link>
		</div>
	);
}

export function ShellSidebarNavigation({
	className,
	collapsed = false,
	navSections,
	onNavigate,
}: {
	className?: string;
	collapsed?: boolean;
	navSections: ReturnType<typeof getShellNavSections>;
	onNavigate?: () => void;
}) {
	const { t } = useTranslation();

	return (
		<TooltipProvider delay={100}>
			<nav
				className={cn(
					"flex flex-col gap-6",
					collapsed && "items-center",
					className,
				)}
			>
				{navSections.map((section) => (
					<div
						key={section.id}
						className={cn("grid gap-2", collapsed && "justify-items-center")}
					>
						<div
							className={cn(
								"px-4 text-[11px] font-semibold tracking-wide text-emerald-100/55 uppercase",
								collapsed && "sr-only",
							)}
						>
							{t(section.labelKey)}
						</div>
						{section.items.map((item) => (
							<ShellSidebarNavLink
								key={item.to}
								item={item}
								label={t(item.labelKey)}
								onNavigate={onNavigate}
								collapsed={collapsed}
							/>
						))}
					</div>
				))}
			</nav>
		</TooltipProvider>
	);
}

export function ShellSidebarNavLink({
	collapsed = false,
	item,
	label,
	onNavigate,
}: {
	collapsed?: boolean;
	item: ShellNavItem;
	label: string;
	onNavigate?: () => void;
}) {
	const link = (
		<NavLink
			to={item.to}
			end={item.end}
			aria-label={label}
			onFocus={() => {
				void item.preload?.();
			}}
			onMouseEnter={() => {
				void item.preload?.();
			}}
			onClick={onNavigate}
			className={({ isActive }) =>
				cn(
					"flex min-h-11 items-center rounded-lg px-4 text-sm font-semibold transition-[background-color,color,box-shadow,width] duration-200 ease-out motion-reduce:transition-none",
					collapsed
						? "mx-auto size-13 min-h-13 justify-center gap-0 px-0"
						: "w-full justify-start gap-3 px-4",
					isActive
						? "bg-white/12 text-white shadow-lg shadow-black/15"
						: "text-emerald-50/82 hover:bg-white/8 hover:text-white hover:shadow-sm hover:shadow-black/10",
				)
			}
		>
			<span
				className={cn(
					"grid shrink-0 place-items-center transition-[width,height] duration-200 ease-out motion-reduce:transition-none",
					collapsed ? "size-5" : "size-4",
				)}
			>
				<Icon name={item.icon} className="block size-full" />
			</span>
			<span
				className={cn(
					"min-w-0 truncate transition-[opacity,translate] duration-150 ease-out motion-reduce:transition-none",
					collapsed
						? "pointer-events-none w-0 overflow-hidden -translate-x-1 opacity-0"
						: "translate-x-0 opacity-100",
				)}
			>
				{label}
			</span>
		</NavLink>
	);

	if (!collapsed) {
		return link;
	}

	return (
		<Tooltip>
			<TooltipTrigger render={link} />
			<TooltipContent
				side="right"
				className="hidden border-white/10 bg-[#0b271b] text-white shadow-xl shadow-black/20 lg:block"
			>
				{label}
			</TooltipContent>
		</Tooltip>
	);
}
