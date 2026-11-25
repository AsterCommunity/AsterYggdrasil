import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { PublicTextureHeaderActions } from "@/components/yggdrasil/PublicTextureHeaderActions";
import type { AuthUserInfo } from "@/types/api";

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (key: string) => key,
	}),
}));

vi.mock("@/components/common/AuthUserMenu", () => ({
	AuthUserMenu: () => <div data-testid="auth-user-menu" />,
}));

const baseUser: AuthUserInfo = {
	email: "alex@example.com",
	id: 7,
	profile: {
		avatar: {
			source: "none",
			url_1024: null,
			url_512: null,
			version: 0,
		},
		display_name: null,
	},
	role: "user",
	status: "active",
	username: "alex",
};

function renderActions(
	props: Partial<Parameters<typeof PublicTextureHeaderActions>[0]> = {},
) {
	const { isAuthenticated = true, user = baseUser, onLogout = vi.fn() } = props;
	return render(
		<MemoryRouter>
			<PublicTextureHeaderActions
				isAuthenticated={isAuthenticated}
				user={user}
				onLogout={onLogout}
			/>
		</MemoryRouter>,
	);
}

describe("PublicTextureHeaderActions", () => {
	it("renders the login link for signed-out visitors", () => {
		renderActions({ isAuthenticated: false, user: null });

		const loginLink = screen.getByRole("link", { name: "home.loginRegister" });
		expect(loginLink).toHaveAttribute("href", "/login");
		expect(screen.queryByTestId("auth-user-menu")).not.toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: "nav.backToAccount" }),
		).not.toBeInTheDocument();
	});

	it("renders the login link when the user object is missing", () => {
		renderActions({ isAuthenticated: true, user: null });

		expect(
			screen.getByRole("link", { name: "home.loginRegister" }),
		).toHaveAttribute("href", "/login");
	});

	it("renders the workbench link alongside the user menu when signed in", () => {
		renderActions();

		const workbenchLink = screen.getByRole("link", {
			name: "nav.backToAccount",
		});
		expect(workbenchLink).toHaveAttribute("href", "/account");
		expect(workbenchLink).toHaveAttribute("aria-label", "nav.backToAccount");
		expect(screen.getByTestId("auth-user-menu")).toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: "home.loginRegister" }),
		).not.toBeInTheDocument();
	});
});
