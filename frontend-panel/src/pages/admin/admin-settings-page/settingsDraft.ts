import { settingsNumberDraftSchema } from "@/lib/validation";
import type { ConfigValue, SetConfigRequest, SystemConfig } from "@/types/api";

export type DraftValue = {
	array: string[];
	arrayRows: DraftArrayRow[];
	text: string;
};

export type DraftArrayRow = {
	id: string;
	value: string;
};

export type ValidationIssue = {
	key: string;
	message: string;
};

export function draftsFromConfigs(configs: SystemConfig[]) {
	return Object.fromEntries(
		configs.map((config) => [config.key, configToDraft(config)]),
	);
}

export function configToDraft(config: SystemConfig): DraftValue {
	if (config.is_sensitive) {
		return { text: "", array: [], arrayRows: [] };
	}
	if (Array.isArray(config.value)) {
		return {
			text: config.value.join("\n"),
			array: config.value,
			arrayRows: config.value.map(createDraftArrayRow),
		};
	}
	return { text: config.value ?? "", array: [], arrayRows: [] };
}

export function draftEqualsConfig(config: SystemConfig, draft: DraftValue) {
	if (config.is_sensitive && draft.text === "" && draft.array.length === 0) {
		return true;
	}
	const current = normalizeConfigValue(config.value_type, config.value);
	const next = normalizeConfigValue(
		config.value_type,
		draftToValue(config.value_type, draft),
	);
	return JSON.stringify(current) === JSON.stringify(next);
}

export function draftToValue(
	valueType: SystemConfig["value_type"],
	draft: DraftValue | undefined,
): ConfigValue {
	if (valueType === "string_array" || valueType === "string_enum_set") {
		return compactTrimmedStrings(draft?.array ?? []);
	}
	if (valueType === "boolean") {
		return draft?.text === "true" ? "true" : "false";
	}
	return draft?.text ?? "";
}

function normalizeConfigValue(
	valueType: SystemConfig["value_type"],
	value: ConfigValue,
) {
	if (valueType === "string_array" || valueType === "string_enum_set") {
		return Array.isArray(value)
			? compactTrimmedStrings(value)
			: compactTrimmedStrings(String(value).split("\n"));
	}
	if (valueType === "boolean") {
		return String(value) === "true" ? "true" : "false";
	}
	return String(value ?? "");
}

function compactTrimmedStrings(values: string[]) {
	return values.flatMap((item) => {
		const value = item.trim();
		return value ? [value] : [];
	});
}

export function createDraftArrayRow(value: string): DraftArrayRow {
	return {
		id:
			typeof crypto !== "undefined" && "randomUUID" in crypto
				? crypto.randomUUID()
				: `draft-array-row-${Date.now()}-${Math.random().toString(16).slice(2)}`,
		value,
	};
}

/**
 * Draft validation for pending config edits. The schema owns the judgment
 * (key-in-schema, mirroring inviteUserSchema); the page only wraps the
 * schema message with the config key so the save bar keeps reporting
 * which setting failed.
 */
export function validateDraft(
	config: SystemConfig,
	draft: DraftValue | undefined,
	translateMessage: (messageKey: string) => string,
): ValidationIssue | null {
	if (!draft || (config.is_sensitive && draft.text.trim() === "")) return null;
	if (config.value_type === "number") {
		const parsed = settingsNumberDraftSchema.safeParse(draft.text);
		if (!parsed.success) {
			return {
				key: config.key,
				message: `${humanizeKey(config.key)}: ${translateMessage(
					parsed.error.issues[0]?.message ?? "",
				)}`,
			};
		}
	}
	return null;
}

export function buildSetConfigRequest(
	config: SystemConfig,
	value: ConfigValue,
): SetConfigRequest {
	if (config.source === "custom") {
		return { value, visibility: config.visibility };
	}

	return { value };
}

export function buildCaptchaPreviewValues(
	configs: SystemConfig[],
	drafts: Record<string, DraftValue>,
): Record<string, ConfigValue> {
	return Object.fromEntries(
		configs.map((config) => [
			config.key,
			draftToValue(
				config.value_type,
				drafts[config.key] ?? configToDraft(config),
			),
		]),
	);
}

export function humanizeKey(key: string) {
	return key
		.split(/[._-]+/)
		.filter(Boolean)
		.map((part) => part[0]?.toUpperCase() + part.slice(1))
		.join(" ");
}
