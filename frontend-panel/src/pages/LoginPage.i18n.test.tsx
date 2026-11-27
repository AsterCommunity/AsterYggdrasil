import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import LoginPage from "./LoginPage";

/**
 * Rendered-locale coverage for LoginPage: these tests exercise the real
 * i18n pipeline (no react-i18next mock) alongside the shared public shell.
 * The behavioral tests in LoginPage.test.tsx mock `t` and assert keys.
 */

const authServiceMock = vi.hoisted(() => ({
	check: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

function renderLoginPage(initialEntry = "/login") {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<LoginPage />
		</MemoryRouter>,
	);
}

describe("LoginPage rendered locale", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		authServiceMock.check.mockResolvedValue({ initialized: true });
	});

	it("renders the login form when auth check reports an initialized system", async () => {
		authServiceMock.check.mockResolvedValue({ initialized: true });

		renderLoginPage();

		expect(
			await screen.findByRole("heading", { level: 1, name: "Login" }),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Email or username")).toBeInTheDocument();
		expect(screen.getByLabelText("Password")).toBeInTheDocument();
		const passwordInput = screen.getByLabelText("Password");
		expect(passwordInput).toHaveAttribute("type", "password");

		const revealButton = screen.getByRole("button", {
			name: "Show password",
		});
		expect(revealButton).toHaveClass("bg-transparent", "size-6");
		expect(revealButton).not.toHaveAttribute("data-slot", "button");

		fireEvent.click(revealButton);

		expect(passwordInput).toHaveAttribute("type", "text");
		expect(
			screen.getByRole("button", { name: "Hide password" }),
		).toBeInTheDocument();
	});

	it("renders login through the shared public shell while keeping page animation on the body", async () => {
		authServiceMock.check.mockResolvedValue({ initialized: true });

		renderLoginPage();

		expect(
			await screen.findByRole("heading", { level: 1, name: "Login" }),
		).toBeInTheDocument();

		const brandLink = within(screen.getByRole("banner")).getByRole("link", {
			name: /AsterYggdrasil.*Minecraft skin site and Yggdrasil authentication server/,
		});
		expect(brandLink).toHaveAttribute("href", "/");
		expect(brandLink.querySelector("img")).toHaveClass("size-10");

		expect(
			document.querySelector('[data-slot="public-entry-backdrop-image"]'),
		).toHaveClass("fixed", "inset-0", "bg-cover", "bg-center");
		const languageSelect = screen.getByLabelText("Language");
		expect(languageSelect).toHaveClass("h-10");
		expect(languageSelect).not.toHaveClass("hidden");
		expect(
			document.querySelector("main.app-route-transition"),
		).toBeInTheDocument();
		expect(
			document.querySelector("header.app-route-transition"),
		).not.toBeInTheDocument();
	});

	it("renders the register form on the register route", async () => {
		authServiceMock.check.mockResolvedValue({ initialized: true });

		renderLoginPage("/register");

		expect(
			await screen.findByRole("heading", {
				level: 1,
				name: "Create account",
			}),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Username")).toBeInTheDocument();
		expect(screen.getByLabelText("Email")).toBeInTheDocument();
		expect(screen.getByLabelText("Password")).toBeInTheDocument();
		expect(screen.getByLabelText("Confirm password")).toBeInTheDocument();
		expect(screen.getByRole("link", { name: "Login" })).toHaveAttribute(
			"href",
			"/login",
		);
	});
});
