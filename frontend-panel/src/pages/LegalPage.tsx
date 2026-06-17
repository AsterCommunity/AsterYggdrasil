import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { AuthUserMenu } from "@/components/common/AuthUserMenu";
import { AppFooter } from "@/components/layout/AppFooter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/buttonVariants";
import { Icon, type IconName } from "@/components/ui/icon";
import { usePageTitle } from "@/hooks/usePageTitle";
import { cn } from "@/lib/utils";
import { publicPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

type LegalPageKind = "tos" | "privacy";

type LegalSection = {
	id: string;
	icon: IconName;
	titleKey: string;
	bodyKeys: readonly string[];
};

type LegalPageConfig = {
	icon: IconName;
	pageTitleKey: string;
	badgeKey: string;
	headingKey: string;
	leadKey: string;
	sections: readonly LegalSection[];
};

const legalPages = {
	tos: {
		icon: "Scroll",
		pageTitleKey: "legal.tos.pageTitle",
		badgeKey: "legal.tos.badge",
		headingKey: "legal.tos.heading",
		leadKey: "legal.tos.lead",
		sections: [
			{
				id: "scope",
				icon: "Globe",
				titleKey: "legal.tos.sections.scope.title",
				bodyKeys: [
					"legal.tos.sections.scope.body.0",
					"legal.tos.sections.scope.body.1",
				],
			},
			{
				id: "account",
				icon: "User",
				titleKey: "legal.tos.sections.account.title",
				bodyKeys: [
					"legal.tos.sections.account.body.0",
					"legal.tos.sections.account.body.1",
				],
			},
			{
				id: "textures",
				icon: "FileImage",
				titleKey: "legal.tos.sections.textures.title",
				bodyKeys: [
					"legal.tos.sections.textures.body.0",
					"legal.tos.sections.textures.body.1",
				],
			},
			{
				id: "launcher",
				icon: "WifiHigh",
				titleKey: "legal.tos.sections.launcher.title",
				bodyKeys: [
					"legal.tos.sections.launcher.body.0",
					"legal.tos.sections.launcher.body.1",
				],
			},
			{
				id: "operations",
				icon: "Wrench",
				titleKey: "legal.tos.sections.operations.title",
				bodyKeys: [
					"legal.tos.sections.operations.body.0",
					"legal.tos.sections.operations.body.1",
				],
			},
			{
				id: "changes",
				icon: "ClipboardText",
				titleKey: "legal.tos.sections.changes.title",
				bodyKeys: [
					"legal.tos.sections.changes.body.0",
					"legal.tos.sections.changes.body.1",
				],
			},
		],
	},
	privacy: {
		icon: "Shield",
		pageTitleKey: "legal.privacy.pageTitle",
		badgeKey: "legal.privacy.badge",
		headingKey: "legal.privacy.heading",
		leadKey: "legal.privacy.lead",
		sections: [
			{
				id: "collected",
				icon: "ListChecks",
				titleKey: "legal.privacy.sections.collected.title",
				bodyKeys: [
					"legal.privacy.sections.collected.body.0",
					"legal.privacy.sections.collected.body.1",
				],
			},
			{
				id: "use",
				icon: "Gear",
				titleKey: "legal.privacy.sections.use.title",
				bodyKeys: [
					"legal.privacy.sections.use.body.0",
					"legal.privacy.sections.use.body.1",
				],
			},
			{
				id: "public",
				icon: "FileImage",
				titleKey: "legal.privacy.sections.public.title",
				bodyKeys: [
					"legal.privacy.sections.public.body.0",
					"legal.privacy.sections.public.body.1",
				],
			},
			{
				id: "security",
				icon: "Lock",
				titleKey: "legal.privacy.sections.security.title",
				bodyKeys: [
					"legal.privacy.sections.security.body.0",
					"legal.privacy.sections.security.body.1",
				],
			},
			{
				id: "sharing",
				icon: "LinkSimple",
				titleKey: "legal.privacy.sections.sharing.title",
				bodyKeys: [
					"legal.privacy.sections.sharing.body.0",
					"legal.privacy.sections.sharing.body.1",
				],
			},
			{
				id: "retention",
				icon: "Clock",
				titleKey: "legal.privacy.sections.retention.title",
				bodyKeys: [
					"legal.privacy.sections.retention.body.0",
					"legal.privacy.sections.retention.body.1",
				],
			},
		],
	},
} satisfies Record<LegalPageKind, LegalPageConfig>;

export function LegalPage({ kind }: { kind: LegalPageKind }) {
	const { t } = useTranslation();
	const config = legalPages[kind];
	const branding = useFrontendConfigStore((state) => state.branding);
	const user = useAuthStore((state) => state.user);
	const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
	const hydrate = useAuthStore((state) => state.hydrate);
	const logout = useAuthStore((state) => state.logout);
	const serverName = branding.title || t("home.titleFallback");
	const pageTitle = t(config.pageTitleKey);
	usePageTitle(pageTitle);

	useEffect(() => {
		void hydrate();
	}, [hydrate]);

	return (
		<PublicEntryShell
			branding={branding}
			title={serverName}
			tagline={t("brand.tagline")}
			variant="home"
			hideLanguageOnMobile
			headerActions={
				isAuthenticated && user ? (
					<AuthUserMenu
						user={user}
						scope="public"
						className="border-black/10 bg-white/64 text-[#102118] shadow-lg shadow-black/12 backdrop-blur hover:bg-white/80 aria-expanded:bg-white/80 dark:border-white/14 dark:bg-white/8 dark:text-white dark:shadow-black/20 dark:hover:bg-white/14 dark:aria-expanded:bg-white/14"
						onLogout={() => void logout()}
					/>
				) : (
					<Link
						to={publicPaths.login}
						className={cn(
							buttonVariants({ variant: "default", size: "sm" }),
							"h-10 rounded-lg border-emerald-300/24 bg-emerald-500 px-3 text-white shadow-lg shadow-emerald-950/35 hover:bg-emerald-400 sm:px-4",
						)}
					>
						<Icon name="SignIn" className="size-4" />
						<span className="hidden sm:inline">{t("home.loginRegister")}</span>
					</Link>
				)
			}
			footer={<AppFooter />}
		>
			<main className="relative z-10 mx-auto grid w-full max-w-[92rem] gap-8 px-4 pt-8 pb-14 sm:px-8 lg:grid-cols-[20rem_minmax(0,1fr)] lg:px-12 lg:pt-14">
				<aside className="lg:sticky lg:top-8 lg:self-start">
					<div className="rounded-lg border border-black/10 bg-white/70 p-5 shadow-xl shadow-black/10 backdrop-blur-xl dark:border-white/10 dark:bg-white/[0.07] dark:shadow-black/24">
						<Badge className="rounded-full border-emerald-700/20 bg-emerald-600/12 px-3 py-1 text-emerald-800 dark:border-emerald-300/24 dark:bg-emerald-400/14 dark:text-emerald-100">
							<Icon name={config.icon} className="size-3.5" />
							{t(config.badgeKey)}
						</Badge>
						<h1 className="mt-5 text-balance font-black text-4xl leading-none tracking-normal text-[#102118] sm:text-5xl dark:text-white">
							{t(config.headingKey)}
						</h1>
						<p className="mt-5 text-sm leading-6 text-slate-700 dark:text-slate-300">
							{t(config.leadKey)}
						</p>
						<div className="mt-5 flex items-center gap-2 text-slate-600 text-xs dark:text-slate-400">
							<Icon name="Clock" className="size-4" />
							<span>
								{t("legal.common.lastUpdatedLabel")}{" "}
								{t("legal.common.lastUpdatedValue")}
							</span>
						</div>
					</div>

					<nav
						aria-label={t("legal.common.contents")}
						className="mt-4 rounded-lg border border-black/10 bg-white/60 p-2 shadow-lg shadow-black/8 backdrop-blur-xl dark:border-white/10 dark:bg-white/[0.055]"
					>
						{config.sections.map((section) => (
							<a
								key={section.id}
								href={`#${section.id}`}
								className="flex items-center gap-3 rounded-md px-3 py-2.5 text-sm text-slate-700 transition-colors hover:bg-emerald-600/10 hover:text-emerald-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40 dark:text-slate-300 dark:hover:bg-emerald-400/10 dark:hover:text-emerald-100"
							>
								<Icon
									name={section.icon}
									className="size-4 shrink-0 text-emerald-700 dark:text-emerald-300"
								/>
								<span className="min-w-0 truncate">{t(section.titleKey)}</span>
							</a>
						))}
					</nav>
				</aside>

				<section className="min-w-0 rounded-lg border border-black/10 bg-white/78 p-5 shadow-2xl shadow-black/10 backdrop-blur-xl sm:p-8 lg:p-10 dark:border-white/10 dark:bg-[#07110d]/82 dark:shadow-black/28">
					<div className="mb-8 border-black/10 border-b pb-6 dark:border-white/10">
						<div className="flex flex-wrap items-center gap-3 text-slate-600 text-sm dark:text-slate-400">
							<Link
								to={publicPaths.home}
								className="inline-flex items-center gap-2 rounded-md text-slate-700 transition-colors hover:text-emerald-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40 dark:text-slate-300 dark:hover:text-emerald-300"
							>
								<Icon name="House" className="size-4" />
								{t("nav.home")}
							</Link>
							<span aria-hidden="true">/</span>
							<span className="font-medium text-[#102118] dark:text-white">
								{pageTitle}
							</span>
						</div>
					</div>

					<div className="grid gap-8">
						{config.sections.map((section) => (
							<article key={section.id} id={section.id} className="scroll-mt-8">
								<div className="flex items-start gap-4">
									<div className="flex size-11 shrink-0 items-center justify-center rounded-md border border-emerald-700/16 bg-emerald-600/10 text-emerald-700 dark:border-emerald-300/16 dark:bg-emerald-400/12 dark:text-emerald-200">
										<Icon name={section.icon} className="size-5" />
									</div>
									<div className="min-w-0">
										<h2 className="text-pretty font-semibold text-2xl text-[#102118] tracking-normal dark:text-white">
											{t(section.titleKey)}
										</h2>
										<div className="mt-3 grid gap-3 text-base leading-7 text-slate-700 dark:text-slate-300">
											{section.bodyKeys.map((bodyKey) => (
												<p key={bodyKey}>{t(bodyKey)}</p>
											))}
										</div>
									</div>
								</div>
							</article>
						))}
					</div>
				</section>
			</main>
		</PublicEntryShell>
	);
}
