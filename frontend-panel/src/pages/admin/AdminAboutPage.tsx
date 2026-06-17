import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { BrandMark } from "@/components/layout/BrandMark";
import { Badge } from "@/components/ui/badge";
import { Icon, type IconName } from "@/components/ui/icon";
import { config } from "@/config/app";
import { usePageTitle } from "@/hooks/usePageTitle";
import { formatDateTimeOrFallback } from "@/lib/dateTime";
import { cn } from "@/lib/utils";
import { adminSystemService } from "@/services/adminService";
import { formatUnknownError } from "@/services/http";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { SystemInfoResponse } from "@/types/api";

const REPOSITORY_URL = "https://github.com/AsterCommunity/AsterYggdrasil";
const README_URL = `${REPOSITORY_URL}#readme`;
const LICENSE_URL = `${REPOSITORY_URL}/blob/master/LICENSE`;

function formatVersion(version: string) {
	if (!version || version === "unknown") return "unknown";
	if (version === "dev") return "dev";
	return version.startsWith("v") ? version : `v${version}`;
}

function releaseChannel(version: string) {
	const normalized = version.toLowerCase();
	if (!version || normalized === "unknown") return "unknown";
	if (normalized === "dev" || normalized.includes("dev")) return "development";
	if (normalized.includes("alpha")) return "alpha";
	if (normalized.includes("beta")) return "beta";
	if (normalized.includes("rc")) return "rc";
	return "release";
}

function channelClassName(channel: string) {
	switch (channel) {
		case "release":
			return "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/60 dark:text-emerald-300";
		case "development":
			return "border-sky-200 bg-sky-50 text-sky-700 dark:border-sky-900 dark:bg-sky-950/60 dark:text-sky-300";
		case "alpha":
		case "beta":
		case "rc":
			return "border-amber-200 bg-amber-50 text-amber-800 dark:border-amber-900 dark:bg-amber-950/60 dark:text-amber-300";
		default:
			return "border-border bg-muted/35 text-muted-foreground";
	}
}

export default function AdminAboutPage() {
	const { t } = useTranslation();
	const branding = useFrontendConfigStore((state) => state.branding);

	usePageTitle(t("admin.aboutPage.title"));

	const [systemInfo, setSystemInfo] = useState<SystemInfoResponse | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);

	const load = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const nextInfo = await adminSystemService.getInfo();
			setSystemInfo(nextInfo);
		} catch (nextError) {
			setSystemInfo(null);
			setError(formatUnknownError(nextError));
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		void load();
	}, [load]);

	const backendVersion = systemInfo?.version ?? "unknown";
	const channel = releaseChannel(backendVersion);
	const buildDetails = useMemo(
		() => [
			{
				label: t("admin.aboutPage.backendVersion"),
				value: formatVersion(backendVersion),
				icon: "Cpu",
			},
			{
				label: t("admin.aboutPage.buildTime"),
				value: systemInfo
					? formatDateTimeOrFallback(
							systemInfo.build_time,
							t("admin.aboutPage.buildTimeUnknown"),
						)
					: t("admin.aboutPage.notLoaded"),
				icon: "Clock",
			},
			{
				label: t("admin.aboutPage.frontendVersion"),
				value: formatVersion(config.appVersion),
				icon: "Monitor",
			},
			{
				label: t("admin.aboutPage.license"),
				value: "MIT",
				icon: "Scroll",
			},
		],
		[backendVersion, systemInfo, t],
	);
	const capabilityItems = useMemo(
		() => [
			{
				icon: "Shield",
				title: t("admin.aboutPage.capabilityAuth"),
				description: t("admin.aboutPage.capabilityAuthDesc"),
			},
			{
				icon: "User",
				title: t("admin.aboutPage.capabilityProfiles"),
				description: t("admin.aboutPage.capabilityProfilesDesc"),
			},
			{
				icon: "FileImage",
				title: t("admin.aboutPage.capabilityTextures"),
				description: t("admin.aboutPage.capabilityTexturesDesc"),
			},
			{
				icon: "Gauge",
				title: t("admin.aboutPage.capabilityOperations"),
				description: t("admin.aboutPage.capabilityOperationsDesc"),
			},
		],
		[t],
	);
	const resourceLinks = useMemo(
		() => [
			{
				href: README_URL,
				icon: "FileText" as const,
				label: t("admin.aboutPage.readme"),
				description: t("admin.aboutPage.readmeDesc"),
			},
			{
				href: REPOSITORY_URL,
				icon: "BracketsCurly" as const,
				label: t("admin.aboutPage.repository"),
				description: t("admin.aboutPage.repositoryDesc"),
			},
			{
				href: LICENSE_URL,
				icon: "Scroll" as const,
				label: t("admin.aboutPage.licenseLink"),
				description: t("admin.aboutPage.licenseLinkDesc"),
			},
		],
		[t],
	);

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.aboutPage.title")}
				description={t("admin.aboutPage.description")}
			/>

			{error ? (
				<AdminSurface className="border-destructive/35 bg-destructive/5 text-destructive">
					<div className="flex items-start gap-3">
						<Icon name="Warning" className="mt-0.5 size-5 shrink-0" />
						<div className="min-w-0">
							<h2 className="text-sm font-semibold">
								{t("admin.aboutPage.loadErrorTitle")}
							</h2>
							<p className="mt-1 text-sm [overflow-wrap:anywhere]">{error}</p>
						</div>
					</div>
				</AdminSurface>
			) : null}

			<AdminSurface className="overflow-hidden" padded={false}>
				<div className="grid lg:grid-cols-[minmax(0,1fr)_22rem]">
					<section className="relative border-b bg-muted/15 p-5 md:p-6 lg:border-r lg:border-b-0">
						<div className="absolute inset-x-0 top-0 h-1 bg-linear-to-r from-emerald-500 via-sky-500 to-amber-500" />
						<div className="relative flex flex-col gap-6">
							<div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
								<div className="flex min-w-0 items-center gap-4">
									<div className="grid size-14 shrink-0 place-items-center rounded-xl border border-border/70 bg-background shadow-xs">
										<BrandMark
											branding={branding}
											className="size-10 object-contain"
											wordmarkClassName="h-10 max-w-40 object-contain"
										/>
									</div>
									<div className="min-w-0">
										<h2 className="truncate text-xl font-semibold tracking-normal text-foreground">
											{branding.title || config.appName}
										</h2>
										<p className="mt-1 text-sm leading-5 text-muted-foreground">
											{t("admin.aboutPage.tagline")}
										</p>
									</div>
								</div>
								<div className="flex shrink-0 flex-wrap items-center gap-2">
									<Badge
										variant="outline"
										className="rounded-md bg-background/80"
									>
										{t("admin.aboutPage.productBadge")}
									</Badge>
									<Badge
										variant="outline"
										className={cn("rounded-md", channelClassName(channel))}
									>
										{t(`admin.aboutPage.channel.${channel}`)}
									</Badge>
								</div>
							</div>

							<div className="max-w-3xl">
								<p className="text-base leading-7 text-foreground">
									{t("admin.aboutPage.summary")}
								</p>
								<p className="mt-2 text-sm leading-6 text-muted-foreground">
									{t("admin.aboutPage.metadataNote")}
								</p>
							</div>

							<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
								{buildDetails.map((detail) => (
									<div
										key={detail.label}
										className="min-w-0 rounded-lg border border-border/70 bg-background/75 p-3 shadow-xs dark:border-white/10 dark:shadow-none"
									>
										<div className="flex items-center gap-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
											<Icon name={detail.icon as IconName} className="size-4" />
											{detail.label}
										</div>
										<div className="mt-2 truncate font-mono text-sm font-semibold text-foreground">
											{loading && detail.value === "unknown"
												? t("common.loading")
												: detail.value}
										</div>
									</div>
								))}
							</div>
						</div>
					</section>

					<section className="bg-background p-5 md:p-6">
						<h2 className="text-base font-semibold text-foreground">
							{t("admin.aboutPage.resourcesTitle")}
						</h2>
						<p className="mt-1 text-sm leading-6 text-muted-foreground">
							{t("admin.aboutPage.resourcesDescription")}
						</p>
						<div className="mt-4 grid gap-2">
							{resourceLinks.map((link) => (
								<a
									key={link.href}
									href={link.href}
									target="_blank"
									rel="noreferrer"
									className="flex min-h-16 items-center justify-between gap-3 rounded-lg border border-border/70 bg-muted/20 px-3 py-3 text-left transition-colors hover:bg-muted/45 focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35 dark:border-white/10"
								>
									<span className="flex min-w-0 items-center gap-3">
										<span className="grid size-9 shrink-0 place-items-center rounded-md border border-border/70 bg-background text-muted-foreground dark:border-white/10">
											<Icon name={link.icon} className="size-4" />
										</span>
										<span className="min-w-0">
											<span className="block truncate text-sm font-semibold text-foreground">
												{link.label}
											</span>
											<span className="mt-0.5 block truncate text-xs text-muted-foreground">
												{link.description}
											</span>
										</span>
									</span>
									<Icon
										name="ArrowSquareOut"
										className="size-4 shrink-0 text-muted-foreground"
									/>
								</a>
							))}
						</div>
					</section>
				</div>

				<section className="border-t border-border/70 p-5 md:p-6 dark:border-white/10">
					<div className="mb-4">
						<h2 className="text-base font-semibold text-foreground">
							{t("admin.aboutPage.capabilitiesTitle")}
						</h2>
						<p className="mt-1 text-sm leading-6 text-muted-foreground">
							{t("admin.aboutPage.capabilitiesDescription")}
						</p>
					</div>
					<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
						{capabilityItems.map((item) => (
							<div
								key={item.title}
								className="rounded-lg border border-border/70 bg-background p-4 shadow-xs transition-colors hover:border-primary/35 hover:bg-muted/20 dark:border-white/10 dark:shadow-none"
							>
								<div className="mb-3 grid size-9 place-items-center rounded-md bg-emerald-100 text-emerald-700 dark:bg-emerald-400/15 dark:text-emerald-200">
									<Icon name={item.icon as IconName} className="size-4" />
								</div>
								<h3 className="text-sm font-semibold text-foreground">
									{item.title}
								</h3>
								<p className="mt-1 text-sm leading-5 text-muted-foreground">
									{item.description}
								</p>
							</div>
						))}
					</div>
				</section>
			</AdminSurface>
		</AdminPageShell>
	);
}
