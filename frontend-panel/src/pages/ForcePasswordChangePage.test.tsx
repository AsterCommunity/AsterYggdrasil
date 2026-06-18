import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { AuthUserInfo } from "@/types/api";
import ForcePasswordChangePage from "./ForcePasswordChangePage";

const authServiceMock = vi.hoisted(() => ({
	changePassword: vi.fn(),
	logout: vi.fn(),
	me: vi.fn(),
}));

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

const forcedUser: AuthUserInfo = {
	email: "alex@example.com",
	email_verified: true,
	id: 7,
	must_change_password: true,
	pending_email: null,
	profile: {
		avatar: { source: "none", url_1024: null, url_512: null, version: 0 },
		display_name: "Alex",
	},
	role: "user",
	status: "active",
	username: "alex",
};

function renderPage(initialEntry = "/force-password-change") {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<Routes>
				<Route
					path="/force-password-change"
					element={<ForcePasswordChangePage />}
				/>
				<Route path="/login" element={<div>login route</div>} />
				<Route path="/account" element={<div>account route</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

function setAuthState(user: AuthUserInfo | null) {
	useAuthStore.setState({
		checking: false,
		error: null,
		errorCode: null,
		expiresAt: Date.now() + 60_000,
		isAdmin: user?.role === "admin",
		isAuthenticated: Boolean(user),
		isAuthStale: false,
		user,
	});
}

describe("ForcePasswordChangePage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		useFrontendConfigStore.getState().invalidate();
		authServiceMock.changePassword.mockResolvedValue({
			expires_in: 3600,
			status: "authenticated",
		});
		authServiceMock.logout.mockResolvedValue({ message: "ok" });
		authServiceMock.me.mockResolvedValue({
			...forcedUser,
			must_change_password: false,
		});
	});

	it("redirects unauthenticated users to login", async () => {
		setAuthState(null);

		renderPage();

		expect(await screen.findByText("login route")).toBeInTheDocument();
	});

	it("redirects users who are no longer required to change password", async () => {
		setAuthState({ ...forcedUser, must_change_password: false });

		renderPage();

		expect(await screen.findByText("account route")).toBeInTheDocument();
	});

	it("validates fields while typing and disables submit until valid", async () => {
		setAuthState(forcedUser);
		renderPage();

		const submit = await screen.findByRole("button", {
			name: "Change password",
		});
		expect(submit).toBeDisabled();

		fireEvent.change(screen.getByLabelText("Current password"), {
			target: { value: "old-password" },
		});
		fireEvent.change(screen.getByLabelText("New password"), {
			target: { value: "short" },
		});
		expect(
			screen.getByText("Password must be 8-128 characters."),
		).toBeInTheDocument();
		expect(submit).toBeDisabled();

		fireEvent.change(screen.getByLabelText("New password"), {
			target: { value: "old-password" },
		});
		expect(
			screen.getByText("New password cannot match the current password."),
		).toBeInTheDocument();

		fireEvent.change(screen.getByLabelText("New password"), {
			target: { value: "new-password" },
		});
		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "different" },
		});
		expect(screen.getByText("Passwords do not match.")).toBeInTheDocument();

		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "new-password" },
		});
		expect(submit).not.toBeDisabled();
	});

	it("changes password and navigates to the account page", async () => {
		setAuthState(forcedUser);
		renderPage();

		fireEvent.change(await screen.findByLabelText("Current password"), {
			target: { value: "old-password" },
		});
		fireEvent.change(screen.getByLabelText("New password"), {
			target: { value: "new-password" },
		});
		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "new-password" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Change password" }));

		await waitFor(() =>
			expect(authServiceMock.changePassword).toHaveBeenCalledWith({
				current_password: "old-password",
				new_password: "new-password",
			}),
		);
		expect(authServiceMock.me).toHaveBeenCalled();
		expect(toastMock.success).toHaveBeenCalledWith("Password changed");
		expect(await screen.findByText("account route")).toBeInTheDocument();
	});

	it("logs out from the forced password change page", async () => {
		setAuthState(forcedUser);
		renderPage();

		fireEvent.click(await screen.findByRole("button", { name: "Logout" }));

		await waitFor(() => expect(authServiceMock.logout).toHaveBeenCalled());
		expect(await screen.findByText("login route")).toBeInTheDocument();
	});
});
