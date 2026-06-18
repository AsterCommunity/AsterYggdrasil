import { render, screen } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import InitPage from "@/pages/InitPage";
import { authRoutes } from "@/routes/authRoutes";
import { InitializedGate, UninitializedGate } from "@/routes/guards/InitGate";
import { publicRoutes } from "@/routes/publicRoutes";
import { publicPaths } from "@/routes/routePaths";
import { publicElement } from "@/routes/routeSuspense";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import { useInitStatusStore } from "@/stores/initStatusStore";

const authServiceMock = vi.hoisted(() => ({
	check: vi.fn(),
	me: vi.fn(),
}));

const externalAuthServiceMock = vi.hoisted(() => ({
	listPublic: vi.fn(),
	startAuthAlias: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

vi.mock("@/services/externalAuthService", () => ({
	externalAuthService: externalAuthServiceMock,
}));

vi.mock("@/pages/InitPage", () => ({
	default: () => <div data-testid="init-page">init route</div>,
}));

vi.mock("@/pages/LoginPage", () => ({
	default: () => <div data-testid="login-page">login/register route</div>,
}));

vi.mock("@/pages/InvitePage", () => ({
	default: () => <div data-testid="invite-page">invite route</div>,
}));

vi.mock("@/pages/ResetPasswordPage", () => ({
	default: () => <div data-testid="reset-password-page">reset route</div>,
}));

vi.mock("@/pages/PublicConnectPage", () => ({
	default: () => (
		<div data-testid="public-connect-page">public connect route</div>
	),
}));

vi.mock("@/pages/TermsPage", () => ({
	default: () => <div data-testid="terms-page">terms route</div>,
}));

vi.mock("@/pages/PrivacyPage", () => ({
	default: () => <div data-testid="privacy-page">privacy route</div>,
}));

const initializedPublicRouteObjects = [...publicRoutes, ...authRoutes];

function renderPublicRoute(path: string) {
	const router = createMemoryRouter(
		[
			{
				path: publicPaths.init,
				element: <UninitializedGate />,
				children: [{ index: true, element: publicElement(<InitPage />) }],
			},
			{
				element: <InitializedGate />,
				children: initializedPublicRouteObjects,
			},
		],
		{ initialEntries: [path] },
	);

	return render(<RouterProvider router={router} />);
}

function setInitializedState(initialized: boolean) {
	useInitStatusStore.setState({
		checking: false,
		error: null,
		initialized,
	});
	authServiceMock.check.mockResolvedValue({ initialized });
}

function setAuthenticatedState() {
	const user = {
		id: 7,
		username: "alex",
		email: "alex@example.com",
		role: "user",
		status: "active",
	} as const;

	useAuthStore.setState({
		checking: false,
		error: null,
		expiresAt: Date.now() + 60_000,
		isAdmin: false,
		isAuthStale: false,
		isAuthenticated: true,
		user,
	});
	authServiceMock.me.mockResolvedValue(user);
}

describe("fixed public routes", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		useFrontendConfigStore.getState().invalidate();
		useInitStatusStore.getState().reset();
		externalAuthServiceMock.listPublic.mockResolvedValue([]);
		externalAuthServiceMock.startAuthAlias.mockResolvedValue({
			authorization_url: "https://example.com/oauth",
		});
		setInitializedState(true);
	});

	it("keeps the fixed public route paths explicit", () => {
		expect(publicRoutes.map((route) => route.path)).toEqual([
			publicPaths.home,
			publicPaths.tos,
			publicPaths.privacy,
		]);
		expect(authRoutes.map((route) => route.path)).toEqual([
			publicPaths.login,
			publicPaths.register,
			publicPaths.invite,
			publicPaths.resetPassword,
		]);
		expect(publicRoutes.map((route) => route.path)).not.toContain("/connect");
	});

	it.each([
		[publicPaths.home, "public-connect-page"],
		[publicPaths.tos, "terms-page"],
		[publicPaths.privacy, "privacy-page"],
		[publicPaths.login, "login-page"],
		[publicPaths.register, "login-page"],
		["/invite/test-token", "invite-page"],
		[publicPaths.resetPassword, "reset-password-page"],
	])("renders initialized public route %s", async (path, testId) => {
		renderPublicRoute(path);

		expect(await screen.findByTestId(testId)).toBeInTheDocument();
		expect(screen.queryByText("Setup required")).not.toBeInTheDocument();
	});

	it("renders init only when the instance is not initialized", async () => {
		setInitializedState(false);

		renderPublicRoute(publicPaths.init);

		expect(await screen.findByTestId("init-page")).toBeInTheDocument();
		expect(
			screen.queryByText("Setup already complete"),
		).not.toBeInTheDocument();
	});

	it.each([
		publicPaths.home,
		publicPaths.tos,
		publicPaths.privacy,
		publicPaths.login,
		publicPaths.register,
		publicPaths.resetPassword,
	])("blocks %s before initialization without redirecting", async (path) => {
		setInitializedState(false);

		renderPublicRoute(path);

		expect(await screen.findByText("Setup required")).toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Start setup" })).toHaveAttribute(
			"href",
			publicPaths.init,
		);
		expect(screen.queryByTestId("public-connect-page")).not.toBeInTheDocument();
		expect(screen.queryByTestId("terms-page")).not.toBeInTheDocument();
		expect(screen.queryByTestId("privacy-page")).not.toBeInTheDocument();
		expect(screen.queryByTestId("login-page")).not.toBeInTheDocument();
	});

	it("blocks init after initialization without redirecting", async () => {
		setInitializedState(true);

		renderPublicRoute(publicPaths.init);

		expect(
			await screen.findByText("Setup already complete"),
		).toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Go to login" })).toHaveAttribute(
			"href",
			publicPaths.login,
		);
		expect(screen.queryByTestId("init-page")).not.toBeInTheDocument();
	});

	it.each([
		publicPaths.login,
		publicPaths.register,
		publicPaths.resetPassword,
	])("blocks guest-only route %s for authenticated users without redirecting", async (path) => {
		setAuthenticatedState();

		renderPublicRoute(path);

		expect(await screen.findByText("Already signed in")).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Open workbench" }),
		).toHaveAttribute("href", "/account");
		expect(screen.queryByTestId("login-page")).not.toBeInTheDocument();
	});

	it("keeps the root public connect page reachable for authenticated users", async () => {
		setAuthenticatedState();

		renderPublicRoute(publicPaths.home);

		expect(
			await screen.findByTestId("public-connect-page"),
		).toBeInTheDocument();
		expect(screen.queryByText("Already signed in")).not.toBeInTheDocument();
	});

	it("treats stale auth as an auth boundary on login/register", async () => {
		authServiceMock.me.mockImplementation(() => new Promise(() => undefined));
		useAuthStore.setState({
			checking: true,
			error: null,
			expiresAt: Date.now() + 60_000,
			isAdmin: false,
			isAuthStale: true,
			isAuthenticated: false,
			user: null,
		});

		renderPublicRoute(publicPaths.login);

		expect(await screen.findByText("Loading")).toBeInTheDocument();
		expect(screen.queryByTestId("login-page")).not.toBeInTheDocument();
	});
});
