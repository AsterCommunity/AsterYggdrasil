import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import type { AdminUserInfo } from "@/types/api";
import AdminUsersPage from "./AdminUsersPage";

const adminUserServiceMock = vi.hoisted(() => ({
	create: vi.fn(),
	createInvitation: vi.fn(),
	list: vi.fn(),
	revokeSessions: vi.fn(),
}));

vi.mock("@/services/adminService", () => ({
	adminUserService: adminUserServiceMock,
}));

vi.mock("sonner", () => ({
	toast: {
		error: vi.fn(),
		success: vi.fn(),
	},
}));

const createdUser: AdminUserInfo = {
	active_session_count: 0,
	created_at: "2026-06-18T00:00:00.000Z",
	email: "alex@example.com",
	email_verified_at: null,
	id: 7,
	must_change_password: true,
	pending_email: null,
	profile: {
		avatar: { source: "none", url_1024: null, url_512: null, version: 0 },
		display_name: null,
	},
	profile_count: 0,
	role: "user",
	session_version: 1,
	status: "active",
	updated_at: "2026-06-18T00:00:00.000Z",
	username: "alex-1",
};

function renderPage() {
	return render(
		<MemoryRouter initialEntries={["/admin/users"]}>
			<Routes>
				<Route path="/admin/users" element={<AdminUsersPage />} />
				<Route path="/admin/users/:id" element={<div>user detail</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

describe("AdminUsersPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		adminUserServiceMock.list.mockResolvedValue({ items: [], total: 0 });
		adminUserServiceMock.create.mockResolvedValue({
			generated_password: "TempPass-123456789!",
			user: createdUser,
		});
	});

	it("shows the generated temporary password returned by user creation", async () => {
		renderPage();

		fireEvent.click(await screen.findByRole("button", { name: "Add user" }));
		fireEvent.change(screen.getByLabelText(/^Username/), {
			target: { value: "alex-1" },
		});
		fireEvent.change(screen.getByLabelText(/^Email/), {
			target: { value: "alex@example.com" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Create" }));

		await waitFor(() =>
			expect(adminUserServiceMock.create).toHaveBeenCalledWith({
				email: "alex@example.com",
				password: null,
				must_change_password: false,
				role: "user",
				status: "active",
				username: "alex-1",
			}),
		);
		expect(
			await screen.findByText("Temporary password shown once"),
		).toBeInTheDocument();
		expect(screen.getByDisplayValue("TempPass-123456789!")).toBeInTheDocument();
	});
});
