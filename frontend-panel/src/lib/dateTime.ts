import type { TFunction } from "i18next";

export function formatDateTime(value: string, locale?: string) {
	const date = new Date(value);
	if (!Number.isFinite(date.getTime())) return value;
	return new Intl.DateTimeFormat(locale, {
		dateStyle: "medium",
		timeStyle: "short",
	}).format(date);
}

export function formatDateTimeOrFallback(
	value: string | null | undefined,
	fallback: string,
	locale?: string,
) {
	if (!value || value === "unknown") return fallback;
	return formatDateTime(value, locale);
}

export function formatDurationSeconds(
	t: TFunction,
	value: number | null | undefined,
	fallback: string,
	locale?: string,
) {
	if (value == null || !Number.isFinite(value) || value < 0) return fallback;

	const totalSeconds = Math.floor(value);
	const units = [
		{ key: "day", seconds: 86_400 },
		{ key: "hour", seconds: 3_600 },
		{ key: "minute", seconds: 60 },
		{ key: "second", seconds: 1 },
	] as const;
	const parts: Array<{ key: (typeof units)[number]["key"]; value: number }> =
		[];
	let remaining = totalSeconds;

	for (const unit of units) {
		const amount = Math.floor(remaining / unit.seconds);
		if (amount > 0 || (unit.key === "second" && parts.length === 0)) {
			parts.push({ key: unit.key, value: amount });
		}
		remaining %= unit.seconds;
		if (parts.length >= 2) break;
	}

	return parts
		.map((part) =>
			t(`duration.${part.key}`, { count: part.value, lng: locale }),
		)
		.join(" ");
}
