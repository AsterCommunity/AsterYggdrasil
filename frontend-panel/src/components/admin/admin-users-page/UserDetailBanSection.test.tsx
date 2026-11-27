import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { UserDetailBanSection } from "@/components/admin/admin-users-page/UserDetailBanSection";
import { i18next } from "@/i18n";
import {
	createUserBan,
	createUserBanEvent,
	createUserBanPage,
} from "@/test/fixtures";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const adminUserServiceMock = vi.hoisted(() => ({
	createBan: vi.fn(),
	listBanEvents: vi.fn(),
	listBans: vi.fn(),
	revokeBan: vi.fn(),
	updateBan: vi.fn(),
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

vi.mock("@/services/adminService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/adminService")>();
	return {
		...actual,
		adminUserService: {
			...actual.adminUserService,
			...adminUserServiceMock,
		},
	};
});

vi.mock("@/services/permissionService", () => ({
	permissionService: {
		list: vi.fn().mockImplementation(async () => {
			const { REGISTRY_PERMISSIONS } = await import("@/test/permissions");
			return { permissions: REGISTRY_PERMISSIONS };
		}),
	},
}));

function renderSection() {
	render(<UserDetailBanSection userId={7} />);
}

function topDialog() {
	const dialog = screen
		.getAllByRole("dialog", { hidden: true })
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

describe("UserDetailBanSection", () => {
	beforeEach(async () => {
		vi.clearAllMocks();
		await i18next.changeLanguage("en-US");
		adminUserServiceMock.listBans.mockResolvedValue(createUserBanPage());
		adminUserServiceMock.createBan.mockResolvedValue(createUserBan({ id: 12 }));
		adminUserServiceMock.updateBan.mockResolvedValue(
			createUserBan({ admin_note: "updated", id: 11, reason: "profile abuse" }),
		);
		adminUserServiceMock.revokeBan.mockResolvedValue(
			createUserBan({
				effective: false,
				effective_status: "revoked",
				revoke_note: "appeal accepted",
				revoked_at: "2026-06-18T01:00:00Z",
				status: "revoked",
			}),
		);
		adminUserServiceMock.listBanEvents.mockResolvedValue([
			createUserBanEvent({
				event_type: "revoked",
				id: 22,
				next_status: "revoked",
				note: "appeal accepted",
				previous_status: "active",
			}),
			createUserBanEvent(),
		]);
	});

	it("loads user bans and renders active status, scope, and timing", async () => {
		renderSection();

		expect(
			await screen.findByText("Texture upload and wardrobe"),
		).toBeInTheDocument();
		expect(adminUserServiceMock.listBans).toHaveBeenCalledWith({
			limit: 50,
			user_id: 7,
		});
		expect(screen.getByText("upload abuse")).toBeInTheDocument();
		expect(screen.getByText("1 active")).toBeInTheDocument();
		expect(screen.getByText("visible reason")).toBeInTheDocument();
	});

	it("validates the create dialog and sends trimmed nullable fields", async () => {
		adminUserServiceMock.listBans
			.mockResolvedValueOnce(createUserBanPage([]))
			.mockResolvedValueOnce(createUserBanPage([createUserBan({ id: 12 })]));
		renderSection();
		await screen.findByText("This user has no ban records.");

		fireEvent.click(screen.getByRole("button", { name: "Add ban" }));
		const dialog = topDialog();
		fireEvent.click(within(dialog).getByRole("button", { name: "Add ban" }));
		expect(
			await within(dialog).findByText("Enter a ban reason."),
		).toBeInTheDocument();

		fireEvent.change(within(dialog).getByLabelText("Reason"), {
			target: { value: "  profile abuse  " },
		});
		fireEvent.change(within(dialog).getByLabelText("User-facing reason"), {
			target: { value: "  visible  " },
		});
		fireEvent.change(within(dialog).getByLabelText("Admin note"), {
			target: { value: "   " },
		});
		fireEvent.change(within(dialog).getByLabelText("Starts"), {
			target: { value: "2026-06-18T09:30" },
		});
		fireEvent.click(within(dialog).getByRole("button", { name: "Add ban" }));

		await waitFor(() => {
			expect(adminUserServiceMock.createBan).toHaveBeenCalledWith(7, {
				admin_note: null,
				expires_at: null,
				public_reason: "visible",
				reason: "profile abuse",
				scopes: ["com.astercosm.yggdrasil.capability.yggdrasil.access"],
				starts_at: new Date("2026-06-18T09:30").toISOString(),
			});
		});
		expect(toastMock.success).toHaveBeenCalledWith("Ban created");
	});

	it("unchecks an extra capability before submitting the ban", async () => {
		adminUserServiceMock.listBans
			.mockResolvedValueOnce(createUserBanPage([]))
			.mockResolvedValueOnce(createUserBanPage([]));
		renderSection();
		await screen.findByText("This user has no ban records.");

		fireEvent.click(screen.getByRole("button", { name: "Add ban" }));
		const dialog = topDialog();

		const joinCheckbox = within(dialog).getByRole("checkbox", {
			name: "Yggdrasil server join",
		});
		fireEvent.click(joinCheckbox);
		expect(joinCheckbox).toBeChecked();
		// Unchecking exercises the removal path; the default capability stays.
		fireEvent.click(joinCheckbox);
		expect(joinCheckbox).not.toBeChecked();

		fireEvent.change(within(dialog).getByLabelText("Reason"), {
			target: { value: "scope toggle" },
		});
		fireEvent.click(within(dialog).getByRole("button", { name: "Add ban" }));

		await waitFor(() => {
			expect(adminUserServiceMock.createBan).toHaveBeenCalledWith(7, {
				admin_note: null,
				expires_at: null,
				public_reason: null,
				reason: "scope toggle",
				scopes: ["com.astercosm.yggdrasil.capability.yggdrasil.access"],
				starts_at: null,
			});
		});
	});

	it("edits active bans with nullable clear fields", async () => {
		adminUserServiceMock.listBans
			.mockResolvedValueOnce(createUserBanPage())
			.mockResolvedValueOnce(
				createUserBanPage([createUserBan({ reason: "profile abuse" })]),
			);
		renderSection();
		await screen.findByText("Texture upload and wardrobe");

		fireEvent.click(screen.getByRole("button", { name: "Edit" }));
		const dialog = topDialog();
		fireEvent.change(within(dialog).getByLabelText("Reason"), {
			target: { value: "  profile abuse  " },
		});
		fireEvent.change(within(dialog).getByLabelText("User-facing reason"), {
			target: { value: "   " },
		});
		fireEvent.change(within(dialog).getByLabelText("Admin note"), {
			target: { value: "  updated  " },
		});
		fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

		await waitFor(() => {
			expect(adminUserServiceMock.updateBan).toHaveBeenCalledWith(11, {
				admin_note: "updated",
				expires_at: null,
				public_reason: null,
				reason: "profile abuse",
				scopes: ["com.astercosm.yggdrasil.capability.texture.upload"],
				starts_at: "2026-06-18T00:00:00.000Z",
			});
		});
		expect(toastMock.success).toHaveBeenCalledWith("Ban updated");
	});

	it("revokes active bans and opens event history", async () => {
		adminUserServiceMock.listBans
			.mockResolvedValueOnce(createUserBanPage())
			.mockResolvedValueOnce(
				createUserBanPage([
					createUserBan({
						effective: false,
						effective_status: "revoked",
						status: "revoked",
					}),
				]),
			);
		renderSection();
		await screen.findByText("Texture upload and wardrobe");

		fireEvent.click(screen.getByRole("button", { name: "Revoke" }));
		let dialog = topDialog();
		fireEvent.change(within(dialog).getByLabelText("Revocation note"), {
			target: { value: "  appeal accepted  " },
		});
		fireEvent.click(within(dialog).getByRole("button", { name: "Revoke" }));

		await waitFor(() => {
			expect(adminUserServiceMock.revokeBan).toHaveBeenCalledWith(11, {
				revoke_note: "appeal accepted",
			});
		});
		expect(toastMock.success).toHaveBeenCalledWith("Ban revoked");

		fireEvent.click(screen.getByRole("button", { name: "History" }));
		dialog = topDialog();
		expect(adminUserServiceMock.listBanEvents).toHaveBeenCalledWith(11);
		expect(
			await within(dialog).findByText("appeal accepted"),
		).toBeInTheDocument();
		expect(within(dialog).getByText("Created")).toBeInTheDocument();
		expect(within(dialog).getAllByText("Actor #1")).toHaveLength(2);
	});

	it("disables edit and revoke for expired or revoked bans", async () => {
		adminUserServiceMock.listBans.mockResolvedValueOnce(
			createUserBanPage([
				createUserBan({
					effective: false,
					effective_status: "expired",
					expires_at: "2026-06-17T00:00:00Z",
				}),
			]),
		);
		renderSection();

		await screen.findByText("Expired");
		expect(screen.getByRole("button", { name: "Edit" })).toBeDisabled();
		expect(screen.getByRole("button", { name: "Revoke" })).toBeDisabled();
		expect(screen.getByRole("button", { name: "History" })).not.toBeDisabled();
	});
});
