import { describe, expect, it } from "vitest";
import { normalizeExternalAuthIconUrl } from "@/lib/externalAuthProviders";
import type {
	AdminExternalAuthProviderInfo,
	ExternalAuthProviderKindInfo,
} from "@/types/api";
import {
	connectionRequirementsMissing,
	createPayload,
	defaultScopesForKind,
	type ExternalAuthProviderFormData,
	formFromProvider,
	microsoftTenantFromIssuerUrl,
	microsoftTenantModeForValue,
	parseAllowedDomains,
	requiredFieldsMissing,
	testParamsPayload,
	updatePayload,
} from "./shared";

function providerKind(
	kind: ExternalAuthProviderKindInfo["kind"],
	overrides: Partial<ExternalAuthProviderKindInfo> = {},
): ExternalAuthProviderKindInfo {
	return {
		authorization_url_required: false,
		default_scopes: "openid email profile",
		description: kind,
		display_name: kind,
		issuer_url_required: false,
		kind,
		manual_endpoint_configuration_supported: false,
		protocol: kind === "github" || kind === "qq" ? "oauth2" : "oidc",
		supports_discovery: false,
		supports_email_verified_claim: true,
		supports_pkce: true,
		token_url_required: false,
		userinfo_url_required: false,
		...overrides,
	};
}

function form(
	overrides: Partial<ExternalAuthProviderFormData> = {},
): ExternalAuthProviderFormData {
	return {
		allowedDomains: "",
		authorizationUrl: "https://id.example.com/oauth/authorize",
		autoLinkVerifiedEmailEnabled: false,
		autoProvisionEnabled: false,
		avatarUrlClaim: "",
		clientId: "client-id",
		clientSecret: "secret",
		displayName: "Provider",
		displayNameClaim: "",
		emailClaim: "",
		emailVerifiedClaim: "",
		enabled: true,
		groupsClaim: "",
		iconUrl: "https://cdn.example.com/provider.svg",
		issuerUrl: "https://id.example.com",
		key: "provider",
		microsoftTenant: "common",
		microsoftTenantMode: "common",
		providerKind: "github",
		requireEmailVerified: true,
		scopes: "custom scope",
		subjectClaim: "",
		tokenUrl: "https://id.example.com/oauth/token",
		userinfoUrl: "https://id.example.com/oauth/userinfo",
		usernameClaim: "",
		...overrides,
	};
}

describe("external auth provider shared helpers", () => {
	it("uses provider kind default scopes from the backend descriptor", () => {
		expect(
			defaultScopesForKind(
				providerKind("oidc", {
					default_scopes: " openid backend_profile backend_email ",
				}),
			),
		).toBe("openid backend_profile backend_email");
		expect(
			defaultScopesForKind(
				providerKind("oidc", {
					default_scopes: " ",
				}),
			),
		).toBe("openid email profile");
	});

	it("omits fixed connection URLs while preserving backend-provided scopes", () => {
		const kind = providerKind("github", {
			default_scopes: "read:user user:email",
		});
		const create = createPayload(form({ scopes: " " }), kind);
		const test = testParamsPayload(form({ scopes: " " }), kind);
		const update = updatePayload(form({ scopes: " " }), kind);

		expect(create).toMatchObject({
			authorization_url: null,
			issuer_url: null,
			scopes: "read:user user:email",
			token_url: null,
			userinfo_url: null,
		});
		expect(test).toMatchObject({
			authorization_url: null,
			issuer_url: null,
			scopes: "read:user user:email",
			token_url: null,
			userinfo_url: null,
		});
		expect(create).not.toHaveProperty("key");
		expect(create).toMatchObject({
			icon_url: "https://cdn.example.com/provider.svg",
			provider_kind: "github",
		});
		expect(update).toMatchObject({
			scopes: "read:user user:email",
		});
		expect(JSON.stringify(update)).not.toContain("authorization_url");
		expect(JSON.stringify(update)).not.toContain("issuer_url");
		expect(JSON.stringify(update)).not.toContain("token_url");
		expect(JSON.stringify(update)).not.toContain("userinfo_url");
		expect(update).not.toHaveProperty("key");
		expect(update).not.toHaveProperty("provider_kind");
	});

	it("keeps configurable URLs and scopes for generic OAuth2", () => {
		const kind = providerKind("generic_oauth2", {
			authorization_url_required: true,
			manual_endpoint_configuration_supported: true,
			protocol: "oauth2",
			token_url_required: true,
			userinfo_url_required: true,
		});
		const payload = createPayload(
			form({ providerKind: "generic_oauth2" }),
			kind,
		);

		expect(payload).toMatchObject({
			authorization_url: "https://id.example.com/oauth/authorize",
			icon_url: "https://cdn.example.com/provider.svg",
			provider_kind: "generic_oauth2",
			scopes: "custom scope",
			token_url: "https://id.example.com/oauth/token",
			userinfo_url: "https://id.example.com/oauth/userinfo",
		});
	});

	it("serializes full single-provider policy, claims, domains and Microsoft options", () => {
		const kind = providerKind("microsoft");
		const payload = createPayload(
			form({
				allowedDomains: "Example.COM, @example.org\nexample.com\n",
				autoLinkVerifiedEmailEnabled: true,
				autoProvisionEnabled: true,
				avatarUrlClaim: "picture",
				displayNameClaim: "name",
				emailClaim: "mail",
				emailVerifiedClaim: "email_verified",
				groupsClaim: "groups",
				microsoftTenant: "TENANT.Example.COM",
				microsoftTenantMode: "custom",
				providerKind: "microsoft",
				requireEmailVerified: false,
				subjectClaim: "oid",
				usernameClaim: "preferred_username",
			}),
			kind,
		);

		expect(payload).toMatchObject({
			allowed_domains: ["example.com", "example.org"],
			auto_link_verified_email_enabled: true,
			auto_provision_enabled: true,
			avatar_url_claim: "picture",
			display_name_claim: "name",
			email_claim: "mail",
			email_verified_claim: "email_verified",
			groups_claim: "groups",
			options: { microsoft: { tenant: "tenant.example.com" } },
			provider_kind: "microsoft",
			require_email_verified: false,
			subject_claim: "oid",
			username_claim: "preferred_username",
		});
	});

	it("serializes blank optional values as nullable create fields and omitted update fields", () => {
		const kind = providerKind("oidc", {
			default_scopes: "openid custom_profile custom_email",
			issuer_url_required: true,
			supports_discovery: true,
		});
		const blankForm = form({
			clientSecret: " ",
			iconUrl: " ",
			issuerUrl: " https://id.example.com ",
			providerKind: "oidc",
			scopes: " ",
		});
		const create = createPayload(blankForm, kind);
		const update = updatePayload(blankForm, kind);

		expect(create).toMatchObject({
			client_secret: null,
			icon_url: null,
			issuer_url: "https://id.example.com",
			scopes: "openid custom_profile custom_email",
		});
		expect(update).toMatchObject({
			client_secret: undefined,
			icon_url: undefined,
			issuer_url: "https://id.example.com",
			scopes: "openid custom_profile custom_email",
		});
		expect(JSON.stringify(update)).not.toContain("client_secret");
		expect(JSON.stringify(update)).not.toContain("icon_url");
	});

	it("clears optional claim fields as null while preserving configured redacted secret on update", () => {
		const update = updatePayload(
			form({
				avatarUrlClaim: " ",
				clientSecret: "***REDACTED***",
				displayNameClaim: "",
				emailClaim: " ",
				emailVerifiedClaim: "",
				groupsClaim: "",
				providerKind: "oidc",
				subjectClaim: "",
				usernameClaim: " ",
			}),
			providerKind("oidc", {
				issuer_url_required: true,
				supports_discovery: true,
			}),
		);

		expect(update).toMatchObject({
			avatar_url_claim: null,
			display_name_claim: null,
			email_claim: null,
			email_verified_claim: null,
			groups_claim: null,
			subject_claim: null,
			username_claim: null,
		});
		expect(update).not.toHaveProperty("client_secret");
		expect(JSON.stringify(update)).not.toContain("***REDACTED***");
	});

	it("keeps redacted secrets out of test params and includes provider options", () => {
		const payload = testParamsPayload(
			form({
				clientSecret: "***REDACTED***",
				microsoftTenant: "organizations",
				microsoftTenantMode: "organizations",
				providerKind: "microsoft",
			}),
			providerKind("microsoft"),
		);

		expect(payload).toMatchObject({
			client_secret: null,
			options: { microsoft: { tenant: "organizations" } },
			provider_kind: "microsoft",
		});
	});

	it("normalizes allowed domain lists with trimming, leading @ removal, lowercase and dedupe", () => {
		expect(
			parseAllowedDomains(
				" Example.COM, @Example.org\n@@sub.example.net\n\nexample.com ",
			),
		).toEqual(["example.com", "example.org", "sub.example.net"]);
	});

	it("requires a Microsoft custom tenant for save and connection test readiness", () => {
		const kind = providerKind("microsoft");
		const missingTenant = form({
			microsoftTenant: " ",
			microsoftTenantMode: "custom",
			providerKind: "microsoft",
		});
		const completeTenant = form({
			microsoftTenant: "tenant.example.com",
			microsoftTenantMode: "custom",
			providerKind: "microsoft",
		});

		expect(requiredFieldsMissing(missingTenant, kind)).toBe(true);
		expect(connectionRequirementsMissing(missingTenant, kind)).toBe(true);
		expect(requiredFieldsMissing(completeTenant, kind)).toBe(false);
		expect(connectionRequirementsMissing(completeTenant, kind)).toBe(false);
	});

	it("maps provider info into form data including claims, policy flags, domains and configured secret", () => {
		const provider: AdminExternalAuthProviderInfo = {
			allowed_domains: ["example.com", "example.org"],
			authorization_url: null,
			auto_link_verified_email_enabled: true,
			auto_provision_enabled: true,
			avatar_url_claim: "picture",
			client_id: "client",
			client_secret: null,
			client_secret_configured: true,
			created_at: "2026-01-01T00:00:00Z",
			display_name: "Microsoft",
			display_name_claim: "name",
			email_claim: "mail",
			email_verified_claim: "email_verified",
			enabled: true,
			groups_claim: "groups",
			icon_url: null,
			id: 1,
			issuer_url: null,
			key: "microsoft",
			options: { microsoft: { tenant: "organizations" } },
			protocol: "oidc",
			provider_kind: "microsoft",
			require_email_verified: false,
			scopes: "openid profile email",
			subject_claim: "sub_id",
			token_url: null,
			updated_at: "2026-01-01T00:00:00Z",
			userinfo_url: null,
			username_claim: "upn",
		};

		expect(formFromProvider(provider)).toMatchObject({
			allowedDomains: "example.com, example.org",
			autoLinkVerifiedEmailEnabled: true,
			autoProvisionEnabled: true,
			avatarUrlClaim: "picture",
			clientId: "client",
			clientSecret: "***REDACTED***",
			displayName: "Microsoft",
			displayNameClaim: "name",
			emailClaim: "mail",
			emailVerifiedClaim: "email_verified",
			groupsClaim: "groups",
			microsoftTenant: "organizations",
			microsoftTenantMode: "organizations",
			requireEmailVerified: false,
			subjectClaim: "sub_id",
			usernameClaim: "upn",
		});
	});

	it("derives Microsoft tenant modes from preset, custom and legacy issuer URL values", () => {
		expect(microsoftTenantModeForValue("Organizations")).toBe("organizations");
		expect(microsoftTenantModeForValue("TENANT.Example.COM")).toBe("custom");
		expect(
			microsoftTenantFromIssuerUrl(
				"https://login.microsoftonline.com/Organizations/v2.0",
			),
		).toBe("organizations");
		expect(
			microsoftTenantFromIssuerUrl("https://login.microsoftonline.com/common"),
		).toBe("");
		expect(
			microsoftTenantFromIssuerUrl("https://example.com/organizations/v2.0"),
		).toBe("");
	});

	it("normalizes external auth icon URLs with safe boundary rules", () => {
		expect(
			normalizeExternalAuthIconUrl(" https://cdn.example.com/a.svg "),
		).toBe("https://cdn.example.com/a.svg");
		expect(normalizeExternalAuthIconUrl("/static/external-auth/a.svg")).toBe(
			"/static/external-auth/a.svg",
		);

		expect(normalizeExternalAuthIconUrl(null)).toBe("");
		expect(normalizeExternalAuthIconUrl("")).toBe("");
		expect(normalizeExternalAuthIconUrl("http://cdn.example.com/a.svg")).toBe(
			"",
		);
		expect(normalizeExternalAuthIconUrl("//cdn.example.com/a.svg")).toBe("");
		expect(normalizeExternalAuthIconUrl("static/external-auth/a.svg")).toBe("");
		expect(normalizeExternalAuthIconUrl("javascript:alert(1)")).toBe("");
		expect(
			normalizeExternalAuthIconUrl("https://cdn.example.com/a.svg#v1"),
		).toBe("");
		expect(normalizeExternalAuthIconUrl("/static/external-auth/a.svg#v1")).toBe(
			"",
		);
	});
});
