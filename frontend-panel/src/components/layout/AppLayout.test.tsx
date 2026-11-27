import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { createMemoryRouter, Link, RouterProvider } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppLayout } from "@/components/layout/AppLayout";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import { useInitStatusStore } from "@/stores/initStatusStore";
import { setAuthenticatedUser } from "@/test/authState";

const authServiceMock = vi.hoisted(() => ({
	logout: vi.fn(),
	me: vi.fn(),
}));

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
	warning: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

const DESKTOP_SIDEBAR_STORAGE_KEY = "asteryggdrasil-desktop-sidebar-expanded";

function matchMediaResult(query: string, matches: boolean) {
	return {
		matches,
		media: query,
		onchange: null,
		addEventListener: vi.fn(),
		removeEventListener: vi.fn(),
		addListener: vi.fn(),
		removeListener: vi.fn(),
		dispatchEvent: vi.fn(),
	};
}

function mockXlViewport(matches: boolean) {
	vi.mocked(window.matchMedia).mockImplementation((query: string) =>
		matchMediaResult(query, query === "(min-width: 1280px)" && matches),
	);
}

describe("AppLayout", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		useFrontendConfigStore.getState().invalidate();
		useInitStatusStore.getState().reset();
		authServiceMock.me.mockRejectedValue(new Error("unauthenticated"));
		authServiceMock.logout.mockResolvedValue(undefined);
	});

	it("renders account and admin sections in the shared app sidebar", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		localStorage.setItem(DESKTOP_SIDEBAR_STORAGE_KEY, "true");

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{
							path: "/admin/settings",
							element: <div>admin-settings-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/admin/settings"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("admin-settings-route")).toBeInTheDocument();
		const sidebarNav = document.querySelector(
			'[data-slot="shell-desktop-sidebar"] nav',
		);
		expect(sidebarNav).toBeTruthy();
		const sidebarLinks = within(sidebarNav as HTMLElement).getAllByRole("link");
		expect(sidebarLinks.map((link) => link.getAttribute("href"))).toEqual([
			"/account",
			"/account/profiles",
			"/account/wardrobe",
			"/textures",
			"/account/audit",
			"/account/settings",
			"/admin",
			"/admin/users",
			"/admin/external-auth",
			"/admin/yggdrasil-forwarding",
			"/admin/texture-library",
			"/admin/audit",
			"/admin/tasks",
			"/admin/settings",
			"/admin/about",
		]);
		for (const link of sidebarLinks) {
			expect(link.className).not.toContain("translate-y");
		}
		expect(
			within(sidebarNav as HTMLElement).getByText("My space"),
		).toBeVisible();
		expect(
			within(sidebarNav as HTMLElement).getByText("Administration"),
		).toBeVisible();
	});

	it("shows and clears the external login success redirect toast", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout scope="account" />,
					children: [
						{
							path: "/account",
							element: <div>account-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/account?auth_redirect=login_success&kept=1"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		await waitFor(() =>
			expect(toastMock.success).toHaveBeenCalledWith("Welcome back"),
		);
		expect(router.state.location.pathname).toBe("/account");
		expect(router.state.location.search).toBe("?kept=1");
	});

	it("shows and clears the external account linked redirect toast", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout scope="account" />,
					children: [
						{
							path: "/account/wardrobe",
							element: <div>wardrobe-route</div>,
						},
					],
				},
			],
			{
				initialEntries: [
					"/account/wardrobe?auth_redirect=external_auth_linked#skins",
				],
			},
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("wardrobe-route")).toBeInTheDocument();
		await waitFor(() =>
			expect(toastMock.success).toHaveBeenCalledWith("External account linked"),
		);
		expect(router.state.location.pathname).toBe("/account/wardrobe");
		expect(router.state.location.search).toBe("");
		expect(router.state.location.hash).toBe("#skins");
	});

	it("defaults the desktop sidebar to collapsed below xl and expanded at xl", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		mockXlViewport(false);
		const compactRouter = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [{ path: "/account", element: <div>compact-route</div> }],
				},
			],
			{ initialEntries: ["/account"] },
		);

		const { unmount } = render(<RouterProvider router={compactRouter} />);

		expect(await screen.findByText("compact-route")).toBeInTheDocument();
		expect(
			document.querySelector('[data-slot="shell-desktop-sidebar"]'),
		).toHaveAttribute("data-state", "collapsed");

		unmount();
		mockXlViewport(true);
		const expandedRouter = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [{ path: "/account", element: <div>expanded-route</div> }],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={expandedRouter} />);

		expect(await screen.findByText("expanded-route")).toBeInTheDocument();
		expect(
			document.querySelector('[data-slot="shell-desktop-sidebar"]'),
		).toHaveAttribute("data-state", "expanded");
	});

	it("toggles and persists the desktop sidebar preference", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		mockXlViewport(true);
		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [{ path: "/account", element: <div>account-route</div> }],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		const sidebar = document.querySelector(
			'[data-slot="shell-desktop-sidebar"]',
		);
		expect(sidebar).toHaveAttribute("data-state", "expanded");

		fireEvent.click(
			screen.getByRole("button", { name: "Collapse navigation" }),
		);

		expect(sidebar).toHaveAttribute("data-state", "collapsed");
		expect(localStorage.getItem(DESKTOP_SIDEBAR_STORAGE_KEY)).toBe("false");
		expect(
			screen.getByRole("button", { name: "Expand navigation" }),
		).toHaveAttribute("aria-pressed", "false");

		fireEvent.click(screen.getByRole("button", { name: "Expand navigation" }));

		expect(sidebar).toHaveAttribute("data-state", "expanded");
		expect(localStorage.getItem(DESKTOP_SIDEBAR_STORAGE_KEY)).toBe("true");
		expect(
			screen.getByRole("button", { name: "Collapse navigation" }),
		).toHaveAttribute("aria-pressed", "true");
	});

	it("keeps a collapsed desktop sidebar when navigating from account to admin", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		mockXlViewport(true);
		localStorage.setItem(DESKTOP_SIDEBAR_STORAGE_KEY, "false");
		const router = createMemoryRouter(
			[
				{
					path: "/account",
					element: <AppLayout key="account" scope="account" />,
					children: [{ index: true, element: <div>account-route</div> }],
				},
				{
					path: "/admin/users",
					element: <AppLayout key="admin" scope="admin" />,
					children: [{ index: true, element: <div>admin-users-route</div> }],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		const accountSidebar = document.querySelector(
			'[data-slot="shell-desktop-sidebar"]',
		);
		expect(accountSidebar).toHaveAttribute("data-state", "collapsed");

		fireEvent.click(screen.getByRole("link", { name: "Users" }));

		expect(await screen.findByText("admin-users-route")).toBeInTheDocument();
		expect(
			document.querySelector('[data-slot="shell-desktop-sidebar"]'),
		).toHaveAttribute("data-state", "collapsed");
	});

	it("resets page scroll when navigating between admin detail pages", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		const scrollTo = vi.mocked(window.scrollTo);
		const router = createMemoryRouter(
			[
				{
					element: <AppLayout scope="admin" />,
					children: [
						{
							path: "/admin/users/1",
							element: (
								<div>
									admin-user-detail-route
									<Link to="/admin/minecraft-profiles/16eb7a7fa2124230959738ebe4e1b2d0">
										open-minecraft-profile
									</Link>
								</div>
							),
						},
						{
							path: "/admin/minecraft-profiles/:uuid",
							element: (
								<div>
									admin-minecraft-profile-route
									<Link to="/admin/users/1">open-user-detail</Link>
								</div>
							),
						},
					],
				},
			],
			{ initialEntries: ["/admin/users/1"] },
		);

		render(<RouterProvider router={router} />);

		expect(
			await screen.findByText("admin-user-detail-route"),
		).toBeInTheDocument();
		expect(scrollTo).not.toHaveBeenCalled();

		fireEvent.click(
			screen.getByRole("link", { name: "open-minecraft-profile" }),
		);

		expect(
			await screen.findByText("admin-minecraft-profile-route"),
		).toBeInTheDocument();
		expect(scrollTo).toHaveBeenLastCalledWith({
			top: 0,
			left: 0,
			behavior: "auto",
		});

		scrollTo.mockClear();
		fireEvent.click(screen.getByRole("link", { name: "open-user-detail" }));

		expect(
			await screen.findByText("admin-user-detail-route"),
		).toBeInTheDocument();
		expect(scrollTo).toHaveBeenLastCalledWith({
			top: 0,
			left: 0,
			behavior: "auto",
		});
	});

	it("loads a stored desktop sidebar preference across a remount", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		mockXlViewport(true);
		localStorage.setItem(DESKTOP_SIDEBAR_STORAGE_KEY, "false");
		const collapsedRouter = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{ path: "/account", element: <div>stored-collapsed-route</div> },
					],
				},
			],
			{ initialEntries: ["/account"] },
		);

		const { unmount } = render(<RouterProvider router={collapsedRouter} />);

		expect(
			await screen.findByText("stored-collapsed-route"),
		).toBeInTheDocument();
		expect(
			document.querySelector('[data-slot="shell-desktop-sidebar"]'),
		).toHaveAttribute("data-state", "collapsed");

		unmount();
		localStorage.setItem(DESKTOP_SIDEBAR_STORAGE_KEY, "true");
		const expandedRouter = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{ path: "/account", element: <div>stored-expanded-route</div> },
					],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={expandedRouter} />);

		expect(
			await screen.findByText("stored-expanded-route"),
		).toBeInTheDocument();
		expect(
			document.querySelector('[data-slot="shell-desktop-sidebar"]'),
		).toHaveAttribute("data-state", "expanded");
	});

	it("falls back to the viewport default when the stored sidebar preference is invalid", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		mockXlViewport(true);
		localStorage.setItem(DESKTOP_SIDEBAR_STORAGE_KEY, "maybe");
		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [{ path: "/account", element: <div>account-route</div> }],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		expect(
			document.querySelector('[data-slot="shell-desktop-sidebar"]'),
		).toHaveAttribute("data-state", "expanded");
	});

	it("keeps the desktop sidebar usable when preference storage is unavailable", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });
		mockXlViewport(false);
		const getItemSpy = vi
			.spyOn(Storage.prototype, "getItem")
			.mockImplementation((key: string) => {
				if (key === DESKTOP_SIDEBAR_STORAGE_KEY) {
					throw new Error("storage unavailable");
				}
				return null;
			});
		const setItemSpy = vi
			.spyOn(Storage.prototype, "setItem")
			.mockImplementation((key: string) => {
				if (key === DESKTOP_SIDEBAR_STORAGE_KEY) {
					throw new Error("storage unavailable");
				}
			});
		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [{ path: "/account", element: <div>account-route</div> }],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		const sidebar = document.querySelector(
			'[data-slot="shell-desktop-sidebar"]',
		);
		expect(sidebar).toHaveAttribute("data-state", "collapsed");

		fireEvent.click(screen.getByRole("button", { name: "Expand navigation" }));

		expect(sidebar).toHaveAttribute("data-state", "expanded");
		expect(setItemSpy).toHaveBeenCalledWith(
			DESKTOP_SIDEBAR_STORAGE_KEY,
			"true",
		);

		getItemSpy.mockRestore();
		setItemSpy.mockRestore();
	});

	it("hides the account mobile topbar brand and removes the desktop search box", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{
							path: "/account",
							element: <div>account-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		const topbar = screen.getByRole("banner");
		expect(
			within(topbar).queryByRole("link", { name: /AsterYggdrasil/ }),
		).not.toBeInTheDocument();
		expect(
			within(topbar).queryByPlaceholderText(
				"Search players, UUIDs, sessions, or settings...",
			),
		).not.toBeInTheDocument();
		expect(within(topbar).queryByText("⌘K")).not.toBeInTheDocument();
	});

	it("keeps the admin mobile topbar brand while removing the desktop search box", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{
							path: "/admin/settings",
							element: <div>admin-settings-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/admin/settings"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("admin-settings-route")).toBeInTheDocument();
		const topbar = screen.getByRole("banner");
		const brandLink = within(topbar).getByRole("link", {
			name: /AsterYggdrasil/,
		});
		expect(brandLink).toHaveAttribute("href", "/");
		expect(brandLink.querySelector("img")).toHaveClass("size-9");
		expect(
			within(topbar).queryByPlaceholderText(
				"Search players, UUIDs, sessions, or settings...",
			),
		).not.toBeInTheDocument();
		expect(within(topbar).queryByText("⌘K")).not.toBeInTheDocument();
	});

	it("opens the shared app navigation from the mobile topbar", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{
							path: "/account",
							element: <div>account-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Open navigation" }));

		const drawer = document.querySelector('[data-slot="shell-mobile-drawer"]');
		expect(drawer).toBeTruthy();
		expect(drawer).toHaveClass("translate-x-0", "shadow-2xl");
		expect(within(drawer as HTMLElement).getByText("My space")).toBeVisible();
		expect(
			within(drawer as HTMLElement).getByText("Administration"),
		).toBeVisible();
		expect(
			within(drawer as HTMLElement).getByRole("link", { name: "Users" }),
		).toHaveAttribute("href", "/admin/users");

		fireEvent.click(
			within(drawer as HTMLElement).getByRole("button", { name: "Close" }),
		);

		expect(drawer).toHaveClass("-translate-x-[calc(100%+1rem)]", "shadow-none");
	});

	it("shows home and personal settings actions in the admin user menu", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{
							path: "/admin/settings",
							element: <div>admin-settings-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/admin/settings"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("admin-settings-route")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "alex" }));

		const menu = document.querySelector('[data-slot="dropdown-menu-content"]');
		expect(menu).toBeTruthy();
		expect(
			within(menu as HTMLElement).getByRole("menuitem", {
				name: "Back to home",
			}),
		).toHaveAttribute("href", "/");
		expect(
			within(menu as HTMLElement).getByRole("menuitem", {
				name: "Personal settings",
			}),
		).toHaveAttribute("href", "/account/settings");
		expect(
			within(menu as HTMLElement).queryByRole("menuitem", {
				name: "Admin",
			}),
		).not.toBeInTheDocument();
		expect(
			within(menu as HTMLElement).queryByRole("separator"),
		).not.toBeInTheDocument();
	});

	it("shows a home action in the account user menu", async () => {
		setAuthenticatedUser(undefined, { isAdmin: true });

		const router = createMemoryRouter(
			[
				{
					element: <AppLayout />,
					children: [
						{
							path: "/account",
							element: <div>account-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "alex" }));

		const menu = document.querySelector('[data-slot="dropdown-menu-content"]');
		expect(menu).toBeTruthy();
		expect(
			within(menu as HTMLElement).getByRole("menuitem", {
				name: "Back to home",
			}),
		).toHaveAttribute("href", "/");
		expect(
			within(menu as HTMLElement).queryByRole("menuitem", {
				name: "Personal settings",
			}),
		).not.toBeInTheDocument();
	});

	it("shows a success toast after logging out from the user menu", async () => {
		setAuthenticatedUser();

		const router = createMemoryRouter(
			[
				{
					path: "/",
					element: <div>home-route</div>,
				},
				{
					element: <AppLayout />,
					children: [
						{
							path: "/account",
							element: <div>account-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "alex" }));
		fireEvent.click(screen.getByRole("menuitem", { name: "Logout" }));

		await waitFor(() => {
			expect(authServiceMock.logout).toHaveBeenCalled();
		});
		expect(toastMock.success).toHaveBeenCalledWith("Logged out");
		expect(await screen.findByText("home-route")).toBeInTheDocument();
		expect(router.state.location.pathname).toBe("/");
	});

	it("warns but still leaves the shell when logout revocation fails", async () => {
		setAuthenticatedUser();
		authServiceMock.logout.mockRejectedValueOnce(new Error("network gone"));

		const router = createMemoryRouter(
			[
				{
					path: "/",
					element: <div>home-route</div>,
				},
				{
					element: <AppLayout />,
					children: [
						{
							path: "/account",
							element: <div>account-route</div>,
						},
					],
				},
			],
			{ initialEntries: ["/account"] },
		);

		render(<RouterProvider router={router} />);

		expect(await screen.findByText("account-route")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "alex" }));
		fireEvent.click(screen.getByRole("menuitem", { name: "Logout" }));

		await waitFor(() => {
			expect(toastMock.warning).toHaveBeenCalledWith(
				"Signed out locally, but the server session may not have been revoked and could still be active",
			);
		});
		expect(toastMock.success).not.toHaveBeenCalled();
		expect(router.state.location.pathname).toBe("/");
	});
});
