import { lazy, Suspense, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Badge } from "@/components/ui/badge";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import type { AdminOverviewTrendPoint } from "@/types/api";

import type { TrendPoint, TrendSeries } from "./trendChartTypes";

const RechartsTrendPlot = lazy(() => import("./RechartsTrendPlot"));

export function ActivityTrendChart({
	loading,
	trend,
}: {
	loading: boolean;
	trend: AdminOverviewTrendPoint[];
}) {
	const { t } = useTranslation();
	const series = useMemo<TrendSeries[]>(
		() => [
			{
				badgeClass:
					"border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
				key: "active_users",
				label: t("admin.overview.chart.users"),
				stroke: "#059669",
				strokeWidth: 2.5,
			},
			{
				badgeClass:
					"border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300",
				key: "active_players",
				label: t("admin.overview.chart.profiles"),
				stroke: "#0284c7",
				strokeWidth: 2.5,
			},
			{
				badgeClass:
					"border-violet-500/30 bg-violet-500/10 text-violet-700 dark:text-violet-300",
				key: "new_textures",
				label: t("admin.overview.chart.textures"),
				stroke: "#7c3aed",
				strokeWidth: 2.5,
			},
			{
				badgeClass:
					"border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
				key: "yggdrasil_api_calls",
				label: t("admin.overview.chart.yggdrasil"),
				stroke: "#d97706",
				strokeWidth: 2.5,
			},
		],
		[t],
	);
	const chartData = useMemo(
		() => createTrendData(trend, series),
		[trend, series],
	);
	const hasTrendData = chartData.length > 0;
	const hasActivity = chartData.some((point) => point.total_activity > 0);

	return (
		<AdminSurface className="p-5">
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0">
					<h2 className="text-base font-semibold">
						{t("admin.overview.chartTitle")}
					</h2>
					<p className="mt-1 text-sm text-muted-foreground">
						{t("admin.overview.chartDescription")}
					</p>
				</div>
				<Icon name="ChartBar" className="size-5 shrink-0 text-primary" />
			</div>

			{hasTrendData ? (
				<div className="mt-5 min-w-0 overflow-hidden rounded-xl border border-border/70 bg-linear-to-br from-emerald-500/5 via-background to-sky-500/5 p-4 dark:from-emerald-400/8 dark:to-sky-400/8">
					<div className="mb-3 flex flex-wrap items-center gap-2">
						{series.map((seriesItem) => (
							<Badge
								key={seriesItem.key}
								variant="outline"
								className={cn("gap-2 border", seriesItem.badgeClass)}
							>
								<span
									className="inline-flex size-2 rounded-full"
									style={{ backgroundColor: seriesItem.stroke }}
								/>
								{seriesItem.label}
							</Badge>
						))}
					</div>
					<div className="h-[280px] min-h-[280px] min-w-0">
						<Suspense
							fallback={
								<div className="grid h-full place-items-center text-sm text-muted-foreground">
									{t("admin.overview.chartLoading")}
								</div>
							}
						>
							<RechartsTrendPlot series={series} trendData={chartData} />
						</Suspense>
					</div>
					{!hasActivity ? (
						<p className="mt-3 text-xs text-muted-foreground">
							{t("admin.overview.chartNoActivity")}
						</p>
					) : null}
				</div>
			) : (
				<div className="mt-5 grid h-[280px] place-items-center rounded-xl border border-dashed border-border/70 bg-muted/20 text-sm text-muted-foreground">
					{loading
						? t("admin.overview.chartLoading")
						: t("admin.overview.chartEmpty")}
				</div>
			)}
		</AdminSurface>
	);
}

function createTrendData(
	trend: AdminOverviewTrendPoint[],
	series: TrendSeries[],
): TrendPoint[] {
	return trend
		.toSorted((left, right) => left.date.localeCompare(right.date))
		.map((point) => {
			const totalActivity = series.reduce(
				(sum, item) => sum + normalizeCount(point[item.key]),
				0,
			);

			return {
				...point,
				active_users: normalizeCount(point.active_users),
				active_players: normalizeCount(point.active_players),
				label: formatTrendDayLabel(point.date),
				new_textures: normalizeCount(point.new_textures),
				total_activity: totalActivity,
				yggdrasil_api_calls: normalizeCount(point.yggdrasil_api_calls),
			};
		});
}

function normalizeCount(value: number) {
	return Number.isFinite(value) && value > 0 ? value : 0;
}

function formatTrendDayLabel(value: string) {
	const [year, month, day] = value.split("-");
	return year && month && day ? `${Number(month)}/${Number(day)}` : value;
}
