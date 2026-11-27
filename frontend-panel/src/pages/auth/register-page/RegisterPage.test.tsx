import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import RegisterPage from "@/pages/auth/register-page/RegisterPage";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { AuthUserInfo } from "@/types/api";

const authServiceMock = vi.hoisted(() => ({
	check: vi.fn(),
	me: vi.fn(),
	register: vi.fn(),
}));

const externalAuthServiceMock = vi.hoisted(() => ({
	listPublic: vi.fn(),
}));

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const translationMock = vi.hoisted(() => ({
	i18n: {
		changeLanguage: vi.fn(),
		language: "en-US",
	},
	t: (key: string) => key,
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

vi.mock("@/services/externalAuthService", () => ({
	externalAuthService: externalAuthServiceMock,
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

vi.mock("react-i18next", () => ({
	initReactI18next: {
		init: vi.fn(),
		type: "3rdParty",
	},
	useTranslation: () => translationMock,
}));

const user: AuthUserInfo = {
	id: 7,
	username: "alex",
	email: "alex@example.com",
	email_verified: true,
	must_change_password: false,
	pending_email: null,
	role: "user",
	status: "active",
	profile: {
		display_name: null,
		avatar: {
			source: "none",
			url_512: null,
			url_1024: null,
			version: 0,
		},
	},
};

function renderRegisterPage() {
	return render(
		<MemoryRouter initialEntries={["/register"]}>
			<RegisterPage />
		</MemoryRouter>,
	);
}

describe("RegisterPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		useFrontendConfigStore.getState().invalidate();
		authServiceMock.check.mockResolvedValue({ initialized: true });
		authServiceMock.me.mockResolvedValue(user);
		authServiceMock.register.mockResolvedValue({
			expires_in: 3600,
			requires_activation: false,
		});
		externalAuthServiceMock.listPublic.mockResolvedValue([]);
	});

	it("shows a success toast after registration succeeds", async () => {
		renderRegisterPage();

		fireEvent.change(await screen.findByLabelText("login.username"), {
			target: { value: "alex-1" },
		});
		fireEvent.change(screen.getByLabelText("login.email"), {
			target: { value: "alex@example.com" },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("login.confirmPassword"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(screen.getByLabelText("login.acceptTerms"));
		fireEvent.click(screen.getByRole("button", { name: "login.registerNow" }));

		await waitFor(() =>
			expect(authServiceMock.register).toHaveBeenCalledWith({
				username: "alex-1",
				email: "alex@example.com",
				password: "secret-password",
			}),
		);
		expect(toastMock.success).toHaveBeenCalledWith("login.registerSuccess");
	});

	it("trims register identity fields after zod validation", async () => {
		renderRegisterPage();

		fireEvent.change(await screen.findByLabelText("login.username"), {
			target: { value: "  alex  " },
		});
		fireEvent.change(screen.getByLabelText("login.email"), {
			target: { value: "  alex@example.com  " },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("login.confirmPassword"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(screen.getByLabelText("login.acceptTerms"));
		fireEvent.click(screen.getByRole("button", { name: "login.registerNow" }));

		await waitFor(() =>
			expect(authServiceMock.register).toHaveBeenCalledWith({
				username: "alex",
				email: "alex@example.com",
				password: "secret-password",
			}),
		);
	});

	it("validates register fields while typing", async () => {
		renderRegisterPage();

		const usernameInput = await screen.findByLabelText("login.username");
		const emailInput = screen.getByLabelText("login.email");
		const passwordInput = screen.getByLabelText("login.password");
		const confirmPasswordInput = screen.getByLabelText("login.confirmPassword");

		fireEvent.change(usernameInput, { target: { value: "abc" } });
		expect(
			screen.getByText("login.validationUsernameLength"),
		).toBeInTheDocument();
		fireEvent.change(usernameInput, { target: { value: "alex-1" } });
		expect(
			screen.queryByText("login.validationUsernameLength"),
		).not.toBeInTheDocument();

		fireEvent.change(usernameInput, { target: { value: "bad name" } });
		expect(
			screen.getByText("login.validationUsernameChars"),
		).toBeInTheDocument();
		fireEvent.change(usernameInput, { target: { value: "alex_1" } });
		expect(
			screen.queryByText("login.validationUsernameChars"),
		).not.toBeInTheDocument();

		fireEvent.change(emailInput, { target: { value: "bad-email" } });
		expect(
			screen.getByText("login.validationEmailInvalid"),
		).toBeInTheDocument();
		fireEvent.change(emailInput, { target: { value: "alex@example.com" } });
		expect(
			screen.queryByText("login.validationEmailInvalid"),
		).not.toBeInTheDocument();

		fireEvent.change(passwordInput, { target: { value: "short" } });
		expect(
			screen.getByText("login.validationPasswordLength"),
		).toBeInTheDocument();
		fireEvent.change(passwordInput, { target: { value: "secret-password" } });
		expect(
			screen.queryByText("login.validationPasswordLength"),
		).not.toBeInTheDocument();

		fireEvent.change(confirmPasswordInput, { target: { value: "different" } });
		expect(screen.getByText("login.passwordMismatch")).toBeInTheDocument();
		fireEvent.change(confirmPasswordInput, {
			target: { value: "secret-password" },
		});
		expect(
			screen.queryByText("login.passwordMismatch"),
		).not.toBeInTheDocument();
	});

	it("disables register submit until all fields are valid", async () => {
		renderRegisterPage();

		fireEvent.change(await screen.findByLabelText("login.username"), {
			target: { value: "abc" },
		});
		fireEvent.change(screen.getByLabelText("login.email"), {
			target: { value: "not-an-email" },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("login.confirmPassword"), {
			target: { value: "secret-password" },
		});

		const submitButton = screen.getByRole("button", {
			name: "login.registerNow",
		});
		expect(submitButton).toBeDisabled();
		expect(authServiceMock.register).not.toHaveBeenCalled();
		expect(
			screen.getByText("login.validationUsernameLength"),
		).toBeInTheDocument();
		expect(
			screen.getByText("login.validationEmailInvalid"),
		).toBeInTheDocument();

		fireEvent.change(screen.getByLabelText("login.username"), {
			target: { value: "alex" },
		});
		fireEvent.change(screen.getByLabelText("login.email"), {
			target: { value: "alex@example.com" },
		});
		fireEvent.click(screen.getByLabelText("login.acceptTerms"));
		expect(submitButton).not.toBeDisabled();
	});

	it("validates register password confirmation with zod", async () => {
		renderRegisterPage();

		fireEvent.change(await screen.findByLabelText("login.username"), {
			target: { value: "alex" },
		});
		fireEvent.change(screen.getByLabelText("login.email"), {
			target: { value: "alex@example.com" },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("login.confirmPassword"), {
			target: { value: "different" },
		});
		fireEvent.click(screen.getByLabelText("login.acceptTerms"));
		expect(authServiceMock.register).not.toHaveBeenCalled();
		expect(screen.getByText("login.passwordMismatch")).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "login.registerNow" }),
		).toBeDisabled();
	});

	it("validates register password length boundaries with zod", async () => {
		renderRegisterPage();

		fireEvent.change(await screen.findByLabelText("login.username"), {
			target: { value: "alex" },
		});
		fireEvent.change(screen.getByLabelText("login.email"), {
			target: { value: "alex@example.com" },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "short" },
		});
		fireEvent.change(screen.getByLabelText("login.confirmPassword"), {
			target: { value: "short" },
		});
		fireEvent.click(screen.getByLabelText("login.acceptTerms"));
		expect(authServiceMock.register).not.toHaveBeenCalled();
		expect(
			screen.getByText("login.validationPasswordLength"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "login.registerNow" }),
		).toBeDisabled();

		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "a".repeat(129) },
		});
		fireEvent.change(screen.getByLabelText("login.confirmPassword"), {
			target: { value: "a".repeat(129) },
		});
		expect(authServiceMock.register).not.toHaveBeenCalled();
		expect(
			screen.getByText("login.validationPasswordLength"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("button", { name: "login.registerNow" }),
		).toBeDisabled();
	});

	it("updates password strength color on the register form", async () => {
		renderRegisterPage();

		const passwordInput = await screen.findByLabelText("login.password");

		fireEvent.change(passwordInput, { target: { value: "short" } });
		expect(screen.getByText("login.passwordStrengthWeak")).toHaveClass(
			"text-red-700",
		);

		fireEvent.change(passwordInput, { target: { value: "longpassword12" } });
		expect(screen.getByText("login.passwordStrengthMedium")).toHaveClass(
			"text-amber-700",
		);

		fireEvent.change(passwordInput, {
			target: { value: "LongPassword12!" },
		});
		expect(screen.getByText("login.passwordStrengthStrong")).toHaveClass(
			"text-emerald-700",
		);
	});
});
