import type { TFunction } from "i18next";
import type { SystemConfig, TemplateVariableGroup } from "@/types/api";
import { translateOrFallback } from "./settingsCategories";
import { humanizeKey } from "./settingsDraft";

export type MailTemplateGroupItem = {
	configs: SystemConfig[];
	groupKey: string;
	templateCode: string;
};

export function buildMailTemplateGroups(
	category: string,
	configs: SystemConfig[],
) {
	const groups = configs.reduce((map, config) => {
		const templateCode = getMailTemplateCode(config.key);
		const existing = map.get(templateCode);
		if (existing) {
			existing.push(config);
		} else {
			map.set(templateCode, [config]);
		}
		return map;
	}, new Map<string, SystemConfig[]>());

	return Array.from(groups, ([templateCode, items]) => ({
		configs: items.toSorted(
			(left, right) =>
				getMailTemplateFieldOrder(left.key) -
					getMailTemplateFieldOrder(right.key) ||
				left.key.localeCompare(right.key),
		),
		groupKey: `${category}:${templateCode}`,
		templateCode,
	})).toSorted(
		(left, right) =>
			getMailTemplateGroupOrderIndex(left.templateCode) -
				getMailTemplateGroupOrderIndex(right.templateCode) ||
			left.templateCode.localeCompare(right.templateCode),
	);
}

const mailTemplateOrder = [
	"register_activation",
	"contact_change_confirmation",
	"password_reset",
	"password_reset_notice",
	"contact_change_notice",
	"external_auth_email_verification",
	"login_email_code",
	"user_invitation",
];

function getMailTemplateGroupOrderIndex(templateCode: string) {
	const index = mailTemplateOrder.indexOf(templateCode);
	return index === -1 ? Number.MAX_SAFE_INTEGER : index;
}

function getMailTemplateFieldOrder(key: string) {
	if (key.endsWith("_subject")) return 0;
	if (key.endsWith("_html")) return 1;
	return 2;
}

function getMailTemplateCode(key: string) {
	return key.replace(/^mail_template_/, "").replace(/_(subject|html)$/, "");
}

export function formatMailTemplateGroupLabel(
	templateCode: string,
	t: TFunction<"translation", undefined>,
) {
	return translateOrFallback(
		t,
		`settings_mail_template_group_${templateCode}`,
		humanizeKey(templateCode),
	);
}

export function formatTemplateVariableGroupLabel(
	group: TemplateVariableGroup,
	t: TFunction<"translation", undefined>,
) {
	return translateOrFallback(
		t,
		group.label_i18n_key,
		formatMailTemplateGroupLabel(group.template_code, t),
	);
}
