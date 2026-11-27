import type { TFunction } from "i18next";

export { formatDateTime } from "@/lib/dateTime";

import {
	externalAuthKindIconPath,
	normalizeExternalAuthIconUrl,
} from "@/lib/externalAuthProviders";
import { emptyToNull, emptyToUndefined } from "@/lib/form";
import { cn } from "@/lib/utils";
import type {
	AdminExternalAuthProviderInfo,
	CreateExternalAuthProviderRequest,
	ExternalAuthKind,
	ExternalAuthProviderKindInfo,
	ExternalAuthProviderOptions,
	ExternalAuthProviderTestParamsRequest,
	ExternalAuthProviderTestResult,
	UpdateExternalAuthProviderRequest,
} from "@/types/api";

export const EXTERNAL_AUTH_PAGE_SIZE_OPTIONS = [10, 20, 50] as const;
export const DEFAULT_EXTERNAL_AUTH_PAGE_SIZE = 20 as const;
export const DEFAULT_SCOPES = "openid email profile";
const REDACTED_SECRET = "***REDACTED***";
export const MICROSOFT_DEFAULT_TENANT = "common";
export const MICROSOFT_CUSTOM_TENANT_MODE = "custom";
export const MICROSOFT_TENANT_PRESETS = [
	"consumers",
	"organizations",
	MICROSOFT_DEFAULT_TENANT,
] as const;

export type MicrosoftTenantMode =
	| (typeof MICROSOFT_TENANT_PRESETS)[number]
	| typeof MICROSOFT_CUSTOM_TENANT_MODE;

export const STANDARD_CLAIMS = {
	avatarUrlClaim: "picture",
	displayNameClaim: "name",
	emailClaim: "email",
	emailVerifiedClaim: "email_verified",
	groupsClaim: "groups",
	subjectClaim: "sub",
	usernameClaim: "preferred_username",
} as const;

export interface ExternalAuthProviderFormData {
	allowedDomains: string;
	authorizationUrl: string;
	autoLinkVerifiedEmailEnabled: boolean;
	autoProvisionEnabled: boolean;
	avatarUrlClaim: string;
	clientId: string;
	clientSecret: string;
	displayName: string;
	displayNameClaim: string;
	emailClaim: string;
	emailVerifiedClaim: string;
	enabled: boolean;
	groupsClaim: string;
	iconUrl: string;
	issuerUrl: string;
	key: string;
	microsoftTenant: string;
	microsoftTenantMode: MicrosoftTenantMode;
	providerKind: ExternalAuthKind;
	requireEmailVerified: boolean;
	scopes: string;
	subjectClaim: string;
	tokenUrl: string;
	userinfoUrl: string;
	usernameClaim: string;
}

export type ExternalAuthProviderFieldChange = <
	K extends keyof ExternalAuthProviderFormData,
>(
	key: K,
	value: ExternalAuthProviderFormData[K],
) => void;

export interface ExternalAuthCreateStep {
	description: string;
	title: string;
}

export const emptyExternalAuthForm: ExternalAuthProviderFormData = {
	allowedDomains: "",
	authorizationUrl: "",
	autoLinkVerifiedEmailEnabled: false,
	autoProvisionEnabled: false,
	avatarUrlClaim: "",
	clientId: "",
	clientSecret: "",
	displayName: "",
	displayNameClaim: "",
	emailClaim: "",
	emailVerifiedClaim: "",
	enabled: true,
	groupsClaim: "",
	iconUrl: "",
	issuerUrl: "",
	key: "",
	microsoftTenant: MICROSOFT_DEFAULT_TENANT,
	microsoftTenantMode: MICROSOFT_DEFAULT_TENANT,
	providerKind: "oidc",
	requireEmailVerified: true,
	scopes: DEFAULT_SCOPES,
	subjectClaim: "",
	tokenUrl: "",
	userinfoUrl: "",
	usernameClaim: "",
};

export function kindDisplayName(
	t: TFunction,
	kind: ExternalAuthKind,
	providerKinds: ExternalAuthProviderKindInfo[],
) {
	const fallback =
		providerKinds.find((item) => item.kind === kind)?.display_name ??
		kindFallbackLabel(kind);
	return t(`admin.externalAuth.kind.${kind}`, { defaultValue: fallback });
}

export function kindDescription(
	t: TFunction,
	kind: ExternalAuthProviderKindInfo,
) {
	return t(`admin.externalAuth.kindDesc.${kind.kind}`, {
		defaultValue: kind.description,
	});
}

export function defaultScopesForKind(
	kind?: ExternalAuthProviderKindInfo | null,
) {
	return kind?.default_scopes.trim() || DEFAULT_SCOPES;
}

function kindFallbackLabel(kind: ExternalAuthKind) {
	switch (kind) {
		case "generic_oauth2":
			return "Generic OAuth2";
		case "github":
			return "GitHub";
		case "google":
			return "Google";
		case "microsoft":
			return "Microsoft";
		case "qq":
			return "QQ";
		case "oidc":
			return "OpenID Connect";
	}
}

function providerKindValue(
	kind: ExternalAuthKind | ExternalAuthProviderKindInfo | null | undefined,
) {
	return typeof kind === "string" ? kind : kind?.kind;
}

export function isSpecializedProviderKind(
	kind: ExternalAuthKind | ExternalAuthProviderKindInfo | null | undefined,
) {
	const value = providerKindValue(kind);
	return (
		value === "github" ||
		value === "google" ||
		value === "microsoft" ||
		value === "qq"
	);
}

export function providerUsesFixedConnection(
	kind: ExternalAuthKind | ExternalAuthProviderKindInfo | null | undefined,
	descriptor?: ExternalAuthProviderKindInfo | null,
) {
	if (descriptor && descriptor.kind === providerKindValue(kind)) {
		return (
			!descriptor.issuer_url_required &&
			!descriptor.manual_endpoint_configuration_supported
		);
	}
	return isSpecializedProviderKind(kind);
}

export function shouldShowIssuerUrl(
	kind: ExternalAuthProviderKindInfo | null | undefined,
) {
	return Boolean(
		kind?.issuer_url_required && !providerUsesFixedConnection(kind),
	);
}

export function shouldShowManualEndpoints(
	kind: ExternalAuthProviderKindInfo | null | undefined,
) {
	return Boolean(
		kind?.manual_endpoint_configuration_supported &&
			!providerUsesFixedConnection(kind),
	);
}

export function sortExternalAuthProviderKinds(
	kinds: ExternalAuthProviderKindInfo[],
) {
	const order: Record<ExternalAuthKind, number> = {
		oidc: 0,
		generic_oauth2: 1,
		github: 2,
		google: 3,
		microsoft: 4,
		qq: 5,
	};
	return kinds.toSorted((left, right) => order[left.kind] - order[right.kind]);
}

export function formFromProvider(
	provider: AdminExternalAuthProviderInfo,
): ExternalAuthProviderFormData {
	const microsoftTenant =
		provider.provider_kind === "microsoft"
			? normalizeMicrosoftTenantValue(
					provider.options.microsoft?.tenant ||
						microsoftTenantFromIssuerUrl(provider.issuer_url) ||
						MICROSOFT_DEFAULT_TENANT,
				) || MICROSOFT_DEFAULT_TENANT
			: MICROSOFT_DEFAULT_TENANT;
	return {
		allowedDomains: provider.allowed_domains.join(", "),
		authorizationUrl: provider.authorization_url ?? "",
		autoLinkVerifiedEmailEnabled: provider.auto_link_verified_email_enabled,
		autoProvisionEnabled: provider.auto_provision_enabled,
		avatarUrlClaim: provider.avatar_url_claim ?? "",
		clientId: provider.client_id,
		clientSecret: provider.client_secret_configured
			? REDACTED_SECRET
			: (provider.client_secret ?? ""),
		displayName: provider.display_name,
		displayNameClaim: provider.display_name_claim ?? "",
		emailClaim: provider.email_claim ?? "",
		emailVerifiedClaim: provider.email_verified_claim ?? "",
		enabled: provider.enabled,
		groupsClaim: provider.groups_claim ?? "",
		iconUrl: provider.icon_url ?? "",
		issuerUrl: provider.issuer_url ?? "",
		key: provider.key,
		microsoftTenant,
		microsoftTenantMode: microsoftTenantModeForValue(microsoftTenant),
		providerKind: provider.provider_kind,
		requireEmailVerified: provider.require_email_verified,
		scopes: provider.scopes || DEFAULT_SCOPES,
		subjectClaim: provider.subject_claim ?? "",
		tokenUrl: provider.token_url ?? "",
		userinfoUrl: provider.userinfo_url ?? "",
		usernameClaim: provider.username_claim ?? "",
	};
}

export function createPayload(
	form: ExternalAuthProviderFormData,
	kind?: ExternalAuthProviderKindInfo | null,
): CreateExternalAuthProviderRequest {
	const allowedDomains = parseAllowedDomains(form.allowedDomains);
	return {
		allowed_domains: allowedDomains.length > 0 ? allowedDomains : null,
		authorization_url: createConnectionValue(form, form.authorizationUrl, kind),
		auto_link_verified_email_enabled: form.autoLinkVerifiedEmailEnabled,
		auto_provision_enabled: form.autoProvisionEnabled,
		avatar_url_claim: emptyToNull(form.avatarUrlClaim),
		client_id: form.clientId.trim(),
		client_secret: emptyToNull(form.clientSecret),
		display_name: form.displayName.trim(),
		display_name_claim: emptyToNull(form.displayNameClaim),
		email_claim: emptyToNull(form.emailClaim),
		email_verified_claim: emptyToNull(form.emailVerifiedClaim),
		enabled: form.enabled,
		groups_claim: emptyToNull(form.groupsClaim),
		icon_url: emptyToNull(form.iconUrl),
		issuer_url: createConnectionValue(form, form.issuerUrl, kind),
		options: optionsPayload(form),
		provider_kind: form.providerKind,
		require_email_verified: form.requireEmailVerified,
		scopes: createScopesValue(form, kind),
		subject_claim: emptyToNull(form.subjectClaim),
		token_url: createConnectionValue(form, form.tokenUrl, kind),
		userinfo_url: createConnectionValue(form, form.userinfoUrl, kind),
		username_claim: emptyToNull(form.usernameClaim),
	};
}

export function updatePayload(
	form: ExternalAuthProviderFormData,
	kind?: ExternalAuthProviderKindInfo | null,
): UpdateExternalAuthProviderRequest {
	const allowedDomains = parseAllowedDomains(form.allowedDomains);
	return {
		allowed_domains: allowedDomains.length > 0 ? allowedDomains : null,
		authorization_url: updateConnectionValue(form, form.authorizationUrl, kind),
		auto_link_verified_email_enabled: form.autoLinkVerifiedEmailEnabled,
		auto_provision_enabled: form.autoProvisionEnabled,
		avatar_url_claim: emptyToNull(form.avatarUrlClaim),
		client_id: emptyToUndefined(form.clientId),
		...(isRedactedSecret(form.clientSecret)
			? {}
			: { client_secret: emptyToUndefined(form.clientSecret) }),
		display_name: emptyToUndefined(form.displayName),
		display_name_claim: emptyToNull(form.displayNameClaim),
		email_claim: emptyToNull(form.emailClaim),
		email_verified_claim: emptyToNull(form.emailVerifiedClaim),
		enabled: form.enabled,
		groups_claim: emptyToNull(form.groupsClaim),
		icon_url: emptyToUndefined(form.iconUrl),
		issuer_url: updateConnectionValue(form, form.issuerUrl, kind),
		options: optionsPayload(form),
		require_email_verified: form.requireEmailVerified,
		scopes: updateScopesValue(form, kind),
		subject_claim: emptyToNull(form.subjectClaim),
		token_url: updateConnectionValue(form, form.tokenUrl, kind),
		userinfo_url: updateConnectionValue(form, form.userinfoUrl, kind),
		username_claim: emptyToNull(form.usernameClaim),
	};
}

export function testParamsPayload(
	form: ExternalAuthProviderFormData,
	kind?: ExternalAuthProviderKindInfo | null,
): ExternalAuthProviderTestParamsRequest {
	return {
		authorization_url: createConnectionValue(form, form.authorizationUrl, kind),
		client_id: form.clientId.trim(),
		client_secret: isRedactedSecret(form.clientSecret)
			? null
			: emptyToNull(form.clientSecret),
		issuer_url: createConnectionValue(form, form.issuerUrl, kind),
		options: optionsPayload(form),
		provider_kind: form.providerKind,
		scopes: createScopesValue(form, kind),
		token_url: createConnectionValue(form, form.tokenUrl, kind),
		userinfo_url: createConnectionValue(form, form.userinfoUrl, kind),
	};
}

function createScopesValue(
	form: ExternalAuthProviderFormData,
	kind?: ExternalAuthProviderKindInfo | null,
) {
	return form.scopes.trim() || defaultScopesForKind(kind);
}

function updateScopesValue(
	form: ExternalAuthProviderFormData,
	kind?: ExternalAuthProviderKindInfo | null,
) {
	return form.scopes.trim() || defaultScopesForKind(kind);
}

function createConnectionValue(
	form: ExternalAuthProviderFormData,
	value: string,
	kind?: ExternalAuthProviderKindInfo | null,
) {
	if (providerUsesFixedConnection(form.providerKind, kind)) {
		return null;
	}
	return emptyToNull(value);
}

function updateConnectionValue(
	form: ExternalAuthProviderFormData,
	value: string,
	kind?: ExternalAuthProviderKindInfo | null,
) {
	if (providerUsesFixedConnection(form.providerKind, kind)) {
		return undefined;
	}
	return emptyToUndefined(value);
}

export function parseAllowedDomains(value: string) {
	const domains: string[] = [];
	const seen = new Set<string>();
	for (const item of value.split(/[,\n]/)) {
		const domain = item.trim().replace(/^@+/, "").toLowerCase();
		if (!domain || seen.has(domain)) continue;
		seen.add(domain);
		domains.push(domain);
	}
	return domains;
}

function optionsPayload(
	form: ExternalAuthProviderFormData,
): ExternalAuthProviderOptions {
	if (form.providerKind !== "microsoft") return {};
	return {
		microsoft: {
			tenant: microsoftTenantValue(form) || MICROSOFT_DEFAULT_TENANT,
		},
	};
}

function microsoftTenantValue(form: ExternalAuthProviderFormData) {
	return normalizeMicrosoftTenantValue(
		form.microsoftTenantMode === MICROSOFT_CUSTOM_TENANT_MODE
			? form.microsoftTenant
			: form.microsoftTenantMode,
	);
}

export function microsoftTenantModeForValue(
	value: string,
): MicrosoftTenantMode {
	const normalized = normalizeMicrosoftTenantValue(value);
	return MICROSOFT_TENANT_PRESETS.includes(
		normalized as (typeof MICROSOFT_TENANT_PRESETS)[number],
	)
		? (normalized as (typeof MICROSOFT_TENANT_PRESETS)[number])
		: MICROSOFT_CUSTOM_TENANT_MODE;
}

export function microsoftTenantFromIssuerUrl(value: string | null | undefined) {
	const trimmed = value?.trim();
	if (!trimmed) return "";
	try {
		const parsed = new URL(trimmed);
		if (parsed.hostname !== "login.microsoftonline.com") return "";
		const segments = parsed.pathname.split("/").filter(Boolean);
		return segments.length === 2 && segments[1]?.toLowerCase() === "v2.0"
			? normalizeMicrosoftTenantValue(segments[0])
			: "";
	} catch {
		return "";
	}
}

function normalizeMicrosoftTenantValue(value: string) {
	return value.trim().toLowerCase();
}

function isRedactedSecret(value: string) {
	return value.trim() === REDACTED_SECRET;
}

export function callbackUrl(provider: AdminExternalAuthProviderInfo) {
	const origin = typeof window === "undefined" ? "" : window.location.origin;
	return provider.key
		? `${origin}/api/v1/auth/external-auth/${encodeURIComponent(provider.provider_kind)}/${encodeURIComponent(provider.key)}/callback`
		: "";
}

export function requiredFieldsMissing(
	form: ExternalAuthProviderFormData,
	kind: ExternalAuthProviderKindInfo | null,
) {
	if (!form.displayName.trim() || !form.clientId.trim()) return true;
	if (
		form.providerKind === "microsoft" &&
		form.microsoftTenantMode === MICROSOFT_CUSTOM_TENANT_MODE &&
		!form.microsoftTenant.trim()
	) {
		return true;
	}
	if (kind?.issuer_url_required && !form.issuerUrl.trim()) return true;
	if (kind?.authorization_url_required && !form.authorizationUrl.trim()) {
		return true;
	}
	if (kind?.token_url_required && !form.tokenUrl.trim()) return true;
	if (kind?.userinfo_url_required && !form.userinfoUrl.trim()) return true;
	return false;
}

export function connectionRequirementsMissing(
	form: ExternalAuthProviderFormData,
	kind: ExternalAuthProviderKindInfo | null,
) {
	if (!form.clientId.trim()) return true;
	if (
		form.providerKind === "microsoft" &&
		form.microsoftTenantMode === MICROSOFT_CUSTOM_TENANT_MODE &&
		!form.microsoftTenant.trim()
	) {
		return true;
	}
	if (kind?.issuer_url_required && !form.issuerUrl.trim()) return true;
	if (kind?.authorization_url_required && !form.authorizationUrl.trim()) {
		return true;
	}
	if (kind?.token_url_required && !form.tokenUrl.trim()) return true;
	if (kind?.userinfo_url_required && !form.userinfoUrl.trim()) return true;
	return false;
}

export function primaryEndpoint(provider: AdminExternalAuthProviderInfo) {
	if (provider.issuer_url) {
		return provider.issuer_url;
	}
	return (
		provider.authorization_url ??
		provider.userinfo_url ??
		provider.token_url ??
		""
	);
}

export function formatTestResult(
	t: TFunction,
	result: ExternalAuthProviderTestResult,
) {
	const failed = result.checks.filter((check) => !check.success).length;
	const total = result.checks.length;
	return failed === 0
		? t("admin.externalAuth.testSuccess", { count: total })
		: t("admin.externalAuth.testPartial", { failed, total });
}

export function ExternalAuthProviderIcon({
	className,
	iconUrl,
	kind,
}: {
	className?: string;
	iconUrl?: string | null;
	kind: ExternalAuthKind;
}) {
	const configuredIcon = normalizeExternalAuthIconUrl(iconUrl);
	const kindIcon = externalAuthKindIconPath(kind);
	const iconPath = configuredIcon || kindIcon;

	return (
		<img
			src={iconPath}
			alt=""
			aria-hidden="true"
			className={cn("object-contain", className)}
			onError={(event) => {
				if (
					configuredIcon &&
					event.currentTarget.dataset.fallbackTried !== "1"
				) {
					event.currentTarget.dataset.fallbackTried = "1";
					event.currentTarget.src = kindIcon;
				}
			}}
		/>
	);
}
