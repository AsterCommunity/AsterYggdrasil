import type { TFunction } from "i18next";

/**
 * Mock `t` with interpolation support for component tests that should assert
 * rendered copy instead of bare i18n keys. Lookup order: exact key, then
 * `defaultValue`, then the key itself (mirrors i18next's fallback chain).
 */
export function createMockTWithInterpolation(
	translations: Record<string, string> = {},
) {
	return ((key: string, options?: Record<string, unknown>) => {
		const translated = translations[key];
		if (translated) {
			return translated.replace(/\{\{(\w+)\}\}/g, (match, param) => {
				const value = options?.[param];
				return value === undefined || value === null ? match : String(value);
			});
		}
		return typeof options?.defaultValue === "string"
			? options.defaultValue
			: key;
	}) as unknown as TFunction;
}
