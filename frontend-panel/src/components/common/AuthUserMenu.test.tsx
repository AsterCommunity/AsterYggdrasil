import { render, screen } from "@testing-library/react";
import { cloneElement, type ReactElement, type ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { AuthUserMenu } from "@/components/common/AuthUserMenu";
import type { AuthUserInfo } from "@/types/api";

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (key: string) => key,
	}),
}));

vi.mock("@/components/common/UserAvatarImage", () => ({
	UserAvatarImage: () => <div data-testid="user-avatar" />,
}));

vi.mock("@/components/ui/icon", () => ({
	Icon: () => <span data-testid="icon" />,
}));

vi.mock("@/components/ui/dropdown-menu", () => ({
	DropdownMenu: ({ children }: { children: ReactNode }) => (
		<div>{children}</div>
	),
	DropdownMenuTrigger: ({
		render,
		children,
	}: {
		render: ReactElement;
		children: ReactNode;
	}) => cloneElement(render, {}, children),
	DropdownMenuContent: ({ children }: { children: ReactNode }) => (
		<div>{children}</div>
	),
	DropdownMenuItem: ({
		render,
		children,
		onClick,
	}: {
		render: ReactElement;
		children: ReactNode;
		onClick?: () => void;
	}) => cloneElement(render, onClick ? { onClick } : {}, children),
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

function renderMenu(props: Partial<Parameters<typeof AuthUserMenu>[0]> = {}) {
	const {
		user = baseUser,
		scope = "public",
		onLogout = vi.fn(),
		...rest
	} = props;
	return render(
		<MemoryRouter>
			<AuthUserMenu user={user} scope={scope} onLogout={onLogout} {...rest} />
		</MemoryRouter>,
	);
}

describe("AuthUserMenu", () => {
	it("keeps the account link when showAccountLink defaults to true", () => {
		renderMenu({ user: { ...baseUser, role: "user" } });

		const accountLink = screen.getByRole("link", { name: "nav.account" });
		expect(accountLink).toHaveAttribute("href", "/account");
	});

	it("hides the whole entry group when only the account link is disabled", () => {
		const { container } = renderMenu({
			user: { ...baseUser, role: "user" },
			showAccountLink: false,
		});

		expect(
			screen.queryByRole("link", { name: "nav.account" }),
		).not.toBeInTheDocument();
		expect(
			screen.queryByRole("link", { name: "nav.admin" }),
		).not.toBeInTheDocument();
		expect(container.querySelector("div.mt-2.hidden")).not.toBeNull();
	});

	it("keeps the admin entry when account link is disabled for an admin", () => {
		renderMenu({
			user: { ...baseUser, role: "admin" },
			showAccountLink: false,
		});

		expect(
			screen.queryByRole("link", { name: "nav.account" }),
		).not.toBeInTheDocument();
		const adminLink = screen.getByRole("link", { name: "nav.admin" });
		expect(adminLink).toHaveAttribute("href", "/admin/settings");
	});
});
