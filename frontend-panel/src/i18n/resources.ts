import enAccount from "@/i18n/locales/en-US/account.json";
import enAdmin from "@/i18n/locales/en-US/admin.json";
import enAdminAudit from "@/i18n/locales/en-US/admin-audit.json";
import enAdminExternalAuth from "@/i18n/locales/en-US/admin-external-auth.json";
import enAdminTasks from "@/i18n/locales/en-US/admin-tasks.json";
import enAdminUsers from "@/i18n/locales/en-US/admin-users.json";
import enErrors from "@/i18n/locales/en-US/errors.json";
import enPublic from "@/i18n/locales/en-US/public.json";
import enPwa from "@/i18n/locales/en-US/pwa.json";
import enSettings from "@/i18n/locales/en-US/settings.json";
import enShell from "@/i18n/locales/en-US/shell.json";
import enTime from "@/i18n/locales/en-US/time.json";
import zhAccount from "@/i18n/locales/zh-CN/account.json";
import zhAdmin from "@/i18n/locales/zh-CN/admin.json";
import zhAdminAudit from "@/i18n/locales/zh-CN/admin-audit.json";
import zhAdminExternalAuth from "@/i18n/locales/zh-CN/admin-external-auth.json";
import zhAdminTasks from "@/i18n/locales/zh-CN/admin-tasks.json";
import zhAdminUsers from "@/i18n/locales/zh-CN/admin-users.json";
import zhErrors from "@/i18n/locales/zh-CN/errors.json";
import zhPublic from "@/i18n/locales/zh-CN/public.json";
import zhPwa from "@/i18n/locales/zh-CN/pwa.json";
import zhSettings from "@/i18n/locales/zh-CN/settings.json";
import zhShell from "@/i18n/locales/zh-CN/shell.json";
import zhTime from "@/i18n/locales/zh-CN/time.json";

type LocaleFileContent = Record<string, unknown>;

interface LocaleFilePart {
	content: LocaleFileContent;
	file: string;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}

function mergeFileInto(
	target: Record<string, unknown>,
	source: LocaleFileContent,
	part: LocaleFilePart,
	path: string[],
	leafOwners: Map<string, string>,
) {
	for (const [key, value] of Object.entries(source)) {
		const keyPath = [...path, key].join(".");
		const existing = target[key];
		if (existing === undefined) {
			if (isPlainObject(value)) {
				const child: Record<string, unknown> = {};
				target[key] = child;
				mergeFileInto(child, value, part, [...path, key], leafOwners);
			} else {
				target[key] = value;
				leafOwners.set(keyPath, part.file);
			}
			continue;
		}
		if (isPlainObject(existing) && isPlainObject(value)) {
			mergeFileInto(existing, value, part, [...path, key], leafOwners);
			continue;
		}
		if (existing === value) {
			continue;
		}
		throw new Error(
			`Duplicate i18n key "${keyPath}" defined in both "${leafOwners.get(keyPath) ?? "an earlier file"}" and "${part.file}" with different values`,
		);
	}
}

/**
 * Deep-merges locale files into the single `frontend` namespace and rejects
 * silent spread-order overrides: two files may share structural ancestors,
 * but a leaf key defined twice with different values throws at startup.
 */
export function mergeLocaleFiles(
	parts: LocaleFilePart[],
): Record<string, unknown> {
	const merged: Record<string, unknown> = {};
	const leafOwners = new Map<string, string>();
	for (const part of parts) {
		mergeFileInto(merged, part.content, part, [], leafOwners);
	}
	return merged;
}

const enLocaleFiles: LocaleFilePart[] = [
	{ file: "en-US/shell.json", content: enShell },
	{ file: "en-US/public.json", content: enPublic },
	{ file: "en-US/account.json", content: enAccount },
	{ file: "en-US/errors.json", content: enErrors },
	{ file: "en-US/pwa.json", content: enPwa },
	{ file: "en-US/admin.json", content: enAdmin },
	{ file: "en-US/admin-audit.json", content: enAdminAudit },
	{ file: "en-US/admin-external-auth.json", content: enAdminExternalAuth },
	{ file: "en-US/admin-tasks.json", content: enAdminTasks },
	{ file: "en-US/admin-users.json", content: enAdminUsers },
	{ file: "en-US/settings.json", content: enSettings },
	{ file: "en-US/time.json", content: enTime },
];

const zhLocaleFiles: LocaleFilePart[] = [
	{ file: "zh-CN/shell.json", content: zhShell },
	{ file: "zh-CN/public.json", content: zhPublic },
	{ file: "zh-CN/account.json", content: zhAccount },
	{ file: "zh-CN/errors.json", content: zhErrors },
	{ file: "zh-CN/pwa.json", content: zhPwa },
	{ file: "zh-CN/admin.json", content: zhAdmin },
	{ file: "zh-CN/admin-audit.json", content: zhAdminAudit },
	{ file: "zh-CN/admin-external-auth.json", content: zhAdminExternalAuth },
	{ file: "zh-CN/admin-tasks.json", content: zhAdminTasks },
	{ file: "zh-CN/admin-users.json", content: zhAdminUsers },
	{ file: "zh-CN/settings.json", content: zhSettings },
	{ file: "zh-CN/time.json", content: zhTime },
];

export const resources = {
	"en-US": {
		frontend: mergeLocaleFiles(enLocaleFiles),
	},
	"zh-CN": {
		frontend: mergeLocaleFiles(zhLocaleFiles),
	},
};
