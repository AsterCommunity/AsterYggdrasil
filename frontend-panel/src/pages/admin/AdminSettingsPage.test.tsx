import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AdminSettingsPage from "@/pages/admin/AdminSettingsPage";
import { adminConfigService } from "@/services/adminService";
import type {
	ConfigSchemaItem,
	SystemConfig,
	TemplateVariableGroup,
} from "@/types/api";

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
			rotateYggdrasilSignatureKey: vi.fn(),
		},
	};
});

const config = {
	category: "site.urls",
	description: "Public site URLs",
	id: 1,
	is_sensitive: false,
	key: "site.urls",
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
	category: "site.urls",
	description: "Public site URLs",
	description_i18n_key: "",
	is_sensitive: false,
	key: "site.urls",
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
	category: "yggdrasil",
	id: 6,
	key: "yggdrasil_signature_private_key",
	namespace: "yggdrasil",
	is_sensitive: true,
	value: "",
	value_type: "multiline",
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
		limit: 500,
		offset: 0,
		total: items.length,
	});
	vi.mocked(adminConfigService.schema).mockResolvedValue(nextSchema);
	vi.mocked(adminConfigService.templateVariables).mockResolvedValue(
		variableGroups,
	);
}

describe("AdminSettingsPage", () => {
	beforeEach(() => {
		vi.mocked(adminConfigService.list).mockReset();
		vi.mocked(adminConfigService.schema).mockReset();
		vi.mocked(adminConfigService.set).mockReset();
		vi.mocked(adminConfigService.templateVariables).mockReset();
		vi.mocked(adminConfigService.sendTestEmail).mockReset();
		vi.mocked(adminConfigService.rotateYggdrasilSignatureKey).mockReset();
	});

	it("keeps string-array input focused while editing a row", async () => {
		mockSettingsLoad({ items: [config], nextSchema: [schema] });

		render(<AdminSettingsPage />);

		const input = await screen.findByDisplayValue("https://example.com");
		input.focus();
		fireEvent.change(input, {
			target: { value: "https://example.com/account" },
		});

		expect(input).toHaveValue("https://example.com/account");
		expect(document.activeElement).toBe(input);
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

		render(<AdminSettingsPage />);

		const input = await screen.findByDisplayValue("https://example.com");
		fireEvent.change(input, {
			target: { value: "https://example.com/account" },
		});
		fireEvent.click(screen.getAllByRole("button", { name: /save/i })[0]);

		expect(adminConfigService.set).toHaveBeenCalledWith("site.urls", {
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

		render(<AdminSettingsPage />);

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

	it("keeps mail template groups collapsed until the group header is opened", async () => {
		mockSettingsLoad({
			items: [mailTemplateSubjectConfig, mailTemplateHtmlConfig],
			variableGroups: [templateVariableGroup],
		});

		render(<AdminSettingsPage />);

		fireEvent.click(
			await screen.findByRole("button", { name: /settings_category_mail/i }),
		);

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
			screen.getByRole("button", { name: /mail_template_variable_link/i }),
		).toBeInTheDocument();
	});

	it("opens mail template variable dialog with available variables", async () => {
		mockSettingsLoad({
			items: [mailTemplateSubjectConfig, mailTemplateHtmlConfig],
			variableGroups: [templateVariableGroup],
		});

		render(<AdminSettingsPage />);

		fireEvent.click(
			await screen.findByRole("button", { name: /settings_category_mail/i }),
		);
		fireEvent.click(
			await screen.findByRole("button", { name: /Password reset/i }),
		);
		fireEvent.click(
			await screen.findByRole("button", {
				name: /mail_template_variable_link/i,
			}),
		);

		expect(await screen.findAllByText("{{reset_url}}")).toHaveLength(2);
	});

	it("shows an empty variable dialog when a template has no variable group", async () => {
		mockSettingsLoad({
			items: [mailTemplateSubjectConfig, mailTemplateHtmlConfig],
		});

		render(<AdminSettingsPage />);

		fireEvent.click(
			await screen.findByRole("button", { name: /settings_category_mail/i }),
		);
		fireEvent.click(
			await screen.findByRole("button", { name: /Password reset/i }),
		);

		const variableButton = await screen.findByRole("button", {
			name: /mail_template_variable_link/i,
		});
		expect(variableButton).toBeEnabled();
		fireEvent.click(variableButton);

		expect(
			await screen.findByText("mail_template_variables_dialog_empty"),
		).toBeInTheDocument();
	});

	it("executes the mail test action with the requested recipient", async () => {
		mockSettingsLoad({ items: [mailConfig] });
		vi.mocked(adminConfigService.sendTestEmail).mockResolvedValue({
			message: "sent",
			value: null,
		});

		render(<AdminSettingsPage />);

		fireEvent.click(
			await screen.findByRole("button", { name: /settings_category_mail/i }),
		);
		fireEvent.click(
			await screen.findByRole("button", { name: /mail_send_test_email/i }),
		);
		fireEvent.change(
			await screen.findByLabelText(/mail_test_email_recipient_label/i),
			{
				target: { value: "ops@example.com" },
			},
		);
		const sendButtons = screen.getAllByRole("button", {
			name: /mail_send_test_email/i,
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
				limit: 500,
				offset: 0,
				total: 1,
			})
			.mockResolvedValueOnce({
				items: [yggdrasilConfig],
				limit: 500,
				offset: 0,
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

		render(<AdminSettingsPage />);

		fireEvent.click(
			await screen.findByRole("button", {
				name: /yggdrasil_rotate_signature_key/i,
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
