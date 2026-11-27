import { describe, expect, it } from "vitest";
import { documentLanguageTag, i18next } from "@/i18n";
import { mergeLocaleFiles, resources } from "@/i18n/resources";

type LeafMap = Record<string, unknown>;

/**
 * i18next v4 plural forms resolve through Intl plural-category suffixes
 * (`_one`/`_other`, plus the legacy `_plural`). Chinese only ever uses
 * `_other`-style lookups, so en/zh key sets legitimately differ by suffix
 * only — the parity rule below must reject any other drift.
 */
const PLURAL_SUFFIX = /_(zero|one|two|few|many|other|plural)$/;

function flattenLeaves(node: unknown, prefix = ""): LeafMap {
	const leaves: LeafMap = {};
	if (typeof node !== "object" || node === null || Array.isArray(node)) {
		return leaves;
	}
	for (const [key, value] of Object.entries(node)) {
		const keyPath = prefix ? `${prefix}.${key}` : key;
		if (typeof value === "object" && value !== null && !Array.isArray(value)) {
			Object.assign(leaves, flattenLeaves(value, keyPath));
		} else {
			leaves[keyPath] = value;
		}
	}
	return leaves;
}

function stripPluralSuffix(key: string) {
	return key.replace(PLURAL_SUFFIX, "");
}

describe("i18n locale key parity", () => {
	const enLeaves = flattenLeaves(resources["en-US"].frontend);
	const zhLeaves = flattenLeaves(resources["zh-CN"].frontend);

	it("keeps zh and en key sets aligned modulo plural suffixes", () => {
		const enKeys = new Set(Object.keys(enLeaves));
		const zhKeys = new Set(Object.keys(zhLeaves));
		const enNormalized = new Set([...enKeys].map(stripPluralSuffix));
		const zhNormalized = new Set([...zhKeys].map(stripPluralSuffix));
		const missingInZh = [...enNormalized]
			.filter((key) => !zhNormalized.has(key))
			.sort();
		const missingInEn = [...zhNormalized]
			.filter((key) => !enNormalized.has(key))
			.sort();

		// Every plural family must exist on both sides, and every raw one-sided
		// key must belong to a family both sides have: en may spell the family
		// `_one`/`_other` while zh keeps the singular-shaped plain key, but a
		// one-sided key outside any shared family is real drift.
		const orphanOneSided = [...enKeys, ...zhKeys]
			.filter((key) => {
				const base = stripPluralSuffix(key);
				return !enNormalized.has(base) || !zhNormalized.has(base);
			})
			.sort();

		const problems: string[] = [];
		if (missingInZh.length > 0) {
			problems.push(
				`zh-CN is missing keys that en-US has:\n  ${missingInZh.join("\n  ")}`,
			);
		}
		if (missingInEn.length > 0) {
			problems.push(
				`en-US is missing keys that zh-CN has:\n  ${missingInEn.join("\n  ")}`,
			);
		}
		if (orphanOneSided.length > 0) {
			problems.push(
				`keys present in only one language without a plural family on both sides:\n  ${orphanOneSided.join("\n  ")}`,
			);
		}
		expect(problems).toEqual([]);
	});
});

describe("mergeLocaleFiles collision guard", () => {
	const part = (file: string, content: Record<string, unknown>) => ({
		content,
		file,
	});

	it("merges files that only share structural ancestors", () => {
		expect(
			mergeLocaleFiles([
				part("a.json", { admin: { tasks: { title: "Tasks" } } }),
				part("b.json", { admin: { audit: { title: "Audit" } } }),
			]),
		).toEqual({
			admin: { tasks: { title: "Tasks" }, audit: { title: "Audit" } },
		});
	});

	it("allows identical leaf duplicates", () => {
		expect(
			mergeLocaleFiles([
				part("a.json", { section: { label: "Same" } }),
				part("b.json", { section: { label: "Same" } }),
			]),
		).toEqual({ section: { label: "Same" } });
	});

	it("throws with key path and both file names on conflicting values", () => {
		expect(() =>
			mergeLocaleFiles([
				part("a.json", { section: { label: "First" } }),
				part("b.json", { section: { label: "Second" } }),
			]),
		).toThrowError(
			/Duplicate i18n key "section\.label" defined in both "a\.json" and "b\.json"/,
		);
	});

	it("throws when a leaf and a subtree collide on the same key", () => {
		expect(() =>
			mergeLocaleFiles([
				part("a.json", { section: { label: "text" } }),
				part("b.json", { section: { label: { nested: "text" } } }),
			]),
		).toThrowError(/Duplicate i18n key "section\.label"/);
	});
});

describe("i18n runtime wiring", () => {
	it("maps UI languages onto BCP 47 document tags", () => {
		expect(documentLanguageTag("zh-CN")).toBe("zh-CN");
		expect(documentLanguageTag("en-US")).toBe("en-US");
		expect(documentLanguageTag("de-DE")).toBe("de");
		expect(documentLanguageTag("zz")).toBe("zz");
	});

	it("mirrors the active language onto <html lang> and keeps it in sync", async () => {
		expect(document.documentElement.lang).toBe(
			documentLanguageTag(i18next.language),
		);

		await i18next.changeLanguage("zh-CN");
		expect(document.documentElement.lang).toBe("zh-CN");

		await i18next.changeLanguage("en-US");
		expect(document.documentElement.lang).toBe("en-US");
	});

	it("resolves the repaired cancel and duration keys in both languages", () => {
		for (const language of ["en-US", "zh-CN"] as const) {
			expect(i18next.t("common.cancel", { lng: language })).not.toBe(
				"common.cancel",
			);
			expect(i18next.t("duration.hour", { count: 3, lng: language })).toContain(
				"3",
			);
		}
		expect(i18next.t("common.cancel", { lng: "zh-CN" })).toBe("取消");
		expect(i18next.t("common.cancel", { lng: "en-US" })).toBe("Cancel");
	});

	it("renders repaired en plural forms instead of singular fallbacks", () => {
		expect(i18next.t("account.profilesValue", { count: 1, lng: "en-US" })).toBe(
			"1 profile",
		);
		expect(i18next.t("account.profilesValue", { count: 3, lng: "en-US" })).toBe(
			"3 profiles",
		);
		expect(i18next.t("settings_save_notice", { count: 2, lng: "en-US" })).toBe(
			"2 unsaved changes",
		);
		expect(i18next.t("duration.day", { count: 1, lng: "en-US" })).toBe("1 day");
		expect(i18next.t("duration.day", { count: 2, lng: "en-US" })).toBe(
			"2 days",
		);
		expect(i18next.t("duration.day", { count: 2, lng: "zh-CN" })).toBe("2 天");
	});
});
