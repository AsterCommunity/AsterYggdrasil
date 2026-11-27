import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { LanguageMenu } from "@/components/common/LanguageMenu";
import { BrandMark } from "@/components/layout/BrandMark";
import { ThemeToggleButton } from "@/components/layout/ThemeToggleButton";
import type { AppliedBranding } from "@/lib/branding";

type PublicEntryShellProps = {
	branding: AppliedBranding;
	title: string;
	tagline: string;
	variant: "home" | "auth";
	children: ReactNode;
	headerActions?: ReactNode;
	footer?: ReactNode;
	hideLanguageOnMobile?: boolean;
};

export function PublicEntryShell({
	branding,
	title,
	tagline,
	variant,
	children,
	headerActions,
	footer,
	hideLanguageOnMobile = false,
}: PublicEntryShellProps) {
	return (
		<div
			data-theme-surface="public-shell"
			className="relative min-h-dvh min-w-0 overflow-x-clip bg-public-surface text-public-foreground"
		>
			<PublicEntryBackdrop variant={variant} />
			<div className="relative z-10 flex min-h-dvh flex-col">
				<header
					data-theme-surface="chrome"
					className="mx-auto flex h-20 w-full min-w-0 max-w-[92rem] items-center justify-between gap-4 px-4 sm:px-8 lg:px-12"
				>
					<Link to="/" className="flex min-w-0 flex-1 items-center gap-3">
						<BrandMark
							branding={branding}
							className="size-10 shrink-0 object-contain"
							wordmarkClassName="h-10 max-w-52"
						/>
						<span className="min-w-0">
							<span className="block truncate font-semibold text-xl text-public-foreground tracking-normal">
								{title}
							</span>
							<span className="block truncate font-medium text-[0.68rem] text-emerald-700 uppercase tracking-[0.18em] dark:text-emerald-300">
								{tagline}
							</span>
						</span>
					</Link>
					<nav className="flex shrink-0 items-center gap-2">
						<ThemeToggleButton tone="hero" />
						<LanguageMenu tone="hero" compactOnMobile={hideLanguageOnMobile} />
						{headerActions}
					</nav>
				</header>
				{children}
			</div>
			{footer ? (
				<div data-theme-surface="public-shell" className="relative z-10">
					{footer}
				</div>
			) : null}
		</div>
	);
}

function PublicEntryBackdrop({ variant }: { variant: "home" | "auth" }) {
	if (variant === "auth") {
		return (
			<>
				<div
					data-slot="public-entry-backdrop-image"
					className="pointer-events-none fixed inset-0 z-0 bg-cover bg-center"
					style={{ backgroundImage: "url('/static/images/home.webp')" }}
				/>
				<div
					data-theme-surface="public-backdrop"
					className="pointer-events-none fixed inset-0 z-0 bg-[radial-gradient(circle_at_76%_26%,rgba(248,213,154,0.26),transparent_28%),radial-gradient(circle_at_57%_31%,rgba(80,230,150,0.14),transparent_23%),linear-gradient(90deg,rgba(245,250,244,0.96)_0%,rgba(239,247,238,0.82)_38%,rgba(236,244,235,0.58)_68%,rgba(244,235,221,0.52)_100%)] opacity-100 dark:opacity-0"
				/>
				<div
					data-theme-surface="public-backdrop"
					className="pointer-events-none fixed inset-0 z-0 bg-[radial-gradient(circle_at_76%_26%,rgba(248,213,154,0.38),transparent_28%),radial-gradient(circle_at_57%_31%,rgba(80,230,150,0.22),transparent_23%),linear-gradient(90deg,rgba(1,11,12,0.95)_0%,rgba(3,17,18,0.76)_38%,rgba(8,18,16,0.52)_68%,rgba(47,31,17,0.42)_100%)] opacity-0 dark:opacity-100"
				/>
				<div
					data-theme-surface="public-backdrop"
					className="pointer-events-none fixed inset-0 z-0 bg-[linear-gradient(180deg,rgba(255,255,255,0.12)_0%,rgba(237,244,237,0.42)_100%)] opacity-100 dark:opacity-0"
				/>
				<div
					data-theme-surface="public-backdrop"
					className="pointer-events-none fixed inset-0 z-0 bg-[linear-gradient(180deg,rgba(0,0,0,0.1)_0%,rgba(0,0,0,0.42)_100%)] opacity-0 dark:opacity-100"
				/>
			</>
		);
	}

	return (
		<>
			<div
				data-slot="public-entry-backdrop-image"
				className="pointer-events-none fixed inset-0 z-0 bg-cover bg-center"
				style={{ backgroundImage: "url('/static/images/home.webp')" }}
			/>
			<div
				data-theme-surface="public-backdrop"
				className="pointer-events-none fixed inset-0 z-0 bg-[radial-gradient(circle_at_70%_16%,rgba(90,191,119,0.16),transparent_25%),linear-gradient(90deg,rgba(245,250,244,0.96)_0%,rgba(239,247,238,0.82)_43%,rgba(236,244,235,0.58)_100%)] opacity-100 dark:opacity-0"
			/>
			<div
				data-theme-surface="public-backdrop"
				className="pointer-events-none fixed inset-0 z-0 bg-[radial-gradient(circle_at_70%_16%,rgba(90,191,119,0.22),transparent_25%),linear-gradient(90deg,rgba(3,8,10,0.94)_0%,rgba(4,11,13,0.78)_43%,rgba(6,18,16,0.50)_100%)] opacity-0 dark:opacity-100"
			/>
			<div
				data-theme-surface="public-backdrop"
				className="pointer-events-none fixed inset-0 z-0 bg-[linear-gradient(180deg,rgba(237,244,237,0.34)_0%,rgba(237,244,237,0.16)_54%,#edf4ed_100%)] opacity-100 dark:opacity-0"
			/>
			<div
				data-theme-surface="public-backdrop"
				className="pointer-events-none fixed inset-0 z-0 bg-[linear-gradient(180deg,rgba(7,17,13,0.24)_0%,rgba(7,17,13,0.12)_54%,#07110d_100%)] opacity-0 dark:opacity-100"
			/>
			<div
				data-theme-surface="public-backdrop"
				className="pointer-events-none fixed inset-x-0 bottom-0 z-0 h-36 bg-gradient-to-t from-[#edf4ed] via-[#edf4ed]/78 to-transparent opacity-100 dark:opacity-0"
			/>
			<div
				data-theme-surface="public-backdrop"
				className="pointer-events-none fixed inset-x-0 bottom-0 z-0 h-36 bg-gradient-to-t from-[#07110d] via-[#07110d]/78 to-transparent opacity-0 dark:opacity-100"
			/>
		</>
	);
}
