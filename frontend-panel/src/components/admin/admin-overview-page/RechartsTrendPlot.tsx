import {
	CartesianGrid,
	Line,
	LineChart,
	ResponsiveContainer,
	Tooltip,
	XAxis,
	YAxis,
} from "recharts";
import type { TrendPoint, TrendSeries } from "./trendChartTypes";

const COUNT_FORMATTER = new Intl.NumberFormat();
const RESPONSIVE_CONTAINER_RESIZE_DEBOUNCE_MS = 120;

interface RechartsTrendPlotProps {
	series: TrendSeries[];
	trendData: TrendPoint[];
}

export default function RechartsTrendPlot({
	series,
	trendData,
}: RechartsTrendPlotProps) {
	return (
		<ResponsiveContainer
			width="100%"
			height="100%"
			debounce={RESPONSIVE_CONTAINER_RESIZE_DEBOUNCE_MS}
		>
			<LineChart
				data={trendData}
				margin={{ top: 8, right: 8, left: -18, bottom: 0 }}
			>
				<CartesianGrid
					vertical={false}
					stroke="var(--border)"
					strokeDasharray="4 6"
				/>
				<XAxis
					dataKey="label"
					axisLine={false}
					tickLine={false}
					tickMargin={12}
					interval={0}
					minTickGap={0}
					padding={{ left: 12, right: 12 }}
					tick={{ fill: "var(--muted-foreground)", fontSize: 12 }}
				/>
				<YAxis
					allowDecimals={false}
					axisLine={false}
					tickFormatter={formatCompactCount}
					tickLine={false}
					tickMargin={12}
					width={38}
					tick={{ fill: "var(--muted-foreground)", fontSize: 12 }}
				/>
				<Tooltip
					cursor={{ stroke: "var(--border)", strokeDasharray: "4 6" }}
					content={(props) => <TrendTooltipCard {...props} series={series} />}
				/>
				{series.map((seriesItem) => (
					<Line
						key={seriesItem.key}
						type="monotone"
						dataKey={seriesItem.key}
						name={seriesItem.label}
						stroke={seriesItem.stroke}
						strokeWidth={seriesItem.strokeWidth}
						dot={false}
						activeDot={{
							r: 4,
							fill: "var(--background)",
							stroke: seriesItem.stroke,
							strokeWidth: 2,
						}}
						isAnimationActive={false}
					/>
				))}
			</LineChart>
		</ResponsiveContainer>
	);
}

interface TrendTooltipCardProps {
	active?: boolean;
	payload?: ReadonlyArray<{
		dataKey?: unknown;
		payload?: unknown;
		value?: unknown;
	}>;
	series: TrendSeries[];
}

function TrendTooltipCard({ active, payload, series }: TrendTooltipCardProps) {
	if (!active || !payload?.length) return null;

	const point = payload.map((entry) => entry.payload).find(isTrendPoint);
	if (!point) return null;

	return (
		<div className="rounded-lg border border-border/70 bg-popover/95 px-3 py-2 text-popover-foreground shadow-md backdrop-blur">
			<div className="text-xs font-semibold text-muted-foreground">
				{point.date}
			</div>
			<div className="mt-2 space-y-1.5">
				{series.map((seriesItem) => (
					<div
						key={seriesItem.key}
						className="flex items-center justify-between gap-5 text-xs"
					>
						<span className="flex items-center gap-2 text-muted-foreground">
							<span
								className="inline-flex size-2 rounded-full"
								style={{ backgroundColor: seriesItem.stroke }}
							/>
							{seriesItem.label}
						</span>
						<span className="font-semibold tabular-nums">
							{COUNT_FORMATTER.format(point[seriesItem.key])}
						</span>
					</div>
				))}
			</div>
		</div>
	);
}

function isTrendPoint(value: unknown): value is TrendPoint {
	if (!value || typeof value !== "object") return false;
	const point = value as Partial<TrendPoint>;
	return (
		typeof point.date === "string" &&
		typeof point.label === "string" &&
		typeof point.active_users === "number" &&
		typeof point.active_players === "number" &&
		typeof point.new_textures === "number" &&
		typeof point.yggdrasil_api_calls === "number"
	);
}

function formatCompactCount(value: number) {
	return new Intl.NumberFormat(undefined, {
		compactDisplay: "short",
		notation: "compact",
	}).format(value);
}
