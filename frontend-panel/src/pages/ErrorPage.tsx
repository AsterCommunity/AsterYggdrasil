import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
	isRouteErrorResponse,
	Link,
	useNavigate,
	useRouteError,
} from "react-router-dom";
import { AppFooter } from "@/components/layout/AppFooter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { buttonVariants } from "@/components/ui/buttonVariants";
import type { IconName } from "@/components/ui/icon";
import { Icon } from "@/components/ui/icon";
import { usePageTitle } from "@/hooks/usePageTitle";
import { cn } from "@/lib/utils";
import { publicPaths } from "@/routes/routePaths";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

type RouteErrorState = {
	code: string;
	detail: string;
	isNotFound: boolean;
	status: number | null;
};

const reasonItems = [
	{ id: "address", icon: "Link", label: "errorPage.reasonAddress" },
	{ id: "moved", icon: "ArrowsClockwise", label: "errorPage.reasonMoved" },
	{ id: "permission", icon: "Lock", label: "errorPage.reasonPermission" },
] as const satisfies readonly {
	id: string;
	icon: IconName;
	label: string;
}[];

function readRouteError(error: unknown): RouteErrorState {
	if (isRouteErrorResponse(error)) {
		const isNotFound = error.status === 404;
		return {
			code: isNotFound
				? "ERR_404_NOT_FOUND"
				: `ERR_ROUTE_${error.status || "UNKNOWN"}`,
			detail: `${error.status} ${error.statusText}`.trim(),
			isNotFound,
			status: error.status,
		};
	}

	if (error instanceof Error) {
		return {
			code: "ERR_ROUTE_LOAD_FAILED",
			detail: error.message,
			isNotFound: false,
			status: null,
		};
	}

	return {
		code: "ERR_ROUTE_LOAD_FAILED",
		detail: "Route failed",
		isNotFound: false,
		status: null,
	};
}

export default function ErrorPage() {
	const error = useRouteError();
	const navigate = useNavigate();
	const { t } = useTranslation();
	const [detailsOpen, setDetailsOpen] = useState(false);
	const branding = useFrontendConfigStore((state) => state.branding);
	const errorState = readRouteError(error);
	const serverName = branding.title || t("home.titleFallback");
	const pageTitle = errorState.isNotFound
		? t("errorPage.notFoundTitle")
		: t("errorPage.genericTitle");

	usePageTitle(pageTitle);

	function goBack() {
		if (window.history.length > 1) {
			navigate(-1);
			return;
		}
		navigate(publicPaths.home);
	}

	return (
		<PublicEntryShell
			branding={branding}
			title={serverName}
			tagline={t("brand.tagline")}
			variant="home"
			hideLanguageOnMobile
			footer={<AppFooter />}
		>
			<main className="relative z-10 mx-auto grid min-h-[calc(100svh-5rem)] w-full max-w-[92rem] items-center gap-8 px-4 pt-8 pb-12 sm:px-8 lg:grid-cols-[minmax(0,1fr)_minmax(26rem,34rem)] lg:px-12 lg:pt-12">
				<section className="public-home-enter min-w-0 py-8">
					<div className="font-black text-7xl leading-none text-emerald-500 drop-shadow-[0_0_32px_rgba(16,185,129,0.38)] sm:text-8xl md:text-9xl">
						{errorState.status === 404 ? "404" : "ERR"}
					</div>
					<h1 className="mt-6 max-w-2xl text-balance font-black text-4xl leading-tight tracking-normal text-public-foreground drop-shadow-[0_1px_18px_rgba(255,255,255,0.52)] sm:text-5xl dark:drop-shadow-[0_2px_18px_rgba(0,0,0,0.42)]">
						{pageTitle}
					</h1>
					<p className="mt-5 max-w-2xl text-base leading-7 text-slate-700 drop-shadow-[0_1px_12px_rgba(255,255,255,0.46)] sm:text-lg dark:text-slate-200 dark:drop-shadow-[0_1px_12px_rgba(0,0,0,0.45)]">
						{errorState.isNotFound
							? t("errorPage.notFoundDescription")
							: t("errorPage.genericDescription")}
					</p>

					<div className="mt-8 flex flex-wrap gap-3">
						<Link
							to={publicPaths.home}
							className={cn(
								buttonVariants({ size: "lg" }),
								"h-12 min-w-40 rounded-lg border-emerald-300/28 bg-emerald-500 bg-clip-border px-5 text-white shadow-xl shadow-emerald-950/28 hover:border-emerald-300/36 hover:bg-emerald-400 hover:text-white hover:shadow-emerald-950/20 active:border-emerald-300/32 active:bg-emerald-500 dark:border-emerald-300/28 dark:shadow-emerald-950/40 dark:hover:border-emerald-300/42 dark:hover:bg-emerald-400",
							)}
						>
							<Icon name="ArrowLeft" className="size-5" />
							{t("errorPage.backHome")}
						</Link>
						<button
							type="button"
							className={cn(
								buttonVariants({ variant: "outline", size: "lg" }),
								"h-12 min-w-40 rounded-lg border-black/12 bg-white/70 px-5 text-public-foreground shadow-lg shadow-black/10 backdrop-blur-md hover:border-black/18 hover:bg-white/85 dark:border-white/18 dark:bg-white/8 dark:text-white dark:shadow-black/20 dark:hover:border-white/32 dark:hover:bg-white/14 dark:hover:text-white",
							)}
							onClick={goBack}
						>
							<Icon name="Undo" className="size-5" />
							{t("errorPage.backPrevious")}
						</button>
					</div>

					<div className="mt-9 flex flex-wrap items-center gap-3 text-sm text-slate-700 dark:text-slate-200/76">
						<span>{t("errorPage.errorCode")}</span>
						<code className="rounded-md border border-emerald-700/16 bg-emerald-600/10 px-2.5 py-1 font-mono text-emerald-800 shadow-lg shadow-black/10 backdrop-blur dark:border-emerald-300/18 dark:bg-emerald-950/28 dark:text-emerald-200 dark:shadow-black/15">
							{errorState.code}
						</code>
					</div>
				</section>

				<aside
					className="public-home-enter min-w-0 rounded-2xl border border-black/10 bg-public-card-strong p-4 text-public-foreground shadow-2xl shadow-black/12 backdrop-blur-xl sm:p-5 dark:border-white/12 dark:shadow-black/38"
					style={{ animationDelay: "120ms" }}
					aria-label={t("errorPage.supportPanel")}
				>
					<div className="relative min-h-56 overflow-hidden rounded-xl border border-emerald-700/14 shadow-inner shadow-emerald-950/12 sm:min-h-64 dark:border-emerald-300/10 dark:shadow-emerald-950/50">
						<img
							src="/static/images/error.png"
							alt=""
							className="absolute inset-0 size-full object-cover"
						/>
						<div
							data-theme-surface="public-backdrop"
							className="absolute inset-0 bg-[radial-gradient(circle_at_58%_26%,transparent_0%,rgba(237,244,237,0.04)_42%,rgba(3,9,7,0.32)_100%)] opacity-100 dark:opacity-0"
						/>
						<div
							data-theme-surface="public-backdrop"
							className="absolute inset-0 bg-[radial-gradient(circle_at_58%_26%,transparent_0%,rgba(3,9,7,0.08)_42%,rgba(3,9,7,0.48)_100%)] opacity-0 dark:opacity-100"
						/>
					</div>

					<div className="mt-4 rounded-xl border border-black/10 bg-public-card p-5 dark:border-white/10">
						<h2 className="text-base font-semibold text-public-foreground">
							{t("errorPage.reasonTitle")}
						</h2>
						<ul className="mt-4 grid gap-3 text-sm text-slate-700 dark:text-slate-200/82">
							{reasonItems.map((item) => (
								<li key={item.id} className="flex items-center gap-3">
									<Icon
										name={item.icon}
										className="size-5 shrink-0 text-emerald-700 dark:text-emerald-300"
									/>
									<span>{t(item.label)}</span>
								</li>
							))}
						</ul>
					</div>

					<div className="mt-3 rounded-xl border border-black/10 bg-public-card p-5 dark:border-white/10">
						<h2 className="text-base font-semibold text-public-foreground">
							{t("errorPage.helpTitle")}
						</h2>
						<p className="mt-2 text-sm leading-6 text-slate-700 dark:text-slate-200/76">
							{t("errorPage.helpDescription")}
						</p>
						<div className="mt-3 overflow-hidden rounded-lg border border-black/10 bg-public-card dark:border-white/10">
							<button
								type="button"
								className="flex w-full items-center justify-between gap-3 px-3 py-2.5 text-sm font-medium text-public-foreground outline-none transition-colors hover:text-emerald-700 focus-visible:ring-2 focus-visible:ring-emerald-600/30 dark:text-slate-100 dark:hover:text-emerald-200 dark:focus-visible:ring-emerald-300/45"
								aria-expanded={detailsOpen}
								aria-controls="route-error-detail"
								onClick={() => setDetailsOpen((open) => !open)}
							>
								<span className="flex min-w-0 items-center gap-2">
									<Icon
										name="CircleAlert"
										className="size-4 shrink-0 text-emerald-700 dark:text-emerald-300"
									/>
									<span>
										{detailsOpen
											? t("errorPage.detailCollapse")
											: t("errorPage.detailExpand")}
									</span>
								</span>
								<Icon
									name="CaretDown"
									className={cn(
										"size-4 shrink-0 text-slate-600 transition-transform duration-200 dark:text-slate-300",
										detailsOpen && "rotate-180",
									)}
								/>
							</button>
							<div
								id="route-error-detail"
								className={cn(
									"grid transition-[grid-template-rows,opacity] duration-200 ease-out",
									detailsOpen
										? "grid-rows-[1fr] opacity-100"
										: "grid-rows-[0fr] opacity-0",
								)}
							>
								<div className="min-h-0 overflow-hidden">
									<div className="border-white/10 border-t px-3 py-3">
										<pre className="max-h-32 overflow-auto rounded-md border border-emerald-700/14 bg-emerald-600/8 p-3 font-mono text-[0.72rem] leading-5 text-emerald-900 dark:border-emerald-300/12 dark:bg-emerald-950/24 dark:text-emerald-100/90">
											{errorState.detail}
										</pre>
									</div>
								</div>
							</div>
						</div>
					</div>
				</aside>
			</main>
		</PublicEntryShell>
	);
}
