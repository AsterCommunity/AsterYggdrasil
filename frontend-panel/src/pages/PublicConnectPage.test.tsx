import { render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import { useInitStatusStore } from "@/stores/initStatusStore";
import { setAuthenticatedUser } from "@/test/authState";
import { deferred } from "@/test/deferred";
import { createAuthUser } from "@/test/fixtures";
import PublicConnectPage from "./PublicConnectPage";

const authServiceMock = vi.hoisted(() => ({
	me: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

function renderPublicConnectPage() {
	return render(
		<MemoryRouter>
			<PublicConnectPage />
		</MemoryRouter>,
	);
}

describe("PublicConnectPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		useFrontendConfigStore.getState().invalidate();
		useInitStatusStore.getState().reset();
		authServiceMock.me.mockRejectedValue(new Error("unauthenticated"));
	});

	it("renders the public Yggdrasil entry route from public frontend config", async () => {
		useFrontendConfigStore.setState({
			branding: {
				title: "AsterYggdrasil",
				description: "AsterYggdrasil public config",
				faviconUrl: "",
				wordmarkDarkUrl: "",
				wordmarkLightUrl: "",
			},
			config: {
				version: 1,
				branding: {
					title: "AsterYggdrasil",
					description: "AsterYggdrasil public config",
					favicon_url: "",
					wordmark_dark_url: "",
					wordmark_light_url: "",
					site_urls: ["http://localhost:5173"],
					allow_user_registration: true,
				},
				yggdrasil: {
					server_name: "AsterYggdrasil",
					skin_domains: ["localhost"],
					public_base_urls: ["http://localhost:5173"],
					allow_profile_name_login: true,
					allow_skin_upload: true,
					allow_cape_upload: true,
					max_texture_pixels: 4096 * 4096,
					max_texture_upload_bytes: 4 * 1024 * 1024,
				},
			},
			isLoaded: true,
			yggdrasil: {
				server_name: "AsterYggdrasil",
				skin_domains: ["localhost"],
				public_base_urls: ["http://localhost:5173"],
				allow_profile_name_login: true,
				allow_skin_upload: true,
				allow_cape_upload: true,
				max_texture_pixels: 4096 * 4096,
				max_texture_upload_bytes: 4 * 1024 * 1024,
			},
		});

		renderPublicConnectPage();

		expect(
			await screen.findByRole("heading", {
				level: 1,
				name: /Your Minecraft identity and skin hub/,
			}),
		).toBeInTheDocument();
		expect(
			screen.getAllByText("AsterYggdrasil public config").length,
		).toBeGreaterThan(0);
		expect(screen.getByLabelText("Language")).toBeInTheDocument();
		expect(
			screen.getByRole("link", { name: "Login / Register" }),
		).toHaveAttribute("href", "/login");
		expect(screen.getByRole("link", { name: /Get started/ })).toHaveAttribute(
			"href",
			"/login",
		);
		expect(screen.queryByText("Learn more")).not.toBeInTheDocument();
		expect(screen.getByText("Safe and reliable")).toBeInTheDocument();
		expect(screen.getByText("Skin management")).toBeInTheDocument();
		expect(screen.getByText("Fast and stable")).toBeInTheDocument();
		expect(screen.getByText("Server support")).toBeInTheDocument();
		expect(screen.getByText("Developer friendly")).toBeInTheDocument();
		expect(screen.getByText("Skin ecosystem")).toBeInTheDocument();
		expect(screen.getByText("Community driven")).toBeInTheDocument();
	});

	it("renders the public entry through the shared public shell", async () => {
		renderPublicConnectPage();

		expect(
			await screen.findByRole("heading", {
				level: 1,
				name: /Your Minecraft identity and skin hub/,
			}),
		).toBeInTheDocument();

		const brandLink = within(screen.getByRole("banner")).getByRole("link", {
			name: /AsterYggdrasil.*Minecraft skin site and Yggdrasil authentication server/,
		});
		expect(brandLink).toHaveAttribute("href", "/");
		expect(brandLink.querySelector("img")).toHaveClass("size-10");

		expect(
			document.querySelector('[data-slot="public-entry-backdrop-image"]'),
		).toHaveClass("fixed", "inset-0", "bg-cover", "bg-center");
		expect(screen.getByLabelText("Language")).toHaveClass("size-10");
		expect(document.querySelector(".public-home-enter")).toBeInTheDocument();
		expect(
			document.querySelector(".app-route-transition"),
		).not.toBeInTheDocument();
	});

	it("shows console CTA on the public entry when authenticated", async () => {
		const user = createAuthUser();
		setAuthenticatedUser(user);
		authServiceMock.me.mockResolvedValue(user);

		renderPublicConnectPage();

		expect(
			await screen.findByRole("link", { name: /Enter console/ }),
		).toHaveAttribute("href", "/account");
		expect(screen.queryByText("Learn more")).not.toBeInTheDocument();
		expect(screen.queryByText("Get started")).not.toBeInTheDocument();
	});

	it("renders the public entry before the session check finishes, then switches to the authenticated CTA", async () => {
		const user = createAuthUser();
		const me = deferred<typeof user>();
		authServiceMock.me.mockImplementation(() => me.promise);
		useAuthStore.setState({
			user: null,
			checking: true,
			error: null,
			expiresAt: null,
			isAuthStale: false,
			isAuthenticated: false,
			isAdmin: false,
		});

		renderPublicConnectPage();

		expect(
			await screen.findByRole("heading", {
				level: 1,
				name: /Your Minecraft identity and skin hub/,
			}),
		).toBeInTheDocument();
		expect(screen.getByRole("link", { name: /Get started/ })).toHaveAttribute(
			"href",
			"/login",
		);
		expect(
			screen.queryByRole("link", { name: /Enter console/ }),
		).not.toBeInTheDocument();

		me.resolve(user);

		expect(
			await screen.findByRole("link", { name: /Enter console/ }),
		).toHaveAttribute("href", "/account");
		expect(screen.queryByText("Get started")).not.toBeInTheDocument();
	});
});
