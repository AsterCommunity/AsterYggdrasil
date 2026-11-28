import { fireEvent, render, screen } from "@testing-library/react";
import {
	createMemoryRouter,
	MemoryRouter,
	Outlet,
	Route,
	RouterProvider,
	Routes,
} from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { AdminOnlyGate } from "@/routes/guards/AdminOnlyGate";
import { AuthenticatedGate } from "@/routes/guards/AuthenticatedGate";
import { GuestOnlyGate } from "@/routes/guards/GuestOnlyGate";
import { InitializedGate } from "@/routes/guards/InitGate";
import { useInitStatusStore } from "@/stores/initStatusStore";

const authState = vi.hoisted(() => ({
	checking: false,
	canAccessAdminShell: false,
	errorCode: null as string | null,
	hasPermissionId: vi.fn(),
	hydrate: vi.fn(),
	isAdmin: false,
	isAuthenticated: false,
	isAuthStale: false,
	bootOffline: false,
	operatorScopes: [] as string[],
	user: null as { must_change_password?: boolean; role?: string } | null,
}));

const authServiceMock = vi.hoisted(() => ({
	check: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
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

function renderAdminRoute(initialEntry = "/admin") {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<Routes>
				<Route path="/admin" element={<AdminOnlyGate />}>
					<Route index element={<div>admin route</div>} />
					<Route path="users" element={<div>admin users route</div>} />
					<Route
						path="minecraft-profiles/:uuid"
						element={<div>admin profile route</div>}
					/>
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

function resetAuthState() {
	authState.checking = false;
	authState.canAccessAdminShell = false;
	authState.errorCode = null;
	authState.hasPermissionId.mockReset();
	authState.hasPermissionId.mockImplementation((scope: string) =>
		authState.operatorScopes.includes(scope),
	);
	authState.hydrate.mockReset();
	authState.hydrate.mockResolvedValue(undefined);
	authState.isAdmin = false;
	authState.isAuthenticated = false;
	authState.isAuthStale = false;
	authState.bootOffline = false;
	authState.operatorScopes = [];
	authState.user = null;
}

function resetInitStatus() {
	authServiceMock.check.mockReset();
	authServiceMock.check.mockResolvedValue({ initialized: true });
	useInitStatusStore.getState().reset();
}

describe("route guards", () => {
	beforeEach(() => {
		resetAuthState();
		resetInitStatus();
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
		authState.canAccessAdminShell = true;
		authState.isAuthenticated = true;
		authState.user = { must_change_password: true, role: "admin" };

		renderAdminRoute();

		expect(await screen.findByText("force route")).toBeInTheDocument();
		expect(screen.queryByText("admin route")).not.toBeInTheDocument();
	});

	it("allows operator admin routes when the matching scope is present", async () => {
		authState.canAccessAdminShell = true;
		authState.isAuthenticated = true;
		authState.operatorScopes = [
			"com.astercosm.yggdrasil.permission.admin.overview",
		];
		authState.user = { must_change_password: false, role: "operator" };

		renderAdminRoute();

		expect(await screen.findByText("admin route")).toBeInTheDocument();
	});

	it("blocks operator admin routes when the matching scope is missing", async () => {
		authState.canAccessAdminShell = true;
		authState.isAuthenticated = true;
		authState.operatorScopes = [
			"com.astercosm.yggdrasil.permission.admin.texture_library",
		];
		authState.user = { must_change_password: false, role: "operator" };

		renderAdminRoute("/admin/users");

		expect(
			await screen.findByText("Admin access required"),
		).toBeInTheDocument();
		expect(screen.queryByText("admin users route")).not.toBeInTheDocument();
	});

	it("allows operator minecraft profile detail routes with profiles scope", async () => {
		authState.canAccessAdminShell = true;
		authState.isAuthenticated = true;
		authState.operatorScopes = [
			"com.astercosm.yggdrasil.permission.admin.profiles",
		];
		authState.user = { must_change_password: false, role: "operator" };

		renderAdminRoute("/admin/minecraft-profiles/profile-uuid");

		expect(await screen.findByText("admin profile route")).toBeInTheDocument();
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

describe("route guards: init gate", () => {
	beforeEach(() => {
		resetAuthState();
		resetInitStatus();
	});

	it("shows setup required state before initialization", async () => {
		authServiceMock.check.mockResolvedValue({ initialized: false });
		const router = createMemoryRouter(
			[
				{
					element: <InitializedGate />,
					children: [
						{
							path: "/register",
							element: <div>register-route</div>,
						},
					],
				},
				{ path: "/init", element: <div>init-route</div> },
			],
			{ initialEntries: ["/register"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("Setup required")).toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Start setup" })).toHaveAttribute(
			"href",
			"/init",
		);
		expect(screen.queryByText("register-route")).not.toBeInTheDocument();
		expect(screen.queryByText("init-route")).not.toBeInTheDocument();
	});
});

describe("route guards: authenticated gate", () => {
	beforeEach(() => {
		resetAuthState();
		resetInitStatus();
	});

	it("shows login required state for protected routes when unauthenticated", async () => {
		const router = createMemoryRouter(
			[
				{
					element: <AuthenticatedGate />,
					children: [
						{
							path: "/account",
							element: <Outlet />,
						},
					],
				},
				{ path: "/login", element: <div>login-route</div> },
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("Login required")).toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Go to login" })).toHaveAttribute(
			"href",
			"/login",
		);
		expect(screen.queryByText("login-route")).not.toBeInTheDocument();
	});

	it("shows network error state for protected routes when auth hydration cannot reach the server", async () => {
		authState.errorCode = "network_error";
		// The gate renders from isAuthStale/errorCode, not from the hydrate
		// result; the mock resolves because `void hydrate()` has no catch, and
		// a rejected mock would surface as an unhandled promise rejection.
		authState.hydrate.mockResolvedValue(undefined);
		authState.isAuthStale = true;
		const router = createMemoryRouter(
			[
				{
					element: <AuthenticatedGate />,
					children: [
						{
							path: "/account",
							element: <Outlet />,
						},
					],
				},
				{ path: "/login", element: <div>login-route</div> },
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("Connection lost")).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Retry connection" }),
		).toBeInTheDocument();
		expect(screen.queryByText("Login required")).not.toBeInTheDocument();
		expect(screen.queryByText("login-route")).not.toBeInTheDocument();
	});

	it("shows the unreachable-server state when boot fails with no cached session", async () => {
		authState.bootOffline = true;
		authState.hydrate.mockResolvedValue(undefined);
		const router = createMemoryRouter(
			[
				{
					element: <AuthenticatedGate />,
					children: [
						{
							path: "/account",
							element: <Outlet />,
						},
					],
				},
				{ path: "/login", element: <div>login-route</div> },
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(
			await screen.findByText("Cannot reach the server"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "Retry connection" }),
		).toBeInTheDocument();
		expect(screen.queryByText("Connection lost")).not.toBeInTheDocument();
		expect(screen.queryByText("Login required")).not.toBeInTheDocument();

		fireEvent.click(screen.getByRole("button", { name: "Retry connection" }));
		expect(authState.hydrate).toHaveBeenCalled();
	});
});

describe("route guards: guest-only gate", () => {
	beforeEach(() => {
		resetAuthState();
		resetInitStatus();
	});

	it("shows already signed in state on guest-only routes", async () => {
		authState.isAuthenticated = true;
		authState.user = { must_change_password: false, role: "user" };

		const router = createMemoryRouter(
			[
				{
					path: "/login",
					element: <GuestOnlyGate />,
					children: [{ index: true, element: <div>login-route</div> }],
				},
				{
					path: "/register",
					element: <GuestOnlyGate />,
					children: [{ index: true, element: <div>register-route</div> }],
				},
				{ path: "/account", element: <div>account-route</div> },
			],
			{ initialEntries: ["/register"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("Already signed in")).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Open workbench" }),
		).toHaveAttribute("href", "/account");
		expect(screen.queryByText("account-route")).not.toBeInTheDocument();
		expect(screen.queryByText("login-route")).not.toBeInTheDocument();
		expect(screen.queryByText("register-route")).not.toBeInTheDocument();
	});

	it("does not block the login route with an auth check for anonymous visitors", async () => {
		authState.checking = true;

		const router = createMemoryRouter(
			[
				{
					path: "/login",
					element: <GuestOnlyGate />,
					children: [{ index: true, element: <div>login-route</div> }],
				},
			],
			{ initialEntries: ["/login"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("login-route")).toBeInTheDocument();
		expect(authState.hydrate).not.toHaveBeenCalled();
	});
});
