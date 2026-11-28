import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useNavigate } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { createAdminUser, createAdminUserPage } from "@/test/fixtures";
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

const createdUser = createAdminUser({
	id: 7,
	username: "alex-1",
	must_change_password: true,
	email_verified_at: null,
});

// Simulates an in-app link or back/forward navigation: the URL keyword
// changes without the page authoring it, which is exactly the case the
// keyword draft state must lose to.
function ExternalNavigateProbe() {
	const navigate = useNavigate();
	return (
		<button
			type="button"
			onClick={() => navigate("/admin/users?keyword=external")}
		>
			navigate external
		</button>
	);
}

function renderPage(options?: {
	initialEntries?: string[];
	withExternalNavigateProbe?: boolean;
}) {
	const { initialEntries = ["/admin/users"], withExternalNavigateProbe } =
		options ?? {};
	return render(
		<MemoryRouter initialEntries={initialEntries}>
			{withExternalNavigateProbe ? <ExternalNavigateProbe /> : null}
			<Routes>
				<Route path="/admin/users" element={<AdminUsersPage />} />
				<Route
					path="/admin/users/invitations"
					element={<div>invitations</div>}
				/>
				<Route path="/admin/users/:id" element={<div>user detail</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

describe("AdminUsersPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		adminUserServiceMock.list.mockResolvedValue(createAdminUserPage());
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

	it("shows invitation records as a users section link", async () => {
		renderPage();

		expect(
			await screen.findByRole("link", { name: /Invitation records/ }),
		).toHaveAttribute("href", "/admin/users/invitations");
	});

	it("adopts a restored URL keyword as the filter on mount", async () => {
		renderPage({ initialEntries: ["/admin/users?keyword=deep-link"] });

		// The filter panel starts collapsed; the input mounts on expand.
		// A restored keyword shows as a count badge in the toggle name.
		fireEvent.click(await screen.findByRole("button", { name: /^Filters/ }));
		const input = await screen.findByPlaceholderText(
			"Search username or email",
		);
		expect(input).toHaveValue("deep-link");
		await waitFor(() => {
			expect(adminUserServiceMock.list).toHaveBeenCalledWith(
				expect.objectContaining({ keyword: "deep-link" }),
			);
		});
	});

	it("commits typed keywords to the URL after the debounce", async () => {
		renderPage();

		fireEvent.click(await screen.findByRole("button", { name: "Filters" }));
		const input = await screen.findByPlaceholderText(
			"Search username or email",
		);
		fireEvent.change(input, { target: { value: "typed" } });

		await waitFor(() => {
			expect(adminUserServiceMock.list).toHaveBeenCalledWith(
				expect.objectContaining({ keyword: "typed" }),
			);
		});
		expect(input).toHaveValue("typed");
	});

	it("retires the user draft when navigation changes the URL keyword", async () => {
		renderPage({ withExternalNavigateProbe: true });

		fireEvent.click(await screen.findByRole("button", { name: "Filters" }));
		const input = await screen.findByPlaceholderText(
			"Search username or email",
		);
		fireEvent.change(input, { target: { value: "stale" } });
		await waitFor(() => {
			expect(adminUserServiceMock.list).toHaveBeenCalledWith(
				expect.objectContaining({ keyword: "stale" }),
			);
		});

		fireEvent.click(screen.getByRole("button", { name: "navigate external" }));

		await waitFor(() => {
			expect(adminUserServiceMock.list).toHaveBeenCalledWith(
				expect.objectContaining({ keyword: "external" }),
			);
		});
		expect(input).toHaveValue("external");
	});

	it("lets external navigation win over a debounce that has not settled yet", async () => {
		renderPage({ withExternalNavigateProbe: true });

		fireEvent.click(await screen.findByRole("button", { name: "Filters" }));
		const input = await screen.findByPlaceholderText(
			"Search username or email",
		);
		fireEvent.change(input, { target: { value: "stale" } });
		// Navigate before the 250ms debounce settles: the stale pre-
		// navigation debounce value must never reach the URL.
		fireEvent.click(screen.getByRole("button", { name: "navigate external" }));

		await waitFor(() => {
			expect(adminUserServiceMock.list).toHaveBeenCalledWith(
				expect.objectContaining({ keyword: "external" }),
			);
		});
		expect(input).toHaveValue("external");
		// The only empty-keyword load is the initial mount; the unsettled
		// "stale" debounce value must never reach the list.
		const keywordCalls = adminUserServiceMock.list.mock.calls.map(
			(call) => (call[0] as { keyword: string }).keyword,
		);
		expect(keywordCalls.filter((k) => k === "" || k === "stale")).toHaveLength(
			1,
		);
	});
});
