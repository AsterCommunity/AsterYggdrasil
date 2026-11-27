import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { ActivityTrendChart } from "@/components/admin/admin-overview-page/OverviewTrendChartContent";
import { DateTimeText } from "@/components/common/DateTimeText";
import { RefreshButton } from "@/components/common/RefreshButton";
import { StatusIndicator } from "@/components/common/StatusIndicator";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon, type IconName } from "@/components/ui/icon";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	formatAuditDetail,
	formatAuditSummary,
	formatAuditTarget,
} from "@/lib/audit";
import { formatDurationSeconds } from "@/lib/dateTime";
import { cn } from "@/lib/utils";
import { adminPaths } from "@/routes/routePaths";
import { adminOverviewService } from "@/services/adminService";
import { formatUnknownError } from "@/services/http";
import { useAuthStore } from "@/stores/authStore";
import type {
	AdminOverview,
	AdminOverviewServiceStatus,
	AdminOverviewSummary,
	AdminOverviewSystemHealthSummary,
	AuditLogEntry,
} from "@/types/api";

const quickActions = [
	{
		to: adminPaths.users,
		titleKey: "admin.overview.quick.users",
		descriptionKey: "admin.overview.quick.usersDesc",
		icon: "User",
	},
	{
		to: adminPaths.settings,
		titleKey: "admin.overview.quick.settings",
		descriptionKey: "admin.overview.quick.settingsDesc",
		icon: "Gear",
	},
	{
		to: adminPaths.externalAuth,
		titleKey: "admin.overview.quick.externalAuth",
		descriptionKey: "admin.overview.quick.externalAuthDesc",
		icon: "SignIn",
	},
	{
		to: adminPaths.audit,
		titleKey: "admin.overview.quick.audit",
		descriptionKey: "admin.overview.quick.auditDesc",
		icon: "ClipboardText",
	},
] as const;

export default function AdminOverviewPage() {
	const { t } = useTranslation();
	const user = useAuthStore((state) => state.user);
	const [overview, setOverview] = useState<AdminOverview | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);

	usePageTitle(t("admin.overview.title"));

	const load = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const nextOverview = await adminOverviewService.get();
			setOverview(nextOverview);
		} catch (nextError) {
			setError(formatUnknownError(nextError));
		} finally {
			setLoading(false);
		}
	}, []);

	useEffect(() => {
		void load();
	}, [load]);

	const username =
		user?.profile?.display_name?.trim() || user?.username?.trim() || "admin";
	const summary = overview?.summary;

	return (
		<AdminPageShell className="gap-5">
			<div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_22rem]">
				<div className="min-w-0 space-y-5">
					<OverviewHero
						error={error}
						health={overview?.system_health}
						username={username}
						loading={loading}
						version={overview?.system_info.version}
					/>
					<SystemHealthBanner
						error={error}
						health={overview?.system_health}
						loading={loading}
						onRetry={() => void load()}
					/>
					<SummaryGrid summary={summary} loading={loading} />
					<div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_23rem]">
						<ActivityTrendChart
							loading={loading}
							trend={overview?.activity_trend ?? []}
						/>
						<RecentActivityPanel
							items={overview?.recent_activity ?? []}
							loading={loading}
						/>
					</div>
				</div>

				<aside className="space-y-5 xl:sticky xl:top-20 xl:self-start">
					<ServiceStatusPanel
						services={overview?.services ?? []}
						loading={loading}
					/>
					<QuickActionsPanel />
					<SystemInfoPanel overview={overview} loading={loading} />
				</aside>
			</div>
		</AdminPageShell>
	);
}

function SystemHealthBanner({
	error,
	health,
	loading,
	onRetry,
}: {
	error?: string | null;
	health?: AdminOverviewSystemHealthSummary;
	loading: boolean;
	onRetry: () => void;
}) {
	const { t } = useTranslation();

	if (loading && !health && !error) {
		return null;
	}

	const currentHealth = error
		? ({
				checked_at: null,
				components: [
					{
						message: error,
						name: "admin_overview",
						status: "unhealthy",
					},
				],
				status: "unhealthy",
				summary: t("admin.overview.loadErrorTitle"),
				task_id: null,
			} satisfies AdminOverviewSystemHealthSummary)
		: (health ??
			({
				checked_at: null,
				components: [],
				status: "unknown",
				summary: null,
				task_id: null,
			} satisfies AdminOverviewSystemHealthSummary));
	const presentation = systemHealthPresentation(currentHealth.status);
	const isIssue =
		currentHealth.status === "degraded" || currentHealth.status === "unhealthy";
	const issueComponents = currentHealth.components.filter(
		(component) => component.status !== "healthy",
	);
	const message = isIssue
		? issueComponents.length > 0
			? t("admin.overview.systemHealth.issueSummary", {
					components: issueComponents
						.map((component) => formatHealthComponentName(t, component.name))
						.join(t("admin.overview.systemHealth.issueSeparator")),
				})
			: currentHealth.summary || t("admin.overview.systemHealth.noSummary")
		: currentHealth.status === "healthy"
			? t("admin.overview.systemHealth.healthyDesc")
			: t("admin.overview.systemHealth.unknownDesc");

	return (
		<AdminSurface className={cn("p-4", presentation.className)}>
			<div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
				<div className="flex min-w-0 items-start gap-3">
					<span
						className={cn(
							"mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg",
							presentation.iconClassName,
						)}
					>
						<Icon name={presentation.icon} className="size-5" />
					</span>
					<div className="min-w-0">
						<div className="flex flex-wrap items-center gap-2">
							<h2 className="text-sm font-semibold">
								{t(presentation.titleKey)}
							</h2>
							<Badge
								variant="outline"
								className="border-current/25 bg-background/45 text-current"
							>
								{currentHealth.checked_at ? (
									<>
										{t("admin.overview.systemHealth.checkedAt")}{" "}
										<DateTimeText value={currentHealth.checked_at} />
									</>
								) : (
									t("admin.overview.systemHealth.notChecked")
								)}
							</Badge>
						</div>
						<p className="mt-1 break-words text-sm text-current/80">
							{message}
						</p>
						{isIssue && issueComponents.length > 0 ? (
							<div className="mt-3 flex flex-wrap gap-2">
								{issueComponents.map((component) => (
									<Badge
										key={component.name}
										variant="outline"
										className="max-w-full border-current/25 bg-background/45 text-current"
										title={component.message || undefined}
									>
										<span className="truncate">
											{t("admin.overview.systemHealth.issueComponent", {
												component: formatHealthComponentName(t, component.name),
												status: formatHealthStatus(t, component.status),
											})}
										</span>
									</Badge>
								))}
							</div>
						) : null}
					</div>
				</div>
				{error ? (
					<RefreshButton
						className="shrink-0 border-current/25 bg-background/45 hover:bg-background/70"
						onClick={onRetry}
					>
						{t("admin.overview.retry")}
					</RefreshButton>
				) : currentHealth.task_id ? (
					<Button
						variant="outline"
						size="sm"
						className="shrink-0 border-current/25 bg-background/45 hover:bg-background/70"
						render={<Link to={adminPaths.tasks} />}
					>
						<Icon name="ArrowSquareOut" className="size-4" />
						{t("admin.overview.systemHealth.viewHistory")}
					</Button>
				) : null}
			</div>
		</AdminSurface>
	);
}

function systemHealthPresentation(
	status: AdminOverviewSystemHealthSummary["status"],
) {
	switch (status) {
		case "healthy":
			return {
				className:
					"border-emerald-500/20 bg-emerald-500/10 text-emerald-950 dark:border-emerald-400/20 dark:bg-emerald-400/10 dark:text-emerald-100",
				icon: "Check" as const,
				iconClassName:
					"bg-emerald-500/12 text-emerald-700 dark:text-emerald-300",
				titleKey: "admin.overview.systemHealth.healthy",
			};
		case "degraded":
			return {
				className:
					"border-amber-500/30 bg-amber-500/10 text-amber-950 dark:border-amber-400/25 dark:bg-amber-400/10 dark:text-amber-100",
				icon: "Warning" as const,
				iconClassName: "bg-amber-500/14 text-amber-700 dark:text-amber-300",
				titleKey: "admin.overview.systemHealth.degraded",
			};
		case "unhealthy":
			return {
				className:
					"border-destructive/35 bg-destructive/10 text-destructive dark:bg-destructive/10",
				icon: "CircleAlert" as const,
				iconClassName: "bg-destructive/12 text-destructive",
				titleKey: "admin.overview.systemHealth.unhealthy",
			};
		case "unknown":
			return {
				className: "border-border/70 bg-muted/35 text-foreground",
				icon: "Info" as const,
				iconClassName: "bg-muted text-muted-foreground",
				titleKey: "admin.overview.systemHealth.unknown",
			};
	}
}

function formatHealthComponentName(
	t: ReturnType<typeof useTranslation>["t"],
	name: string,
) {
	return t(`admin.tasks.runtimeHealth.component.${name}`, {
		defaultValue: humanizeCode(name),
	});
}

function formatHealthStatus(
	t: ReturnType<typeof useTranslation>["t"],
	status: AdminOverviewSystemHealthSummary["status"],
) {
	return t(`admin.tasks.runtimeHealth.status.${status}`, {
		defaultValue: humanizeCode(status),
	});
}

function OverviewHero({
	error,
	health,
	loading,
	username,
	version,
}: {
	error?: string | null;
	health?: AdminOverviewSystemHealthSummary;
	loading: boolean;
	username: string;
	version?: string;
}) {
	const { t } = useTranslation();
	const heroStatus = heroHealthStatus({ error, health, loading });

	return (
		<section className="relative overflow-hidden rounded-xl border border-border/70 bg-card text-card-foreground shadow-sm dark:border-white/10 dark:bg-card/78 dark:shadow-none">
			<div
				className="absolute inset-y-0 right-0 w-[82%] bg-cover bg-center opacity-55 sm:w-[68%] md:opacity-75 dark:opacity-24"
				style={{ backgroundImage: "url('/static/images/home.webp')" }}
			/>
			<div className="absolute inset-0 bg-[linear-gradient(90deg,var(--card)_0%,var(--card)_43%,color-mix(in_oklch,var(--card)_88%,transparent)_64%,color-mix(in_oklch,var(--card)_30%,transparent)_100%)]" />
			<div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-emerald-500/0 via-emerald-500/70 to-sky-500/0" />
			<div className="relative grid min-h-36 gap-4 p-6 sm:p-7 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
				<div className="min-w-0">
					<div
						className={cn(
							"flex items-center gap-2 text-sm font-medium",
							heroStatus.textClassName,
						)}
					>
						<StatusIndicator breathe tone="current" glow />
						{t(heroStatus.labelKey)}
					</div>
					<h1 className="mt-4 max-w-2xl text-3xl font-semibold leading-tight tracking-normal sm:text-4xl">
						{t("admin.overview.welcome", { username })}
					</h1>
					<p className="mt-3 max-w-xl text-sm leading-6 text-muted-foreground sm:text-base">
						{t("admin.overview.description")}
					</p>
				</div>
				<div className="flex flex-wrap items-center gap-2 lg:justify-end">
					<span className="rounded-md border border-border/70 bg-background/75 px-3 py-2 text-xs font-medium text-muted-foreground backdrop-blur">
						{t("admin.overview.instanceVersion", {
							version: formatVersion(version),
						})}
					</span>
				</div>
			</div>
		</section>
	);
}

function heroHealthStatus({
	error,
	health,
	loading,
}: {
	error?: string | null;
	health?: AdminOverviewSystemHealthSummary;
	loading: boolean;
}) {
	if (loading && !health && !error) {
		return {
			labelKey: "admin.overview.loadingStatus",
			textClassName: "text-sky-700 dark:text-sky-300",
		};
	}
	if (error) {
		return {
			labelKey: "admin.overview.systemHealth.unhealthy",
			textClassName: "text-destructive",
		};
	}

	switch (health?.status ?? "unknown") {
		case "healthy":
			return {
				labelKey: "admin.overview.runningStatus",
				textClassName: "text-emerald-700 dark:text-emerald-300",
			};
		case "degraded":
			return {
				labelKey: "admin.overview.systemHealth.degraded",
				textClassName: "text-amber-700 dark:text-amber-300",
			};
		case "unhealthy":
			return {
				labelKey: "admin.overview.systemHealth.unhealthy",
				textClassName: "text-destructive",
			};
		case "unknown":
			return {
				labelKey: "admin.overview.systemHealth.unknown",
				textClassName: "text-muted-foreground",
			};
	}
}

function SummaryGrid({
	loading,
	summary,
}: {
	loading: boolean;
	summary?: AdminOverviewSummary;
}) {
	const { t } = useTranslation();
	const stats = useMemo(
		() => [
			{
				key: "users",
				label: t("admin.overview.stats.users"),
				value: summary?.total_users,
				icon: "User",
				accent: "emerald",
			},
			{
				key: "profiles",
				label: t("admin.overview.stats.profiles"),
				value: summary?.minecraft_profile_count,
				icon: "Grid",
				accent: "sky",
			},
			{
				key: "textures",
				label: t("admin.overview.stats.textures"),
				value: summary?.texture_count,
				icon: "FileImage",
				accent: "violet",
			},
			{
				key: "tokens",
				label: t("admin.overview.stats.tokens"),
				value: summary?.active_yggdrasil_token_count,
				icon: "Key",
				accent: "amber",
			},
		],
		[summary, t],
	);

	return (
		<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
			{stats.map((stat) => (
				<AdminSurface key={stat.key} className="p-4">
					<div className="flex items-center gap-3">
						<div
							className={cn(
								"grid size-10 shrink-0 place-items-center rounded-lg",
								stat.accent === "emerald" &&
									"bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
								stat.accent === "sky" &&
									"bg-sky-500/10 text-sky-700 dark:text-sky-300",
								stat.accent === "violet" &&
									"bg-violet-500/10 text-violet-700 dark:text-violet-300",
								stat.accent === "amber" &&
									"bg-amber-500/12 text-amber-700 dark:text-amber-300",
							)}
						>
							<Icon name={stat.icon as IconName} className="size-5" />
						</div>
						<div className="min-w-0">
							<div className="truncate text-xs font-medium text-muted-foreground">
								{stat.label}
							</div>
							<div className="mt-1 text-2xl font-semibold tabular-nums">
								{loading && stat.value == null
									? "..."
									: formatCount(stat.value)}
							</div>
						</div>
					</div>
				</AdminSurface>
			))}
		</div>
	);
}

function RecentActivityPanel({
	items,
	loading,
}: {
	items: AuditLogEntry[];
	loading: boolean;
}) {
	const { t } = useTranslation();

	return (
		<AdminSurface className="min-w-0 overflow-hidden p-5">
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0">
					<h2 className="truncate text-base font-semibold">
						{t("admin.overview.activityTitle")}
					</h2>
					<p className="mt-1 line-clamp-2 text-sm text-muted-foreground">
						{t("admin.overview.activityDescription")}
					</p>
				</div>
				<Link
					to={adminPaths.audit}
					className="mt-0.5 shrink-0 text-sm font-medium text-primary hover:underline"
				>
					{t("admin.overview.viewAll")}
				</Link>
			</div>
			{items.length > 0 ? (
				<div className="mt-4 min-w-0 divide-y divide-border/60 dark:divide-white/10">
					{items.map((item) => {
						const detail = formatAuditDetail(t, item);
						return (
							<div
								key={item.id}
								className="min-w-0 overflow-hidden py-3 first:pt-0 last:pb-0"
							>
								<div className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)] gap-3">
									<StatusIndicator className="mt-1.5" />
									<div className="min-w-0 flex-1">
										<div className="min-w-0 truncate text-sm font-semibold">
											{formatAuditSummary(t, item)}
										</div>
										<div className="mt-1 min-w-0 truncate text-xs text-muted-foreground">
											{formatAuditTarget(t, item)}
										</div>
										{detail ? (
											<div className="mt-1 min-w-0 truncate text-xs text-muted-foreground">
												{detail}
											</div>
										) : null}
										<DateTimeText
											value={item.created_at}
											className="mt-1 block min-w-0 truncate text-xs text-muted-foreground"
										/>
									</div>
								</div>
							</div>
						);
					})}
				</div>
			) : (
				<div className="mt-4 rounded-lg border border-dashed border-border/70 p-4 text-sm text-muted-foreground">
					{loading
						? t("admin.overview.loadingActivity")
						: t("admin.overview.emptyActivity")}
				</div>
			)}
		</AdminSurface>
	);
}

function ServiceStatusPanel({
	loading,
	services,
}: {
	loading: boolean;
	services: AdminOverviewServiceStatus[];
}) {
	const { t } = useTranslation();

	return (
		<AdminSurface className="p-5">
			<h2 className="text-base font-semibold">
				{t("admin.overview.serviceTitle")}
			</h2>
			<p className="mt-1 text-sm text-muted-foreground">
				{t("admin.overview.serviceDescription")}
			</p>
			<div className="mt-4 divide-y divide-border/60 dark:divide-white/10">
				{services.length > 0 ? (
					services.map((service) => (
						<div
							key={service.key}
							className="grid grid-cols-[auto_minmax(0,1fr)] gap-3 py-3 first:pt-0 last:pb-0"
						>
							<StatusIndicator
								className="mt-1"
								glow
								size="md"
								tone={service.status === "warning" ? "warning" : "success"}
							/>
							<div className="min-w-0">
								<div className="flex items-start justify-between gap-3">
									<div className="truncate text-sm font-semibold">
										{t(`admin.overview.service.${service.key}`)}
									</div>
									<div className="shrink-0 text-xs text-muted-foreground">
										{t(`admin.overview.serviceStatus.${service.status}`)}
									</div>
								</div>
								{service.metric || service.detail ? (
									<div className="mt-1 truncate text-xs text-muted-foreground">
										{[service.metric, service.detail]
											.filter(Boolean)
											.join(" · ")}
									</div>
								) : null}
							</div>
						</div>
					))
				) : (
					<div className="py-3 text-sm text-muted-foreground">
						{loading
							? t("admin.overview.loadingServices")
							: t("admin.overview.emptyServices")}
					</div>
				)}
			</div>
		</AdminSurface>
	);
}

function QuickActionsPanel() {
	const { t } = useTranslation();

	return (
		<AdminSurface className="p-5">
			<h2 className="text-base font-semibold">
				{t("admin.overview.quickTitle")}
			</h2>
			<div className="mt-4 grid gap-3">
				{quickActions.map((item) => (
					<Link
						key={item.to}
						to={item.to}
						className="group grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 rounded-lg border border-border/60 bg-background/55 p-3 transition-[border-color,background-color] hover:border-primary/35 hover:bg-accent/45 dark:border-white/10 dark:bg-background/28 dark:hover:border-primary/35 dark:hover:bg-accent/20"
					>
						<Icon name={item.icon} className="size-5 text-primary" />
						<div className="min-w-0">
							<div className="truncate text-sm font-semibold">
								{t(item.titleKey)}
							</div>
							<div className="mt-0.5 truncate text-xs text-muted-foreground">
								{t(item.descriptionKey)}
							</div>
						</div>
						<Icon
							name="ArrowRight"
							className="size-4 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:text-primary"
						/>
					</Link>
				))}
			</div>
		</AdminSurface>
	);
}

function SystemInfoPanel({
	loading,
	overview,
}: {
	loading: boolean;
	overview: AdminOverview | null;
}) {
	const { i18n, t } = useTranslation();
	const rows = [
		{
			label: t("admin.overview.system.version"),
			value: formatVersion(overview?.system_info.version),
		},
		{
			label: t("admin.overview.system.uptime"),
			value: formatDurationSeconds(
				t,
				overview?.system_info.uptime_seconds,
				loading ? "..." : t("admin.overview.system.unknown"),
				i18n.language,
			),
		},
		{
			label: t("admin.overview.system.tasks"),
			value: overview
				? `${overview.summary.processing_task_count} / ${overview.summary.pending_task_count}`
				: loading
					? "..."
					: "0 / 0",
		},
	];

	return (
		<AdminSurface className="p-5">
			<h2 className="text-base font-semibold">
				{t("admin.overview.systemTitle")}
			</h2>
			<div className="mt-4 space-y-3">
				{rows.map((row) => (
					<div
						key={row.label}
						className="flex items-center justify-between gap-4 text-sm"
					>
						<div className="text-muted-foreground">{row.label}</div>
						<div className="min-w-0 truncate text-right font-medium">
							{row.value}
						</div>
					</div>
				))}
			</div>
		</AdminSurface>
	);
}

function formatCount(value: number | null | undefined) {
	if (value == null) return "0";
	return new Intl.NumberFormat().format(value);
}

function formatVersion(version: string | null | undefined) {
	if (!version || version === "unknown") return "unknown";
	return version.startsWith("v") ? version : `v${version}`;
}

function humanizeCode(value: string) {
	const text = value.replaceAll("-", "_").split("_").filter(Boolean).join(" ");
	return text ? text.charAt(0).toUpperCase() + text.slice(1) : value;
}
