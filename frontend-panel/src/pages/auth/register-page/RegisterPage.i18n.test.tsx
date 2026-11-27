import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import RegisterPage from "@/pages/auth/register-page/RegisterPage";

/**
 * Rendered-locale coverage for RegisterPage: these tests exercise the real
 * i18n pipeline (no react-i18next mock) alongside the shared public shell.
 * The behavioral tests in RegisterPage.test.tsx mock `t` and assert keys.
 */

const authServiceMock = vi.hoisted(() => ({
	check: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

function renderRegisterPage() {
	return render(
		<MemoryRouter initialEntries={["/register"]}>
			<RegisterPage />
		</MemoryRouter>,
	);
}

describe("RegisterPage rendered locale", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		authServiceMock.check.mockResolvedValue({ initialized: true });
	});

	it("renders the register form on the register route", async () => {
		authServiceMock.check.mockResolvedValue({ initialized: true });

		renderRegisterPage();

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
