import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AdminSettingsPage from "@/pages/admin/admin-settings-page/AdminSettingsPage";
import { adminConfigService } from "@/services/adminService";
import { createMockTWithInterpolation } from "@/test/i18n";
import type {
	ConfigSchemaItem,
	SystemConfig,
	TemplateVariableGroup,
} from "@/types/api";

// Settings copy asserted below uses the en-US locale strings so the tests
// verify the key → copy wiring instead of bare key literals.
const settingsTranslations = {
	mail_send_test_email: "Send test email",
	mail_template_variable_link: "Show template variables",
	mail_template_variables_dialog_empty:
		"This template has no variables to show.",
	mail_test_email_recipient_label: "Recipient email",
	settings_category_auth: "Account Auth",
	settings_category_mail: "Mail",
	settings_category_network: "Network",
	settings_category_runtime: "Runtime",
	settings_category_site: "Site",
	settings_category_texture: "Textures",
	settings_category_user: "Users",
	settings_empty_desc: "Adjust the group or search term and query again.",
	settings_empty_title: "No settings found",
	settings_enum_set_search_placeholder: "Filter options",
	settings_sensitive_keep_placeholder: "Leave blank to keep the current value",
	settings_string_array_add_item: "Add item",
	settings_time_unit_hours: "Hours",
	settings_value_on: "Enabled",
	yggdrasil_rotate_signature_key: "Rotate signing key",
};

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		i18n: { changeLanguage: vi.fn(), language: "en-US" },
		t: createMockTWithInterpolation(settingsTranslations),
	}),
}));

vi.mock("@/services/adminService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/adminService")>();
	return {
		...actual,
		adminConfigService: {
			...actual.adminConfigService,
			list: vi.fn(),
			schema: vi.fn(),
			set: vi.fn(),
			templateVariables: vi.fn(),
			sendTestEmail: vi.fn(),
			previewCaptcha: vi.fn(),
			rotateYggdrasilSignatureKey: vi.fn(),
		},
	};
});

const config = {
	category: "site.public",
	description: "Public site URLs",
	id: 1,
	is_sensitive: false,
	key: "public_site_url",
	namespace: "site",
	requires_restart: false,
	source: "system",
	updated_at: "2026-06-15T00:00:00.000Z",
	updated_by: null,
	value: ["https://example.com"],
	value_type: "string_array",
	visibility: "public",
} satisfies SystemConfig;

const customConfig = {
	...config,
	id: 2,
	key: "custom.banner",
	source: "custom",
	value: "hello",
	value_type: "string",
	visibility: "authenticated",
} satisfies SystemConfig;

const schema = {
	category: "site.public",
	description: "Public site URLs",
	description_i18n_key: "",
	is_sensitive: false,
	key: "public_site_url",
	label_i18n_key: "",
	options: [],
	requires_restart: false,
	value_type: "string_array",
} satisfies ConfigSchemaItem;

const mailConfig = {
	...config,
	category: "mail.config",
	id: 3,
	key: "mail_smtp_host",
	namespace: "mail",
	value: "smtp.example.com",
	value_type: "string",
} satisfies SystemConfig;

const mailTemplateSubjectConfig = {
	...config,
	category: "mail.template",
	id: 4,
	key: "mail_template_password_reset_subject",
	namespace: "mail",
	value: "Reset password",
	value_type: "string",
} satisfies SystemConfig;

const mailTemplateHtmlConfig = {
	...config,
	category: "mail.template",
	id: 5,
	key: "mail_template_password_reset_html",
	namespace: "mail",
	value: "<p>{{reset_url}}</p>",
	value_type: "multiline",
} satisfies SystemConfig;

const yggdrasilConfig = {
	...config,
	category: "yggdrasil.signing",
	id: 6,
	key: "yggdrasil_signature_private_key",
	namespace: "yggdrasil",
	is_sensitive: true,
	value: "",
	value_type: "multiline",
} satisfies SystemConfig;

const runtimeNumberConfig = {
	...config,
	category: "runtime.tasks",
	id: 7,
	key: "background_task_max_concurrency",
	namespace: "runtime",
	value: "4",
	value_type: "number",
} satisfies SystemConfig;

const enumSetConfig = {
	...config,
	category: "network.cors",
	id: 8,
	key: "cors_allowed_origins",
	namespace: "network",
	value: ["https://alpha.example"],
	value_type: "string_enum_set",
} satisfies SystemConfig;

const enumSetSchema = {
	category: "network.cors",
	description: "Allowed origins",
	description_i18n_key: "",
	is_sensitive: false,
	key: "cors_allowed_origins",
	label_i18n_key: "",
	options: [
		{ label_i18n_key: "", value: "https://alpha.example" },
		{ label_i18n_key: "", value: "https://beta.example" },
		{ label_i18n_key: "", value: "https://gamma.example" },
	],
	requires_restart: false,
	value_type: "string_enum_set",
} satisfies ConfigSchemaItem;

const sensitiveStringConfig = {
	...config,
	category: "auth.session",
	id: 9,
	is_sensitive: true,
	key: "auth_session_secret",
	namespace: "auth",
	value: "stored-secret",
	value_type: "string",
	visibility: "private",
} satisfies SystemConfig;

const authAccessTokenTtlConfig = {
	...config,
	category: "auth.session",
	id: 12,
	key: "auth_access_token_ttl_secs",
	namespace: "auth",
	value: "3600",
	value_type: "number",
} satisfies SystemConfig;

const userAvatarConfig = {
	...config,
	category: "user.avatar",
	id: 10,
	key: "gravatar_base_url",
	namespace: "system",
	value: "https://www.gravatar.com/avatar",
	value_type: "string",
} satisfies SystemConfig;

const textureLibraryEnabledConfig = {
	...config,
	category: "texture.library",
	id: 13,
	key: "texture_library_enabled",
	namespace: "texture",
	value: "true",
	value_type: "boolean",
} satisfies SystemConfig;

const textureLibraryReviewRequiredConfig = {
	...config,
	category: "texture.library",
	id: 14,
	key: "texture_library_review_required",
	namespace: "texture",
	value: "true",
	value_type: "boolean",
} satisfies SystemConfig;

const textureLibraryEnabledSchema = {
	category: "texture.library",
	description: "Enable the public texture library",
	description_i18n_key: "settings_item_texture_library_enabled_desc",
	is_sensitive: false,
	key: "texture_library_enabled",
	label_i18n_key: "settings_item_texture_library_enabled_label",
	options: [],
	requires_restart: false,
	value_type: "boolean",
} satisfies ConfigSchemaItem;

const textureLibraryReviewRequiredSchema = {
	category: "texture.library",
	description: "Require review before publishing",
	description_i18n_key: "settings_item_texture_library_review_required_desc",
	is_sensitive: false,
	key: "texture_library_review_required",
	label_i18n_key: "settings_item_texture_library_review_required_label",
	options: [],
	requires_restart: false,
	value_type: "boolean",
} satisfies ConfigSchemaItem;

const unknownCategoryConfig = {
	...config,
	category: "legacy.cloud",
	id: 11,
	key: "legacy_cloud_storage",
	namespace: "legacy",
	value: "enabled",
	value_type: "string",
} satisfies SystemConfig;

const templateVariableGroup = {
	category: "mail.template",
	label_i18n_key: "settings_mail_template_group_password_reset",
	template_code: "password_reset",
	variables: [
		{
			description_i18n_key: "settings_template_variable_reset_url_desc",
			label_i18n_key: "settings_template_variable_reset_url_label",
			token: "{{reset_url}}",
		},
	],
} satisfies TemplateVariableGroup;

function mockSettingsLoad({
	items,
	nextSchema = [],
	variableGroups = [],
}: {
	items: SystemConfig[];
	nextSchema?: ConfigSchemaItem[];
	variableGroups?: TemplateVariableGroup[];
}) {
	vi.mocked(adminConfigService.list).mockResolvedValue({
		items,
		limit: 100,
		next_cursor: null,
		total: items.length,
	});
	vi.mocked(adminConfigService.schema).mockResolvedValue(nextSchema);
	vi.mocked(adminConfigService.templateVariables).mockResolvedValue(
		variableGroups,
	);
}

function renderPage(initialEntry = "/admin/settings") {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<Routes>
				<Route path="/admin/settings" element={<AdminSettingsPage />} />
				<Route
					path="/admin/settings/:category"
					element={<AdminSettingsPage />}
				/>
			</Routes>
			<LocationProbe />
		</MemoryRouter>,
	);
}

function LocationProbe() {
	const location = useLocation();
	return <div data-testid="location-pathname">{location.pathname}</div>;
}

describe("AdminSettingsPage", () => {
	beforeEach(() => {
		vi.mocked(adminConfigService.list).mockReset();
		vi.mocked(adminConfigService.schema).mockReset();
		vi.mocked(adminConfigService.set).mockReset();
		vi.mocked(adminConfigService.templateVariables).mockReset();
		vi.mocked(adminConfigService.sendTestEmail).mockReset();
		vi.mocked(adminConfigService.previewCaptcha).mockReset();
		vi.mocked(adminConfigService.rotateYggdrasilSignatureKey).mockReset();
	});

	it("keeps string-array input focused while editing a row", async () => {
		mockSettingsLoad({ items: [config], nextSchema: [schema] });

		renderPage();

		const input = await screen.findByDisplayValue("https://example.com");
		input.focus();
		fireEvent.change(input, {
			target: { value: "https://example.com/account" },
		});

		expect(input).toHaveValue("https://example.com/account");
		expect(document.activeElement).toBe(input);
	});

	it("does not render a duplicate active-category summary above the setting groups", async () => {
		mockSettingsLoad({ items: [config], nextSchema: [schema] });

		renderPage();

		await screen.findByDisplayValue("https://example.com");

		expect(
			screen.queryByRole("heading", { name: "Site" }),
		).not.toBeInTheDocument();
		expect(
			screen.getByRole("heading", { name: "Site Public" }),
		).toBeInTheDocument();
	});

	it("does not show the current group item count in the main content header", async () => {
		mockSettingsLoad({ items: [config], nextSchema: [schema] });

		renderPage();

		const heading = await screen.findByRole("heading", { name: "Site Public" });
		const groupHeader = heading.closest(".border-b");
		expect(groupHeader).not.toBeNull();
		expect(within(groupHeader as HTMLElement).queryByText("1")).toBeNull();
	});

	it("does not send visibility when saving system config changes", async () => {
		mockSettingsLoad({ items: [config], nextSchema: [schema] });
		vi.mocked(adminConfigService.set).mockResolvedValue({
			config: {
				...config,
				value: ["https://example.com/account"],
			},
			warnings: [],
		});

		renderPage();

		const input = await screen.findByDisplayValue("https://example.com");
		fireEvent.change(input, {
			target: { value: "https://example.com/account" },
		});
		fireEvent.click(screen.getAllByRole("button", { name: /save/i })[0]);

		expect(adminConfigService.set).toHaveBeenCalledWith("public_site_url", {
			value: ["https://example.com/account"],
		});
		await waitFor(() =>
			expect(screen.getByTestId("settings-save-bar")).toHaveAttribute(
				"data-phase",
				"exiting",
			),
		);
		expect(screen.getByTestId("settings-save-bar")).toHaveAttribute(
			"aria-hidden",
			"true",
		);
		await waitFor(() =>
			expect(screen.queryByText("Unsaved")).not.toBeInTheDocument(),
		);
	});

	it("keeps visibility when saving custom config changes", async () => {
		mockSettingsLoad({ items: [customConfig] });
		vi.mocked(adminConfigService.set).mockResolvedValue({
			config: {
				...customConfig,
				value: "hello again",
			},
			warnings: [],
		});

		renderPage();

		const input = await screen.findByDisplayValue("hello");
		fireEvent.change(input, {
			target: { value: "hello again" },
		});
		fireEvent.click(screen.getAllByRole("button", { name: /save/i })[0]);

		expect(adminConfigService.set).toHaveBeenCalledWith("custom.banner", {
			value: "hello again",
			visibility: "authenticated",
		});
	});

	it("shows the empty state when settings fail to load", async () => {
		vi.mocked(adminConfigService.list).mockRejectedValue(new Error("offline"));
		vi.mocked(adminConfigService.schema).mockResolvedValue([]);
		vi.mocked(adminConfigService.templateVariables).mockResolvedValue([]);

		renderPage();

		expect(await screen.findByText("No settings found")).toBeInTheDocument();
		expect(
			screen.getByText("Adjust the group or search term and query again."),
		).toBeInTheDocument();
	});

	it("keeps unsupported root categories out of the editable panel", async () => {
		mockSettingsLoad({ items: [unknownCategoryConfig] });

		renderPage();

		expect(await screen.findByText("No settings found")).toBeInTheDocument();
		expect(screen.queryByDisplayValue("enabled")).not.toBeInTheDocument();
	});

	it("shows user avatar settings under the user category", async () => {
		mockSettingsLoad({ items: [userAvatarConfig] });

		renderPage();

		fireEvent.click(
			await screen.findByRole("link", {
				name: /Users/i,
			}),
		);

		expect(
			await screen.findByRole("heading", { name: "User Avatar" }),
		).toBeInTheDocument();
		expect(
			screen.getByDisplayValue("https://www.gravatar.com/avatar"),
		).toBeInTheDocument();
	});

	it("opens a settings category directly from the URL", async () => {
		mockSettingsLoad({
			items: [config, textureLibraryEnabledConfig],
			nextSchema: [schema, textureLibraryEnabledSchema],
		});

		renderPage("/admin/settings/texture");

		expect(
			await screen.findByRole("heading", { name: "Texture Library" }),
		).toBeInTheDocument();
		expect(
			screen.queryByDisplayValue("https://example.com"),
		).not.toBeInTheDocument();
		expect(screen.getByTestId("location-pathname")).toHaveTextContent(
			"/admin/settings/texture",
		);
	});

	it("redirects the settings root to the site category", async () => {
		mockSettingsLoad({ items: [config, userAvatarConfig] });

		renderPage();

		expect(
			await screen.findByDisplayValue("https://example.com"),
		).toBeInTheDocument();
		await waitFor(() =>
			expect(screen.getByTestId("location-pathname")).toHaveTextContent(
				"/admin/settings/site",
			),
		);
	});

	it("updates the browser URL when a settings category is selected", async () => {
		mockSettingsLoad({ items: [config, userAvatarConfig] });

		renderPage();

		fireEvent.click(
			await screen.findByRole("link", {
				name: /Users/i,
			}),
		);

		expect(
			await screen.findByRole("heading", { name: "User Avatar" }),
		).toBeInTheDocument();
		expect(screen.getByTestId("location-pathname")).toHaveTextContent(
			"/admin/settings/user",
		);
	});

	it("replaces an unsupported settings category URL with the first available category", async () => {
		mockSettingsLoad({ items: [config, userAvatarConfig] });

		renderPage("/admin/settings/legacy");

		expect(
			await screen.findByDisplayValue("https://example.com"),
		).toBeInTheDocument();
		await waitFor(() =>
			expect(screen.getByTestId("location-pathname")).toHaveTextContent(
				"/admin/settings/site",
			),
		);
	});

	it("shows public texture library settings under the texture category", async () => {
		mockSettingsLoad({
			items: [textureLibraryEnabledConfig, textureLibraryReviewRequiredConfig],
			nextSchema: [
				textureLibraryEnabledSchema,
				textureLibraryReviewRequiredSchema,
			],
		});

		renderPage();

		const textureCategory = await screen.findByRole("link", {
			name: /Textures/i,
		});
		fireEvent.click(textureCategory);

		await waitFor(() => {
			expect(screen.getAllByText("Enabled")).toHaveLength(2);
		});
		expect(
			screen.getByRole("heading", { name: "Texture Library" }),
		).toBeInTheDocument();
		expect(screen.getByText("Texture Library Enabled")).toBeInTheDocument();
		expect(
			screen.getByText("Texture Library Review Required"),
		).toBeInTheDocument();
	});

	it("loads later config pages so texture library settings are not hidden by pagination", async () => {
		vi.mocked(adminConfigService.list)
			.mockResolvedValueOnce({
				items: [config],
				limit: 100,
				next_cursor: { id: config.id },
				total: 3,
			})
			.mockResolvedValueOnce({
				items: [
					textureLibraryEnabledConfig,
					textureLibraryReviewRequiredConfig,
				],
				limit: 100,
				next_cursor: null,
				total: 3,
			});
		vi.mocked(adminConfigService.schema).mockResolvedValue([
			schema,
			textureLibraryEnabledSchema,
			textureLibraryReviewRequiredSchema,
		]);
		vi.mocked(adminConfigService.templateVariables).mockResolvedValue([]);

		renderPage();

		const textureCategory = await screen.findByRole("link", {
			name: /Textures/i,
		});
		fireEvent.click(textureCategory);

		expect(adminConfigService.list).toHaveBeenNthCalledWith(1, {
			after_id: undefined,
			limit: 100,
		});
		expect(adminConfigService.list).toHaveBeenNthCalledWith(2, {
			after_id: config.id,
			limit: 100,
		});
		expect(
			await screen.findByRole("heading", { name: "Texture Library" }),
		).toBeInTheDocument();
		expect(screen.getByText("Texture Library Enabled")).toBeInTheDocument();
		expect(
			screen.getByText("Texture Library Review Required"),
		).toBeInTheDocument();
	});

	it("does not reveal sensitive stored values and only saves an explicit replacement", async () => {
		mockSettingsLoad({ items: [sensitiveStringConfig] });
		vi.mocked(adminConfigService.set).mockResolvedValue({
			config: {
				...sensitiveStringConfig,
				value: "new-secret",
			},
			warnings: [],
		});

		renderPage();

		fireEvent.click(await screen.findByRole("link", { name: /Account Auth/i }));
		expect(screen.queryByDisplayValue("stored-secret")).not.toBeInTheDocument();

		const input = screen.getByPlaceholderText(
			"Leave blank to keep the current value",
		);
		expect(input).toHaveValue("");
		fireEvent.change(input, { target: { value: "new-secret" } });
		fireEvent.click(screen.getAllByRole("button", { name: /save/i })[0]);

		expect(adminConfigService.set).toHaveBeenCalledWith("auth_session_secret", {
			value: "new-secret",
		});
	});

	it("saves a cleared number draft as an empty value", async () => {
		mockSettingsLoad({ items: [runtimeNumberConfig] });
		vi.mocked(adminConfigService.set).mockResolvedValue({
			config: {
				...runtimeNumberConfig,
				value: "",
			},
			warnings: [],
		});

		renderPage();

		fireEvent.click(
			await screen.findByRole("link", {
				name: /Runtime/i,
			}),
		);
		const input = await screen.findByDisplayValue(4);
		fireEvent.change(input, { target: { value: "" } });
		fireEvent.click(screen.getAllByRole("button", { name: /save/i })[0]);

		expect(adminConfigService.set).toHaveBeenCalledWith(
			"background_task_max_concurrency",
			{ value: "" },
		);
	});

	it("renders time unit selectors with translated labels", async () => {
		mockSettingsLoad({ items: [authAccessTokenTtlConfig] });

		renderPage();

		fireEvent.click(await screen.findByRole("link", { name: /Account Auth/i }));

		expect(await screen.findByText("Hours")).toBeInTheDocument();
		expect(screen.queryByText("hours")).not.toBeInTheDocument();
	});

	it("compacts trimmed string-array rows before saving", async () => {
		mockSettingsLoad({ items: [config], nextSchema: [schema] });
		vi.mocked(adminConfigService.set).mockResolvedValue({
			config: {
				...config,
				value: ["https://example.com/account"],
			},
			warnings: [],
		});

		renderPage();

		const input = await screen.findByDisplayValue("https://example.com");
		fireEvent.change(input, {
			target: { value: "  https://example.com/account  " },
		});
		fireEvent.click(
			screen.getByRole("button", {
				name: "Add item",
			}),
		);
		const rows = screen.getAllByRole("textbox");
		fireEvent.change(rows[rows.length - 1], {
			target: { value: "   " },
		});
		fireEvent.click(screen.getAllByRole("button", { name: /save/i })[0]);

		expect(adminConfigService.set).toHaveBeenCalledWith("public_site_url", {
			value: ["https://example.com/account"],
		});
	});

	it("keeps selected enum-set values while filtering and saves sorted selections", async () => {
		mockSettingsLoad({
			items: [enumSetConfig],
			nextSchema: [enumSetSchema],
		});
		vi.mocked(adminConfigService.set).mockResolvedValue({
			config: {
				...enumSetConfig,
				value: ["https://alpha.example", "https://beta.example"],
			},
			warnings: [],
		});

		renderPage();

		fireEvent.click(
			await screen.findByRole("link", {
				name: /Network/i,
			}),
		);
		const filter = await screen.findByPlaceholderText("Filter options");
		fireEvent.change(filter, { target: { value: "beta" } });

		expect(screen.queryByText("https://alpha.example")).not.toBeInTheDocument();
		fireEvent.click(
			screen.getByRole("button", { name: "https://beta.example" }),
		);
		fireEvent.click(screen.getAllByRole("button", { name: /save/i })[0]);

		expect(adminConfigService.set).toHaveBeenCalledWith(
			"cors_allowed_origins",
			{
				value: ["https://alpha.example", "https://beta.example"],
			},
		);
	});

	it("keeps mail template groups collapsed until the group header is opened", async () => {
		mockSettingsLoad({
			items: [mailTemplateSubjectConfig, mailTemplateHtmlConfig],
			variableGroups: [templateVariableGroup],
		});

		renderPage();

		fireEvent.click(await screen.findByRole("link", { name: /Mail/i }));

		const groupButton = await screen.findByRole("button", {
			name: /Password reset/i,
		});
		expect(groupButton).toHaveAttribute("aria-expanded", "false");
		expect(
			screen.queryByDisplayValue("Reset password"),
		).not.toBeInTheDocument();

		fireEvent.click(groupButton);

		expect(groupButton).toHaveAttribute("aria-expanded", "true");
		expect(
			await screen.findByDisplayValue("Reset password"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: /Show template variables/i }),
		).toBeInTheDocument();
	});

	it("opens mail template variable dialog with available variables", async () => {
		mockSettingsLoad({
			items: [mailTemplateSubjectConfig, mailTemplateHtmlConfig],
			variableGroups: [templateVariableGroup],
		});

		renderPage();

		fireEvent.click(await screen.findByRole("link", { name: /Mail/i }));
		fireEvent.click(
			await screen.findByRole("button", { name: /Password reset/i }),
		);
		fireEvent.click(
			await screen.findByRole("button", {
				name: /Show template variables/i,
			}),
		);

		expect(await screen.findAllByText("{{reset_url}}")).toHaveLength(2);
	});

	it("shows an empty variable dialog when a template has no variable group", async () => {
		mockSettingsLoad({
			items: [mailTemplateSubjectConfig, mailTemplateHtmlConfig],
		});

		renderPage();

		fireEvent.click(await screen.findByRole("link", { name: /Mail/i }));
		fireEvent.click(
			await screen.findByRole("button", { name: /Password reset/i }),
		);

		const variableButton = await screen.findByRole("button", {
			name: /Show template variables/i,
		});
		expect(variableButton).toBeEnabled();
		fireEvent.click(variableButton);

		expect(
			await screen.findByText("This template has no variables to show."),
		).toBeInTheDocument();
	});

	it("executes the mail test action with the requested recipient", async () => {
		mockSettingsLoad({ items: [mailConfig] });
		vi.mocked(adminConfigService.sendTestEmail).mockResolvedValue({
			message: "sent",
			value: null,
		});

		renderPage();

		fireEvent.click(await screen.findByRole("link", { name: /Mail/i }));
		fireEvent.click(
			await screen.findByRole("button", { name: /Send test email/i }),
		);
		fireEvent.change(await screen.findByLabelText(/Recipient email/i), {
			target: { value: "ops@example.com" },
		});
		const sendButtons = screen.getAllByRole("button", {
			name: /Send test email/i,
		});
		fireEvent.click(sendButtons[sendButtons.length - 1]);

		await waitFor(() =>
			expect(adminConfigService.sendTestEmail).toHaveBeenCalledWith(
				"ops@example.com",
			),
		);
	});

	it("rotates the Yggdrasil signature key and refreshes config drafts", async () => {
		vi.mocked(adminConfigService.list)
			.mockResolvedValueOnce({
				items: [yggdrasilConfig],
				limit: 100,
				next_cursor: null,
				total: 1,
			})
			.mockResolvedValueOnce({
				items: [yggdrasilConfig],
				limit: 100,
				next_cursor: null,
				total: 1,
			});
		vi.mocked(adminConfigService.schema).mockResolvedValue([]);
		vi.mocked(adminConfigService.templateVariables).mockResolvedValue([]);
		vi.mocked(adminConfigService.rotateYggdrasilSignatureKey).mockResolvedValue(
			{
				message: "rotated",
				value: null,
			},
		);

		renderPage();

		fireEvent.click(
			await screen.findByRole("button", {
				name: /Rotate signing key/i,
			}),
		);

		await waitFor(() =>
			expect(
				adminConfigService.rotateYggdrasilSignatureKey,
			).toHaveBeenCalledTimes(1),
		);
		await waitFor(() =>
			expect(adminConfigService.list).toHaveBeenCalledTimes(2),
		);
	});
});
