import type { AdminOverviewTrendPoint } from "@/types/api";

export type TrendSeriesKey =
	| "active_users"
	| "active_players"
	| "new_textures"
	| "yggdrasil_api_calls";

export interface TrendSeries {
	badgeClass: string;
	key: TrendSeriesKey;
	label: string;
	stroke: string;
	strokeWidth: number;
}

export interface TrendPoint extends AdminOverviewTrendPoint {
	label: string;
	total_activity: number;
}
