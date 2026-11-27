import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import AdminExternalAuthPage from "@/pages/admin/admin-external-auth-page/AdminExternalAuthPage";
import type {
	AdminExternalAuthProviderInfo,
	ExternalAuthProviderKindInfo,
} from "@/types/api";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const adminExternalAuthServiceMock = vi.hoisted(() => ({
	create: vi.fn(),
	delete: vi.fn(),
	kinds: vi.fn(),
	list: vi.fn(),
	test: vi.fn(),
	testParams: vi.fn(),
	update: vi.fn(),
}));

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		i18n: { language: "en-US" },
		t: (
			key: string,
			values?: Record<string, string | number | null | undefined>,
		) => {
			const suffix =
				values && Object.keys(values).length > 0
					? ` ${JSON.stringify(values)}`
					: "";
			return `${key}${suffix}`;
		},
	}),
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

vi.mock("@/services/adminService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/adminService")>();
	return {
		...actual,
		adminExternalAuthService: adminExternalAuthServiceMock,
	};
});

vi.mock("@/hooks/useApiError", () => ({
	handleApiError: (error: unknown) => {
		toastMock.error(error instanceof Error ? error.message : String(error));
	},
}));

const oidcKind = {
	authorization_url_required: false,
	default_scopes: "openid email profile",
	description: "Any OpenID Connect provider",
	display_name: "OpenID Connect",
	issuer_url_required: true,
	kind: "oidc",
	manual_endpoint_configuration_supported: false,
	protocol: "oidc",
	supports_discovery: true,
	supports_email_verified_claim: true,
	supports_pkce: false,
	token_url_required: false,
	userinfo_url_required: false,
} satisfies ExternalAuthProviderKindInfo;

const genericKind = {
	...oidcKind,
	default_scopes: "email profile",
	description: "Any OAuth 2.0 provider",
	display_name: "Generic OAuth2",
	issuer_url_required: false,
	kind: "generic_oauth2",
	manual_endpoint_configuration_supported: true,
	protocol: "oauth2",
	supports_discovery: false,
} satisfies ExternalAuthProviderKindInfo;

function provider(
	overrides: Partial<AdminExternalAuthProviderInfo> = {},
): AdminExternalAuthProviderInfo {
	return {
		allowed_domains: [],
		auto_link_verified_email_enabled: false,
		auto_provision_enabled: true,
		client_id: "client-123",
		client_secret_configured: true,
		created_at: "2026-06-20T00:00:00Z",
		display_name: "Corporate OIDC",
		enabled: true,
		id: 5,
		issuer_url: "https://id.example.com",
		key: "corporate",
		options: {},
		protocol: "oidc",
		provider_kind: "oidc",
		require_email_verified: true,
		scopes: "openid email profile",
		updated_at: "2026-06-21T00:00:00Z",
		...overrides,
	};
}

function providerPage(items: AdminExternalAuthProviderInfo[]) {
	return {
		items,
		limit: 20,
		next_cursor: null,
		total: items.length,
	};
}

function openCreateDialog() {
	fireEvent.click(
		screen.getAllByRole("button", {
			name: /admin\.externalAuth\.create/,
		})[0],
	);
	return topDialog();
}

function topDialog() {
	// Provider dialog plus two ConfirmDialogs stay mounted (keepMounted);
	// the open one is the visible, last-in-DOM-order dialog surface.
	const dialog = screen
		.getAllByRole("dialog", { hidden: true })
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

async function renderPage() {
	render(
		<MemoryRouter initialEntries={["/admin/external-auth"]}>
			<AdminExternalAuthPage />
		</MemoryRouter>,
	);
	await screen.findByText("Corporate OIDC");
}

describe("AdminExternalAuthPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		// jsdom has no Web Animations API; dialog-lifecycle needs the same
		// stub its own tests use so exit completion settles immediately.
		vi.spyOn(Element.prototype, "getAnimations").mockReturnValue([]);
		adminExternalAuthServiceMock.kinds.mockResolvedValue([
			oidcKind,
			genericKind,
		]);
		adminExternalAuthServiceMock.list.mockResolvedValue(
			providerPage([provider()]),
		);
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	it("loads providers and provider kinds into the table", async () => {
		await renderPage();

		expect(adminExternalAuthServiceMock.list).toHaveBeenCalledWith({
			after_display_name: undefined,
			after_id: undefined,
			limit: 20,
		});
		expect(adminExternalAuthServiceMock.kinds).toHaveBeenCalledTimes(1);
		expect(screen.getByText("Corporate OIDC")).toBeInTheDocument();
		expect(screen.getByText("corporate")).toBeInTheDocument();
		expect(
			screen.getByText(/admin\.externalAuth\.kind\.oidc/),
		).toBeInTheDocument();
	});

	it("opens the create wizard on the provider kind step", async () => {
		await renderPage();

		const dialog = openCreateDialog();

		// The step title renders twice (progress header + step button).
		expect(
			await within(dialog).findAllByText("admin.externalAuth.steps.type.title"),
		).not.toHaveLength(0);
		expect(
			within(dialog).getByText(/admin\.externalAuth\.kind\.oidc/),
		).toBeInTheDocument();
	});

	it("retains dialog content through close and starts fresh on reopen", async () => {
		await renderPage();

		const dialog = openCreateDialog();
		fireEvent.click(
			within(dialog).getByRole("button", {
				name: /Generic OAuth2/,
			}),
		);

		// Picking a kind advances the wizard to the connection step; that
		// step is the draft the close path must retain through exit. The
		// step titles also render inside the always-mounted step buttons,
		// so the connection panel heading is the liveness signal instead.
		expect(
			await within(dialog).findByText(
				"admin.externalAuth.dialog.connectionTitle",
			),
		).toBeInTheDocument();

		// From step 1 the footer offers Back, not Cancel; the dialog-level
		// close button is the step-independent close path.
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.close" }),
		);

		// dialog-lifecycle keeps the retained snapshot mounted through exit,
		// then disposes it once the (stubbed) exit animation settles.
		expect(
			screen.getByText("admin.externalAuth.dialog.connectionTitle"),
		).toBeInTheDocument();
		await waitFor(() => {
			expect(
				screen.queryByText("admin.externalAuth.dialog.connectionTitle"),
			).toBeNull();
		});

		// Reopening starts a fresh session on the first wizard step again.
		const reopened = openCreateDialog();
		expect(
			await within(reopened).findAllByText(
				"admin.externalAuth.steps.type.title",
			),
		).not.toHaveLength(0);
		expect(
			within(reopened).queryByText("admin.externalAuth.dialog.connectionTitle"),
		).toBeNull();
	});

	it("edits an existing provider with its form data", async () => {
		adminExternalAuthServiceMock.update.mockResolvedValue(
			provider({ display_name: "Renamed OIDC" }),
		);

		await renderPage();

		fireEvent.click(screen.getByText("Corporate OIDC"));
		const dialog = topDialog();
		// Edit mode renders the connection fields directly (no wizard).
		expect(
			await within(dialog).findByDisplayValue("Corporate OIDC"),
		).toBeInTheDocument();
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(adminExternalAuthServiceMock.update).toHaveBeenCalled();
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.externalAuth.updated",
		);
	});

	it("deletes a provider after confirmation", async () => {
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "admin.externalAuth.delete" }),
		);
		fireEvent.click(screen.getByRole("button", { name: "common.delete" }));

		await waitFor(() => {
			expect(adminExternalAuthServiceMock.delete).toHaveBeenCalledWith(5);
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.externalAuth.deleted",
		);
	});
});
