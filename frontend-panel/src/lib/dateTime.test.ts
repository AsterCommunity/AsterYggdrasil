import type { TFunction } from "i18next";
import { describe, expect, it } from "vitest";
import { i18next } from "@/i18n";
import {
	formatDateTime,
	formatDateTimeOrFallback,
	formatDurationSeconds,
} from "@/lib/dateTime";

function tFor(language: string): TFunction {
	return ((key: string, options?: Record<string, unknown>) =>
		i18next.t(key, { ...options, lng: language })) as unknown as TFunction;
}

describe("dateTime helpers", () => {
	it("formats valid timestamps with the provided locale", () => {
		expect(formatDateTime("2026-06-15T08:30:00.000Z", "en-US")).toContain(
			"2026",
		);
	});

	it("returns invalid values unchanged", () => {
		expect(formatDateTime("not-a-date", "en-US")).toBe("not-a-date");
	});

	it("uses fallback for missing or unknown timestamps", () => {
		expect(formatDateTimeOrFallback(undefined, "Unknown", "en-US")).toBe(
			"Unknown",
		);
		expect(formatDateTimeOrFallback("unknown", "Unknown", "en-US")).toBe(
			"Unknown",
		);
	});

	it("formats uptime durations through pluralized i18n keys", () => {
		expect(formatDurationSeconds(tFor("en-US"), 3723, "Unknown")).toBe(
			"1 hour 2 minutes",
		);
		expect(formatDurationSeconds(tFor("zh-CN"), 3723, "未知")).toBe(
			"1 小时 2 分钟",
		);
	});

	it("switches singular and plural units per language rules", () => {
		expect(formatDurationSeconds(tFor("en-US"), 3600, "Unknown")).toBe(
			"1 hour",
		);
		expect(formatDurationSeconds(tFor("en-US"), 7200, "Unknown")).toBe(
			"2 hours",
		);
		expect(formatDurationSeconds(tFor("zh-CN"), 7200, "未知")).toBe("2 小时");
	});

	it("uses fallback for missing, invalid, or negative durations", () => {
		expect(formatDurationSeconds(tFor("en-US"), undefined, "Unknown")).toBe(
			"Unknown",
		);
		expect(formatDurationSeconds(tFor("en-US"), "nope", "Unknown")).toBe(
			"Unknown",
		);
		expect(formatDurationSeconds(tFor("en-US"), -5, "Unknown")).toBe("Unknown");
	});

	it("falls back to seconds for zero and sub-minute durations", () => {
		expect(formatDurationSeconds(tFor("en-US"), 0, "Unknown")).toBe(
			"0 seconds",
		);
		expect(formatDurationSeconds(tFor("zh-CN"), 42, "未知")).toBe("42 秒");
	});
});
