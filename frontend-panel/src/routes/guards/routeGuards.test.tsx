import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { AdminOnlyGate } from "@/routes/guards/AdminOnlyGate";
import { AuthenticatedGate } from "@/routes/guards/AuthenticatedGate";
import { GuestOnlyGate } from "@/routes/guards/GuestOnlyGate";

const authState = vi.hoisted(() => ({
	checking: false,
	errorCode: null as string | null,
	hydrate: vi.fn(),
	isAdmin: false,
	isAuthenticated: false,
	isAuthStale: false,
	user: null as { must_change_password?: boolean; role?: string } | null,
}));

vi.mock("@/stores/authStore", () => ({
	useAuthStore: (selector: (state: typeof authState) => unknown) =>
		selector(authState),
}));

function renderAuthenticatedRoute(initialEntry: string) {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<Routes>
				<Route element={<AuthenticatedGate />}>
					<Route path="/account" element={<div>account route</div>} />
					<Route
						path="/force-password-change"
						element={<div>force route</div>}
					/>
				</Route>
				<Route path="/login" element={<div>login route</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

function renderAdminRoute() {
	return render(
		<MemoryRouter initialEntries={["/admin"]}>
			<Routes>
				<Route path="/admin" element={<AdminOnlyGate />}>
					<Route index element={<div>admin route</div>} />
				</Route>
				<Route path="/force-password-change" element={<div>force route</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

function renderGuestRoute() {
	return render(
		<MemoryRouter initialEntries={["/login"]}>
			<Routes>
				<Route path="/login" element={<GuestOnlyGate />}>
					<Route index element={<div>login route</div>} />
				</Route>
			</Routes>
		</MemoryRouter>,
	);
}

describe("route guards", () => {
	beforeEach(() => {
		authState.checking = false;
		authState.errorCode = null;
		authState.hydrate.mockReset();
		authState.hydrate.mockResolvedValue(undefined);
		authState.isAdmin = false;
		authState.isAuthenticated = false;
		authState.isAuthStale = false;
		authState.user = null;
	});

	it("renders protected content for authenticated users without forced password change", async () => {
		authState.isAuthenticated = true;
		authState.user = { must_change_password: false, role: "user" };

		renderAuthenticatedRoute("/account");

		expect(await screen.findByText("account route")).toBeInTheDocument();
	});

	it("redirects protected routes to force password change when required", async () => {
		authState.isAuthenticated = true;
		authState.user = { must_change_password: true, role: "user" };

		renderAuthenticatedRoute("/account");

		expect(await screen.findByText("force route")).toBeInTheDocument();
		expect(screen.queryByText("account route")).not.toBeInTheDocument();
	});

	it("allows the forced password change route itself", async () => {
		authState.isAuthenticated = true;
		authState.user = { must_change_password: true, role: "user" };

		renderAuthenticatedRoute("/force-password-change");

		expect(await screen.findByText("force route")).toBeInTheDocument();
	});

	it("redirects admin routes while forced password change is required", async () => {
		authState.isAdmin = true;
		authState.isAuthenticated = true;
		authState.user = { must_change_password: true, role: "admin" };

		renderAdminRoute();

		expect(await screen.findByText("force route")).toBeInTheDocument();
		expect(screen.queryByText("admin route")).not.toBeInTheDocument();
	});

	it("points authenticated guest-only pages to password change when required", async () => {
		authState.isAuthenticated = true;
		authState.user = { must_change_password: true, role: "user" };

		renderGuestRoute();

		expect(
			await screen.findByText("Password change required"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Change password" }),
		).toHaveAttribute("href", "/force-password-change");
		expect(screen.queryByText("login route")).not.toBeInTheDocument();
	});
});
