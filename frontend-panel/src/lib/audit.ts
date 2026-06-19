import type { TFunction } from "i18next";
import type {
	AuditAction,
	AuditEntityType,
	AuditLogEntry,
	AuditPresentationMessage,
} from "@/types/api";

export const AUDIT_ENTITY_TYPE_FILTER_VALUES = [
	"system",
	"system_config",
	"user",
	"invitation",
	"auth_session",
	"passkey",
	"external_auth_provider",
	"external_auth_identity",
	"api_token",
	"mail",
	"task",
	"minecraft_profile",
	"minecraft_texture",
	"yggdrasil_token",
	"yggdrasil_session",
] as const satisfies readonly AuditEntityType[];

export function isAuditEntityType(value: string): value is AuditEntityType {
	type MissingAuditEntityType = Exclude<
		AuditEntityType,
		(typeof AUDIT_ENTITY_TYPE_FILTER_VALUES)[number]
	>;
	const filterValuesCoverOpenApi: MissingAuditEntityType extends never
		? true
		: never = true;
	return (
		filterValuesCoverOpenApi &&
		AUDIT_ENTITY_TYPE_FILTER_VALUES.includes(value as AuditEntityType)
	);
}

function resolveAuditTranslation(t: TFunction, key: string, fallback?: string) {
	const translated = t(key, { defaultValue: key });
	return translated === key ? fallback : translated;
}

export function formatAuditAction(t: TFunction, action: AuditAction | string) {
	const value = String(action);
	return (
		resolveAuditTranslation(t, `admin.audit.action.${value}`) ??
		humanizeCode(value)
	);
}

export function formatAuditEntityType(
	t: TFunction,
	entityType: string | null | undefined,
) {
	if (!entityType) {
		return "---";
	}

	return (
		resolveAuditTranslation(t, `admin.audit.entity.${entityType}`) ??
		humanizeCode(entityType)
	);
}

type AuditPresentationEntry = Pick<AuditLogEntry, "action" | "presentation">;

function isRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function presentationParams(
	message: AuditPresentationMessage | undefined | null,
) {
	return isRecord(message?.params) ? message.params : {};
}

function stringParam(
	params: Record<string, unknown>,
	key: string,
): string | undefined {
	const value = params[key];
	if (typeof value === "string") {
		return value;
	}
	if (typeof value === "number" || typeof value === "boolean") {
		return String(value);
	}
	if (Array.isArray(value)) {
		const text = value.map((item) => String(item)).join(", ");
		return text || undefined;
	}
	return undefined;
}

function humanizeCode(value: string) {
	const words = value.replaceAll("-", "_").split("_").filter(Boolean);
	if (words.length === 0) {
		return value;
	}
	const text = words.join(" ");
	return text.charAt(0).toUpperCase() + text.slice(1);
}

function cleanUnresolvedInterpolation(value: string) {
	return value
		.replace(/\{\{\s*[\w.]+\s*\}\}\s*[,，]\s*/g, "")
		.replace(/\s*[,，]\s*\{\{\s*[\w.]+\s*\}\}/g, "")
		.replace(/\{\{\s*[\w.]+\s*\}\}/g, "")
		.replace(/\s+([,，])/g, "$1")
		.replace(/([,，])\s*([,，])/g, "$1")
		.trim();
}

function formatAuditPresentationMessage(
	t: TFunction,
	message: AuditPresentationMessage | undefined | null,
	prefer: "action" | "presentation" | "target",
	fallback?: string,
) {
	if (!message?.code) {
		return fallback;
	}

	const params = presentationParams(message);
	if (prefer === "action") {
		const actionLabel = resolveAuditTranslation(
			t,
			`admin.audit.action.${message.code}`,
		);
		if (actionLabel) {
			return actionLabel;
		}
	}

	const direct = resolveAuditTranslation(
		t,
		`admin.audit.presentation.${message.code}`,
	);
	if (direct) {
		const formatted = t(`admin.audit.presentation.${message.code}`, {
			defaultValue: direct,
			...params,
		});
		return cleanUnresolvedInterpolation(formatted);
	}

	const actionLabel = resolveAuditTranslation(
		t,
		`admin.audit.action.${message.code}`,
	);
	if (actionLabel) {
		return actionLabel;
	}

	const entityLabel = resolveAuditTranslation(
		t,
		`admin.audit.entity.${message.code}`,
	);
	if (entityLabel) {
		const name = stringParam(params, "name");
		const id = stringParam(params, "id");
		if (prefer === "target" && name) {
			return `${name} · ${entityLabel}`;
		}
		if (prefer === "target" && id) {
			return `${entityLabel} #${id}`;
		}
		return entityLabel;
	}

	return fallback;
}

export function formatAuditSummary(
	t: TFunction,
	entry: AuditPresentationEntry,
) {
	return (
		formatAuditPresentationMessage(t, entry.presentation?.summary, "action") ??
		formatAuditAction(t, entry.action)
	);
}

export function formatAuditTarget(
	t: TFunction,
	entry: Pick<
		AuditLogEntry,
		"entity_id" | "entity_name" | "entity_type" | "presentation"
	>,
) {
	const fallback = [
		entry.entity_name ?? null,
		formatAuditEntityType(t, entry.entity_type),
		entry.entity_id == null ? null : `#${entry.entity_id}`,
	]
		.filter((value): value is string => Boolean(value))
		.join(" · ");
	return (
		formatAuditPresentationMessage(
			t,
			entry.presentation?.target,
			"target",
			fallback,
		) ?? fallback
	);
}

export function formatAuditTargetType(
	t: TFunction,
	entry: Pick<AuditLogEntry, "entity_type" | "presentation">,
) {
	const targetCode = entry.presentation?.target?.code;
	if (targetCode) {
		const translatedTargetType = resolveAuditTranslation(
			t,
			`admin.audit.entity.${targetCode}`,
		);
		if (translatedTargetType) {
			return translatedTargetType;
		}
	}
	return formatAuditEntityType(t, entry.entity_type);
}

export function formatAuditDetail(t: TFunction, entry: AuditPresentationEntry) {
	return formatAuditPresentationMessage(
		t,
		entry.presentation?.detail,
		"presentation",
	);
}

type AuditActionTone = "danger" | "success" | "info" | "warning";

const AUDIT_ACTION_TONES = {
	admin_delete_config: "danger",
	admin_delete_external_auth_provider: "danger",
	admin_delete_user: "danger",
	admin_delete_yggdrasil_session_forward_server: "danger",
	admin_disable_user: "danger",
	admin_revoke_invitation: "danger",
	admin_revoke_user_sessions: "danger",
	config_delete: "danger",
	external_auth_provider_delete: "danger",
	mail_delivery_failed: "danger",
	minecraft_profile_delete: "danger",
	minecraft_texture_delete: "danger",
	server_shutdown: "danger",
	user_revoke_other_sessions: "danger",
	user_revoke_session: "danger",
	yggdrasil_invalidate_token: "danger",
	yggdrasil_signout: "danger",

	mail_send: "success",
	minecraft_profile_create: "success",
	minecraft_texture_upload: "success",
	system_setup: "success",
	user_register: "success",

	admin_create_external_auth_provider: "info",
	admin_create_invitation: "info",
	admin_create_user: "info",
	admin_create_yggdrasil_session_forward_server: "info",
	admin_update_external_auth_provider: "info",
	admin_update_user: "info",
	admin_update_yggdrasil_session_forward_server: "info",
	config_action_execute: "info",
	config_update: "info",
	external_auth_provider_create: "info",
	external_auth_provider_update: "info",
	minecraft_texture_bind: "info",
	task_retry: "info",
	user_change_password: "info",
	user_confirm_email_change: "info",
	user_confirm_password_reset: "info",
	user_confirm_registration: "info",
	user_request_email_change: "info",
	user_request_password_reset: "info",
	user_resend_email_change: "info",
	user_update_profile: "info",

	admin_test_external_auth_provider: "warning",
	user_external_auth_link: "warning",
	user_external_auth_login: "warning",
	user_external_auth_unlink: "warning",
	user_login: "warning",
	user_logout: "warning",
	user_refresh_token: "warning",
	yggdrasil_authenticate: "warning",
	yggdrasil_join_server: "warning",
	yggdrasil_refresh_token: "warning",
	yggdrasil_session_forward_check: "warning",
} as const satisfies Partial<Record<AuditAction, AuditActionTone>>;

const AUDIT_ACTION_TONE_CLASSES = {
	danger:
		"border-red-200 bg-red-50 text-red-700 dark:border-red-900 dark:bg-red-950/60 dark:text-red-300",
	info: "border-sky-200 bg-sky-50 text-sky-700 dark:border-sky-900 dark:bg-sky-950/60 dark:text-sky-300",
	success:
		"border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/60 dark:text-emerald-300",
	warning:
		"border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-900 dark:bg-amber-950/60 dark:text-amber-300",
} as const satisfies Record<AuditActionTone, string>;

export function getAuditActionBadgeClass(action: AuditAction | string) {
	const tone =
		typeof action === "string" &&
		Object.hasOwn(AUDIT_ACTION_TONES, action) &&
		AUDIT_ACTION_TONES[action as keyof typeof AUDIT_ACTION_TONES];
	if (tone) {
		return AUDIT_ACTION_TONE_CLASSES[tone];
	}
	return "border-border bg-muted/30 text-muted-foreground";
}
