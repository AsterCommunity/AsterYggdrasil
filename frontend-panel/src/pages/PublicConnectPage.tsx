import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { AuthUserMenu } from "@/components/common/AuthUserMenu";
import { AppFooter } from "@/components/layout/AppFooter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/buttonVariants";
import { Icon } from "@/components/ui/icon";
import { usePageTitle } from "@/hooks/usePageTitle";
import { cn } from "@/lib/utils";
import { accountPaths, publicPaths } from "@/routes/routePaths";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

const featureKeys = [
	{
		id: "secure",
		icon: "Shield",
		title: "home.featureSecureTitle",
		description: "home.featureSecureDescription",
	},
	{
		id: "textures",
		icon: "FileImage",
		title: "home.featureTexturesTitle",
		description: "home.featureTexturesDescription",
	},
	{
		id: "auth",
		icon: "WifiHigh",
		title: "home.featureStableTitle",
		description: "home.featureStableDescription",
	},
] as const;

const audienceKeys = [
	{
		id: "server",
		icon: "Queue",
		title: "home.audienceServerTitle",
		description: "home.audienceServerDescription",
	},
	{
		id: "developer",
		icon: "BracketsCurly",
		title: "home.audienceDeveloperTitle",
		description: "home.audienceDeveloperDescription",
	},
	{
		id: "skin",
		icon: "FileImage",
		title: "home.audienceSkinTitle",
		description: "home.audienceSkinDescription",
	},
	{
		id: "community",
		icon: "User",
		title: "home.audienceCommunityTitle",
		description: "home.audienceCommunityDescription",
	},
] as const;

export default function PublicConnectPage() {
	const { t } = useTranslation();
	const branding = useFrontendConfigStore((state) => state.branding);
	const user = useAuthStore((state) => state.user);
	const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
	const hydrate = useAuthStore((state) => state.hydrate);
	const logout = useAuthStore((state) => state.logout);
	const serverName = branding.title || t("home.titleFallback");
	const heroCopy = branding.description || t("home.heroCopy");
	usePageTitle(serverName);

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
			<div className="relative min-h-[calc(100svh-5rem)] overflow-hidden">
				<main className="relative z-10 mx-auto flex min-h-[calc(100svh-5rem)] max-w-[92rem] flex-col justify-between gap-10 px-4 pt-10 pb-8 sm:px-8 lg:px-12 lg:pt-16">
					<section className="grid items-center">
						<div className="public-home-enter max-w-4xl">
							<Badge className="rounded-full border-emerald-700/20 bg-emerald-600/12 px-3 py-1 text-emerald-800 shadow-lg shadow-black/10 dark:border-emerald-300/24 dark:bg-emerald-400/14 dark:text-emerald-100">
								<Icon name="SignIn" className="size-3.5" />
								{t("home.heroEyebrow")}
							</Badge>
							<h1 className="mt-6 max-w-3xl text-balance font-black text-5xl leading-[0.98] tracking-normal sm:text-7xl lg:text-8xl">
								{t("home.heroTitlePrefix")}{" "}
								<span className="text-emerald-700 drop-shadow-[0_0_28px_rgba(22,163,74,0.16)] dark:text-emerald-300 dark:drop-shadow-[0_0_28px_rgba(52,211,153,0.28)]">
									{t("home.heroTitleAccent")}
								</span>
							</h1>
							<p className="mt-6 max-w-2xl text-base leading-7 text-slate-700 sm:text-lg dark:text-slate-200">
								{heroCopy}
							</p>
							<div className="mt-8 flex flex-wrap gap-3">
								{isAuthenticated ? null : (
									<Link
										to={publicPaths.login}
										className={cn(
											buttonVariants({ size: "lg" }),
											"h-12 min-w-44 rounded-lg border-emerald-300/28 bg-emerald-500 px-5 text-white shadow-xl shadow-emerald-950/40 hover:bg-emerald-400",
										)}
									>
										<Icon name="SignIn" className="size-5" />
										{t("home.primaryAction")}
									</Link>
								)}
								<Link
									to={publicPaths.textureLibrary}
									className={cn(
										buttonVariants({ variant: "outline", size: "lg" }),
										"h-12 min-w-40 rounded-lg border-black/12 bg-white/70 px-5 text-[#102118] backdrop-blur hover:border-black/18 hover:bg-white/85 dark:border-white/22 dark:bg-white/7 dark:text-white dark:hover:border-white/38 dark:hover:bg-white/13",
									)}
								>
									<Icon name="Images" className="size-5" />
									{t("home.textureLibraryAction")}
								</Link>
								{isAuthenticated ? (
									<Link
										to={accountPaths.home}
										className={cn(
											buttonVariants({ variant: "outline", size: "lg" }),
											"h-12 min-w-40 rounded-lg border-black/12 bg-white/70 px-5 text-[#102118] backdrop-blur hover:border-black/18 hover:bg-white/85 dark:border-white/22 dark:bg-white/7 dark:text-white dark:hover:border-white/38 dark:hover:bg-white/13",
										)}
									>
										<Icon name="Gauge" className="size-5" />
										{t("home.consoleAction")}
									</Link>
								) : null}
							</div>
							<div className="mt-10 grid max-w-3xl gap-4 sm:grid-cols-3">
								{featureKeys.map((feature, index) => (
									<HeroFeature
										key={feature.id}
										icon={feature.icon}
										title={t(feature.title)}
										description={t(feature.description)}
										delayMs={120 + index * 70}
									/>
								))}
							</div>
						</div>
					</section>

					<section
						className="public-home-enter pt-2"
						style={{ animationDelay: "260ms" }}
					>
						<div className="text-center">
							<h2 className="text-2xl font-semibold tracking-normal sm:text-3xl">
								{t("home.audienceHeadlinePrefix")}{" "}
								<span className="text-emerald-700 dark:text-emerald-300">
									{t("home.audienceHeadlineAccent")}
								</span>
							</h2>
							<p className="mx-auto mt-3 max-w-3xl text-sm leading-6 text-slate-700 dark:text-slate-300">
								{t("home.audienceDescription")}
							</p>
						</div>
						<div className="mx-auto mt-7 grid max-w-6xl gap-4 md:grid-cols-2 xl:grid-cols-4">
							{audienceKeys.map((feature) => (
								<AudienceTile
									key={feature.id}
									icon={feature.icon}
									title={t(feature.title)}
									description={t(feature.description)}
								/>
							))}
						</div>
					</section>
				</main>
			</div>
		</PublicEntryShell>
	);
}

function HeroFeature({
	icon,
	title,
	description,
	delayMs,
}: {
	icon: (typeof featureKeys)[number]["icon"];
	title: string;
	description: string;
	delayMs: number;
}) {
	return (
		<div
			className="public-home-enter grid grid-cols-[auto_minmax(0,1fr)] gap-3"
			style={{ animationDelay: `${delayMs}ms` }}
		>
			<div className="flex size-11 items-center justify-center rounded-xl border border-emerald-700/16 bg-emerald-600/10 text-emerald-700 shadow-lg shadow-black/10 backdrop-blur-md dark:border-emerald-300/18 dark:bg-emerald-400/12 dark:text-emerald-200">
				<Icon name={icon} className="size-5" />
			</div>
			<div className="min-w-0">
				<div className="font-semibold text-[#102118] dark:text-white">
					{title}
				</div>
				<p className="mt-1 text-sm leading-5 text-slate-700 dark:text-slate-300">
					{description}
				</p>
			</div>
		</div>
	);
}

function AudienceTile({
	icon,
	title,
	description,
}: {
	icon: (typeof audienceKeys)[number]["icon"];
	title: string;
	description: string;
}) {
	return (
		<div className="group rounded-lg border border-black/10 bg-white/58 p-5 shadow-xl shadow-black/10 backdrop-blur-md transition-[transform,background-color,border-color,box-shadow] duration-300 ease-out hover:-translate-y-1 hover:border-emerald-700/18 hover:bg-white/76 hover:shadow-2xl hover:shadow-emerald-950/12 dark:border-white/10 dark:bg-white/[0.06] dark:shadow-black/18 dark:hover:border-emerald-300/24 dark:hover:bg-white/[0.085] dark:hover:shadow-black/24">
			<div className="mb-4 flex size-11 items-center justify-center rounded-md border border-emerald-700/16 bg-emerald-600/10 text-emerald-700 transition-[transform,background-color,border-color,color] duration-300 ease-out group-hover:scale-105 group-hover:border-emerald-700/24 group-hover:bg-emerald-600/14 dark:border-emerald-300/16 dark:bg-emerald-400/12 dark:text-emerald-200 dark:group-hover:border-emerald-300/26 dark:group-hover:bg-emerald-400/16">
				<Icon name={icon} className="size-6" />
			</div>
			<h3 className="text-base font-semibold text-[#102118] dark:text-white">
				{title}
			</h3>
			<p className="mt-2 text-sm leading-6 text-slate-700 dark:text-slate-300">
				{description}
			</p>
		</div>
	);
}
