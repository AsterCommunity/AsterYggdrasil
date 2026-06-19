import {
	act,
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AdminYggdrasilForwardingPage from "@/pages/admin/AdminYggdrasilForwardingPage";
import type { AdminYggdrasilSessionForwardServerInfo } from "@/types/api";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const adminYggdrasilSessionForwardServiceMock = vi.hoisted(() => ({
	create: vi.fn(),
	delete: vi.fn(),
	get: vi.fn(),
	list: vi.fn(),
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
		adminYggdrasilSessionForwardService:
			adminYggdrasilSessionForwardServiceMock,
	};
});

vi.mock("@/hooks/useApiError", () => ({
	handleApiError: (error: unknown) => {
		toastMock.error(error instanceof Error ? error.message : String(error));
	},
}));

function server(
	overrides: Partial<AdminYggdrasilSessionForwardServerInfo> = {},
): AdminYggdrasilSessionForwardServerInfo {
	return {
		base_url: "https://remote.example.com/yggdrasil",
		created_at: "2026-06-20T00:00:00Z",
		deletable: true,
		display_name: "Remote Yggdrasil",
		enabled: true,
		id: 2,
		last_checked_at: "2026-06-20T00:10:00Z",
		last_failure_at: null,
		last_failure_message: null,
		last_success_at: "2026-06-20T00:10:00Z",
		local: false,
		priority: 100,
		provider_kind: "remote",
		texture_forward_enabled: true,
		timeout_ms: 1500,
		updated_at: "2026-06-20T00:00:00Z",
		weight: 10,
		...overrides,
	};
}

function serverPage(items: AdminYggdrasilSessionForwardServerInfo[]) {
	return {
		items,
		limit: 20,
		offset: 0,
		total: items.length,
	};
}

function localServer(
	overrides: Partial<AdminYggdrasilSessionForwardServerInfo> = {},
) {
	return server({
		base_url: null,
		deletable: false,
		display_name: "AsterYggdrasil",
		id: 1,
		local: true,
		provider_kind: "local",
		texture_forward_enabled: false,
		...overrides,
	});
}

function remoteServer(
	overrides: Partial<AdminYggdrasilSessionForwardServerInfo> = {},
) {
	return server({
		...overrides,
	});
}

function mojangServer(
	overrides: Partial<AdminYggdrasilSessionForwardServerInfo> = {},
) {
	return remoteServer({
		base_url: "https://sessionserver.mojang.com",
		deletable: false,
		display_name: "Mojang",
		enabled: false,
		id: 3,
		texture_forward_enabled: false,
		...(overrides as Partial<AdminYggdrasilSessionForwardServerInfo>),
	});
}

async function renderPage(initialEntry = "/admin/yggdrasil-forwarding") {
	render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<AdminYggdrasilForwardingPage />
		</MemoryRouter>,
	);
	await screen.findByText("AsterYggdrasil");
}

function topDialog() {
	const dialog = screen
		.getAllByRole("dialog", { hidden: true })
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

function rowForText(text: string) {
	const row = screen.getByText(text).closest("tr");
	expect(row).toBeDefined();
	return row as HTMLElement;
}

describe("AdminYggdrasilForwardingPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		adminYggdrasilSessionForwardServiceMock.list.mockResolvedValue(
			serverPage([localServer(), mojangServer(), remoteServer()]),
		);
		adminYggdrasilSessionForwardServiceMock.create.mockResolvedValue(
			remoteServer({ display_name: "Backup Yggdrasil", id: 3 }),
		);
		adminYggdrasilSessionForwardServiceMock.update.mockImplementation(
			(id: number, patch: Partial<AdminYggdrasilSessionForwardServerInfo>) =>
				Promise.resolve(
					id === 1
						? localServer({ ...patch, id })
						: remoteServer({ ...patch, id }),
				),
		);
		adminYggdrasilSessionForwardServiceMock.delete.mockResolvedValue(undefined);
	});

	it("loads local and remote forwarding servers", async () => {
		await renderPage();

		expect(adminYggdrasilSessionForwardServiceMock.list).toHaveBeenCalledWith({
			limit: 20,
			offset: 0,
			sort_by: "call_order",
		});
		expect(
			screen.getByText("admin.yggdrasilForwarding.sort.call_order"),
		).toBeInTheDocument();
		expect(screen.getByText("AsterYggdrasil")).toBeInTheDocument();
		expect(screen.getByText("Mojang")).toBeInTheDocument();
		expect(
			screen.getByText("admin.yggdrasilForwarding.testing"),
		).toBeInTheDocument();
		expect(screen.getByText("Remote Yggdrasil")).toBeInTheDocument();
		expect(
			within(rowForText("Remote Yggdrasil")).getAllByText(
				"admin.common.enabled",
			).length,
		).toBeGreaterThan(0);
		expect(
			screen.getByText("admin.yggdrasilForwarding.localBaseUrl"),
		).toBeInTheDocument();
		expect(
			within(rowForText("AsterYggdrasil")).getByText("#1"),
		).toBeInTheDocument();
		expect(within(rowForText("Mojang")).getByText("#3")).toBeInTheDocument();
		expect(
			screen.getByText("https://remote.example.com/yggdrasil"),
		).toBeInTheDocument();
		const remoteRow = rowForText("Remote Yggdrasil");
		expect(
			within(remoteRow).getByText("admin.yggdrasilForwarding.priority: 100"),
		).toBeInTheDocument();
		expect(
			within(remoteRow).getByText("admin.yggdrasilForwarding.weight: 10"),
		).toBeInTheDocument();
		expect(
			within(remoteRow).getByText("admin.yggdrasilForwarding.timeout: 1500 ms"),
		).toBeInTheDocument();
	});

	it("loads the selected sort mode from search params", async () => {
		await renderPage("/admin/yggdrasil-forwarding?sort_by=id");

		expect(adminYggdrasilSessionForwardServiceMock.list).toHaveBeenCalledWith({
			limit: 20,
			offset: 0,
			sort_by: "id",
		});
		expect(
			screen.getByText("admin.yggdrasilForwarding.sort.id"),
		).toBeInTheDocument();
		expect(screen.queryByText("id")).not.toBeInTheDocument();
	});

	it("creates a remote forwarding server from the dialog", async () => {
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", {
				name: /admin.yggdrasilForwarding.createAction/,
			}),
		);
		const dialog = topDialog();
		fireEvent.change(
			within(dialog).getByLabelText("admin.yggdrasilForwarding.displayName"),
			{ target: { value: "  Backup Yggdrasil  " } },
		);
		fireEvent.change(
			within(dialog).getByLabelText("admin.yggdrasilForwarding.baseUrl"),
			{ target: { value: "  https://backup.example.com/yggdrasil  " } },
		);
		fireEvent.change(
			within(dialog).getByLabelText("admin.yggdrasilForwarding.priority"),
			{ target: { value: "50" } },
		);
		fireEvent.change(
			within(dialog).getByLabelText("admin.yggdrasilForwarding.weight"),
			{ target: { value: "3" } },
		);
		fireEvent.change(
			within(dialog).getByLabelText("admin.yggdrasilForwarding.timeoutMs"),
			{ target: { value: "2500" } },
		);
		fireEvent.click(
			within(dialog).getByRole("switch", {
				name: "admin.yggdrasilForwarding.textureForward",
			}),
		);
		fireEvent.click(
			within(dialog).getByRole("button", {
				name: /admin.yggdrasilForwarding.createAction/,
			}),
		);

		await waitFor(() => {
			expect(
				adminYggdrasilSessionForwardServiceMock.create,
			).toHaveBeenCalledWith({
				base_url: "https://backup.example.com/yggdrasil",
				display_name: "Backup Yggdrasil",
				enabled: true,
				priority: 50,
				texture_forward_enabled: true,
				timeout_ms: 2500,
				weight: 3,
			});
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.yggdrasilForwarding.createSuccess",
		);
	});

	it("keeps local server URL and texture forwarding locked in the edit dialog", async () => {
		await renderPage();

		fireEvent.click(
			within(rowForText("AsterYggdrasil")).getByRole("button", {
				name: "admin.yggdrasilForwarding.editAction",
			}),
		);
		const dialog = topDialog();

		expect(
			within(dialog).getByLabelText("admin.yggdrasilForwarding.baseUrl"),
		).toBeDisabled();
		expect(
			within(dialog).getByRole("switch", {
				name: "admin.yggdrasilForwarding.textureForward",
			}),
		).toHaveAttribute("aria-disabled", "true");
	});

	it("toggles server enabled state inline", async () => {
		await renderPage();

		fireEvent.click(
			within(rowForText("Remote Yggdrasil")).getByRole("switch", {
				name: "admin.yggdrasilForwarding.enabled",
			}),
		);

		await waitFor(() => {
			expect(
				adminYggdrasilSessionForwardServiceMock.update,
			).toHaveBeenCalledWith(2, { enabled: false });
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.yggdrasilForwarding.updateSuccess",
		);
	});

	it("toggles remote texture forwarding inline", async () => {
		await renderPage();

		fireEvent.click(
			within(rowForText("Remote Yggdrasil")).getByRole("switch", {
				name: "admin.yggdrasilForwarding.textureForward",
			}),
		);

		await waitFor(() => {
			expect(
				adminYggdrasilSessionForwardServiceMock.update,
			).toHaveBeenCalledWith(2, { texture_forward_enabled: false });
		});
	});

	it("prevents deleting the local server and deletes a remote server", async () => {
		await renderPage();

		expect(
			within(rowForText("AsterYggdrasil")).getByRole("button", {
				name: "admin.yggdrasilForwarding.deleteAction",
			}),
		).toBeDisabled();

		fireEvent.click(
			within(rowForText("Remote Yggdrasil")).getByRole("button", {
				name: "admin.yggdrasilForwarding.deleteAction",
			}),
		);
		fireEvent.click(
			within(topDialog()).getByRole("button", { name: "common.delete" }),
		);

		await waitFor(() => {
			expect(
				adminYggdrasilSessionForwardServiceMock.delete,
			).toHaveBeenCalledWith(2);
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.yggdrasilForwarding.deleteSuccess",
		);
	});

	it("keeps dialog content stable while close animations finish", async () => {
		await renderPage();
		vi.useFakeTimers();

		try {
			fireEvent.click(
				within(rowForText("Remote Yggdrasil")).getByRole("button", {
					name: "admin.yggdrasilForwarding.editAction",
				}),
			);
			expect(
				screen.getByText("admin.yggdrasilForwarding.editTitle"),
			).toBeInTheDocument();
			fireEvent.click(
				within(topDialog()).getByRole("button", { name: "common.cancel" }),
			);

			expect(
				screen.getByText("admin.yggdrasilForwarding.editTitle"),
			).toBeInTheDocument();

			await act(async () => {
				vi.advanceTimersByTime(160);
			});

			fireEvent.click(
				within(rowForText("Remote Yggdrasil")).getByRole("button", {
					name: "admin.yggdrasilForwarding.deleteAction",
				}),
			);
			expect(
				screen.getByText(
					'admin.yggdrasilForwarding.deleteDescription {"name":"Remote Yggdrasil"}',
				),
			).toBeInTheDocument();
			fireEvent.click(
				within(topDialog()).getByRole("button", { name: "common.cancel" }),
			);

			expect(
				screen.getByText(
					'admin.yggdrasilForwarding.deleteDescription {"name":"Remote Yggdrasil"}',
				),
			).toBeInTheDocument();
		} finally {
			vi.useRealTimers();
		}
	});
});
