import type { TFunction } from "i18next";
import type { IconName } from "@/components/ui/icon";
import type { NumberUnitOption } from "@/lib/numberUnit";
import type { SystemConfig } from "@/types/api";
import { humanizeKey } from "./settingsDraft";

export type CategoryMeta = {
	descriptionKey: string;
	icon: IconName;
	id: string;
	labelKey: string;
};

export const categoryOrder = [
	"site",
	"auth",
	"user",
	"network",
	"mail",
	"yggdrasil",
	"texture",
	"runtime",
	"audit",
] as const;

export const categoryMeta: Record<string, CategoryMeta> = {
	yggdrasil: {
		id: "yggdrasil",
		icon: "Key",
		labelKey: "settings_category_yggdrasil",
		descriptionKey: "settings_category_yggdrasil_desc",
	},
	texture: {
		id: "texture",
		icon: "Images",
		labelKey: "settings_category_texture",
		descriptionKey: "settings_category_texture_desc",
	},
	auth: {
		id: "auth",
		icon: "Shield",
		labelKey: "settings_category_auth",
		descriptionKey: "settings_category_auth_desc",
	},
	user: {
		id: "user",
		icon: "User",
		labelKey: "settings_category_user",
		descriptionKey: "settings_category_user_desc",
	},
	site: {
		id: "site",
		icon: "Gear",
		labelKey: "settings_category_site",
		descriptionKey: "settings_category_site_desc",
	},
	network: {
		id: "network",
		icon: "Globe",
		labelKey: "settings_category_network",
		descriptionKey: "settings_category_network_desc",
	},
	mail: {
		id: "mail",
		icon: "EnvelopeSimple",
		labelKey: "settings_category_mail",
		descriptionKey: "settings_category_mail_desc",
	},
	runtime: {
		id: "runtime",
		icon: "Gauge",
		labelKey: "settings_category_runtime",
		descriptionKey: "settings_category_runtime_desc",
	},
	audit: {
		id: "audit",
		icon: "Scroll",
		labelKey: "settings_category_audit",
		descriptionKey: "settings_category_audit_desc",
	},
};

export type TimeConfigBaseUnit = "seconds" | "hours" | "days";

export type TimeDisplayUnitValue =
	| "seconds"
	| "minutes"
	| "hours"
	| "days"
	| "weeks";

export type TimeDisplayUnit = NumberUnitOption<TimeDisplayUnitValue>;

export const timeDisplayUnits: Record<
	TimeConfigBaseUnit,
	readonly TimeDisplayUnit[]
> = {
	seconds: [
		{
			labelKey: "settings_time_unit_days",
			multiplier: 86_400,
			value: "days",
		},
		{
			labelKey: "settings_time_unit_hours",
			multiplier: 3_600,
			value: "hours",
		},
		{
			labelKey: "settings_time_unit_minutes",
			multiplier: 60,
			value: "minutes",
		},
		{
			labelKey: "settings_time_unit_seconds",
			multiplier: 1,
			value: "seconds",
		},
	],
	hours: [
		{ labelKey: "settings_time_unit_days", multiplier: 24, value: "days" },
		{ labelKey: "settings_time_unit_hours", multiplier: 1, value: "hours" },
	],
	days: [
		{ labelKey: "settings_time_unit_weeks", multiplier: 7, value: "weeks" },
		{ labelKey: "settings_time_unit_days", multiplier: 1, value: "days" },
	],
};

export function sortConfigs(configs: SystemConfig[]) {
	return configs.toSorted((left, right) => {
		const leftRoot = rootCategory(left.category);
		const rightRoot = rootCategory(right.category);
		const leftIndex = categoryOrder.indexOf(
			leftRoot as (typeof categoryOrder)[number],
		);
		const rightIndex = categoryOrder.indexOf(
			rightRoot as (typeof categoryOrder)[number],
		);
		return (
			(leftIndex === -1 ? Number.MAX_SAFE_INTEGER : leftIndex) -
				(rightIndex === -1 ? Number.MAX_SAFE_INTEGER : rightIndex) ||
			left.category.localeCompare(right.category) ||
			left.key.localeCompare(right.key)
		);
	});
}

export function rootCategory(category: string) {
	const [root] = category.split(".");
	return root || "other";
}

export function normalizeRouteCategory(category: string | undefined) {
	const normalized = category?.trim().toLowerCase();
	return normalized ? normalized : null;
}

export function isKnownCategory(category: string) {
	return categoryOrder.includes(category as (typeof categoryOrder)[number]);
}

export function getTimeConfigBaseUnit(
	config: SystemConfig,
): TimeConfigBaseUnit | null {
	if (config.value_type !== "number") return null;
	if (config.key.endsWith("_secs")) return "seconds";
	if (config.key.endsWith("_hours")) return "hours";
	if (config.key.endsWith("_days")) return "days";
	return null;
}

export function parseWholeNumber(value: string) {
	const trimmed = value.trim();
	if (!trimmed) return null;
	if (!/^-?\d+$/.test(trimmed)) return null;

	const parsed = Number(trimmed);
	return Number.isSafeInteger(parsed) ? parsed : null;
}

export function getAvailableDisplayUnits<T extends TimeDisplayUnit>(
	units: readonly T[],
	_value: string,
) {
	return units;
}

export function getPreferredDisplayUnit<T extends TimeDisplayUnit>(
	units: readonly T[],
	value: string,
) {
	if (!value.trim()) return units[units.length - 1];

	const parsed = parseWholeNumber(value);
	if (parsed === 0) return units[units.length - 1];
	if (parsed === null) return units[units.length - 1];

	return (
		units.find(
			(unit) => unit.multiplier === 1 || parsed % unit.multiplier === 0,
		) ?? units[units.length - 1]
	);
}

export function formatDisplayValue(value: string, unit: TimeDisplayUnit) {
	if (!value.trim()) return "";

	const parsed = parseWholeNumber(value);
	if (parsed === null) return value;

	return String(parsed / unit.multiplier);
}

export function translateOrFallback(
	t: TFunction<"translation", undefined>,
	key: string | undefined,
	fallback: string,
) {
	if (!key) return fallback;
	const translated = t(key);
	return translated === key ? fallback : translated;
}

export function formatSubcategoryLabel(
	root: string,
	category: string,
	t: TFunction<"translation", undefined>,
) {
	return translateOrFallback(
		t,
		`settings_subcategory_${category.replaceAll(".", "_")}`,
		category === root
			? t(categoryMeta[root]?.labelKey ?? "settings_category_other")
			: humanizeKey(category),
	);
}

export function formatSubcategoryDescription(
	root: string,
	category: string,
	t: TFunction<"translation", undefined>,
) {
	return translateOrFallback(
		t,
		`settings_subcategory_${category.replaceAll(".", "_")}_desc`,
		category === root
			? t(categoryMeta[root]?.descriptionKey ?? "settings_category_other_desc")
			: "",
	);
}

export function editorLanguage(config: SystemConfig) {
	if (config.key.endsWith("_html")) return "html";
	if (config.key.endsWith("_json")) return "json";
	if (config.key.includes("private_key") || config.key.includes("public_key")) {
		return "pem";
	}
	return "text";
}

export function formatError(error: unknown) {
	return error instanceof Error ? error.message : String(error);
}
