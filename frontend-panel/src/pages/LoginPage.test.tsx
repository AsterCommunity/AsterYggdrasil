import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { AuthUserInfo } from "@/types/api";
import LoginPage from "./LoginPage";

const authServiceMock = vi.hoisted(() => ({
	check: vi.fn(),
	finishPasskeyLogin: vi.fn(),
	linkExternalAuthWithPassword: vi.fn(),
	login: vi.fn(),
	me: vi.fn(),
	register: vi.fn(),
	setup: vi.fn(),
	startExternalAuthEmailVerification: vi.fn(),
	startPasskeyLogin: vi.fn(),
}));

const externalAuthServiceMock = vi.hoisted(() => ({
	listPublic: vi.fn(),
	startAuthAlias: vi.fn(),
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

function renderLoginPage(initialEntry = "/login") {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<LocationProbe />
			<LoginPage />
		</MemoryRouter>,
	);
}

function LocationProbe() {
	const location = useLocation();
	return (
		<output data-testid="location">
			{location.pathname}
			{location.search}
		</output>
	);
}

describe("LoginPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		useFrontendConfigStore.getState().invalidate();
		authServiceMock.check.mockResolvedValue({ initialized: true });
		authServiceMock.login.mockResolvedValue({
			expires_in: 3600,
			status: "authenticated",
		});
		authServiceMock.register.mockResolvedValue({
			expires_in: 3600,
			requires_activation: false,
		});
		authServiceMock.setup.mockResolvedValue({
			expires_in: 3600,
			status: "authenticated",
		});
		authServiceMock.finishPasskeyLogin.mockResolvedValue({
			expires_in: 3600,
			status: "authenticated",
		});
		authServiceMock.startExternalAuthEmailVerification.mockResolvedValue({
			message: "ok",
		});
		authServiceMock.linkExternalAuthWithPassword.mockResolvedValue({
			expires_in: 3600,
			flow_token: null,
			return_path: "/account",
			status: "authenticated",
		});
		authServiceMock.me.mockResolvedValue(user);
		externalAuthServiceMock.listPublic.mockResolvedValue([]);
		externalAuthServiceMock.startAuthAlias.mockResolvedValue({
			authorization_url: "https://example.com/oauth",
		});
	});

	it("shows a welcome toast after password login succeeds", async () => {
		renderLoginPage();

		fireEvent.change(await screen.findByLabelText("login.identifier"), {
			target: { value: "alex" },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(screen.getByRole("button", { name: "nav.login" }));

		await waitFor(() =>
			expect(authServiceMock.login).toHaveBeenCalledWith({
				identifier: "alex",
				password: "secret-password",
			}),
		);
		expect(toastMock.success).toHaveBeenCalledWith("login.loginSuccess");
	});

	it("opens external auth recovery from callback query and clears callback params", async () => {
		renderLoginPage(
			"/login?external_auth=email_required&flow=flow-1&return_path=%2Faccount%2Fwardrobe&kept=1",
		);

		expect(
			await screen.findByText("login.externalAuthRecoveryTitle"),
		).toBeInTheDocument();
		expect(
			screen.getByRole("tab", { name: "login.externalAuthPasswordTab" }),
		).toBeInTheDocument();
		expect(
			screen.getByRole("tab", { name: "login.externalAuthEmailTab" }),
		).toBeInTheDocument();
		expect(screen.getByLabelText("login.identifier")).toHaveValue("");
		await waitFor(() =>
			expect(screen.getByTestId("location")).toHaveTextContent("/login?kept=1"),
		);
	});

	it("links an external auth flow to an existing account with password", async () => {
		renderLoginPage(
			"/login?external_auth=email_required&flow=flow-1&return_path=%2Faccount%2Fwardrobe",
		);

		fireEvent.change(await screen.findByLabelText("login.identifier"), {
			target: { value: "  alex  " },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(
			screen.getByRole("button", {
				name: "login.externalAuthPasswordSubmit",
			}),
		);

		await waitFor(() =>
			expect(authServiceMock.linkExternalAuthWithPassword).toHaveBeenCalledWith(
				{
					flow_token: "flow-1",
					identifier: "alex",
					password: "secret-password",
				},
			),
		);
		expect(toastMock.success).not.toHaveBeenCalledWith(
			"login.externalAuthPasswordLinked",
		);
		await waitFor(() =>
			expect(screen.getByTestId("location")).toHaveTextContent(
				"/account/wardrobe?auth_redirect=external_auth_linked",
			),
		);
	});

	it("does not submit external auth password linking until required fields are present", async () => {
		renderLoginPage("/login?external_auth=email_required&flow=flow-1");

		const submitButton = await screen.findByRole("button", {
			name: "login.externalAuthPasswordSubmit",
		});
		expect(submitButton).toBeDisabled();

		fireEvent.change(screen.getByLabelText("login.identifier"), {
			target: { value: "alex" },
		});
		expect(submitButton).toBeDisabled();
		expect(authServiceMock.linkExternalAuthWithPassword).not.toHaveBeenCalled();

		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		expect(submitButton).not.toBeDisabled();
	});

	it("sends password-change-required external auth links to force password change", async () => {
		authServiceMock.linkExternalAuthWithPassword.mockResolvedValue({
			expires_in: null,
			flow_token: null,
			return_path: "/account",
			status: "password_change_required",
		});
		renderLoginPage(
			"/login?external_auth=email_required&flow=flow-1&return_path=%2Faccount",
		);

		fireEvent.change(await screen.findByLabelText("login.identifier"), {
			target: { value: "alex" },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(
			screen.getByRole("button", {
				name: "login.externalAuthPasswordSubmit",
			}),
		);

		await waitFor(() =>
			expect(screen.getByTestId("location")).toHaveTextContent(
				"/force-password-change?auth_redirect=external_auth_linked",
			),
		);
	});

	it("validates and requests external auth email verification", async () => {
		renderLoginPage("/login?external_auth=email_required&flow=flow-1");

		fireEvent.click(
			await screen.findByRole("tab", { name: "login.externalAuthEmailTab" }),
		);
		const emailInput = screen.getByLabelText("login.email");
		const submitButton = screen.getByRole("button", {
			name: "login.externalAuthEmailSubmit",
		});

		expect(submitButton).toBeDisabled();
		fireEvent.change(emailInput, { target: { value: "not-an-email" } });
		expect(
			screen.getByText("login.validationEmailInvalid"),
		).toBeInTheDocument();
		expect(submitButton).toBeDisabled();

		fireEvent.change(emailInput, { target: { value: " alex@example.com " } });
		expect(
			screen.queryByText("login.validationEmailInvalid"),
		).not.toBeInTheDocument();
		expect(submitButton).not.toBeDisabled();
		fireEvent.click(submitButton);

		await waitFor(() =>
			expect(
				authServiceMock.startExternalAuthEmailVerification,
			).toHaveBeenCalledWith({
				email: "alex@example.com",
				flow_token: "flow-1",
			}),
		);
		expect(toastMock.success).toHaveBeenCalledWith(
			"login.externalAuthEmailSentToast",
		);
		expect(
			await screen.findByText("login.externalAuthEmailSentTitle"),
		).toBeInTheDocument();
	});

	it.each([
		[
			"/login?external_auth=email_verification_missing",
			"login.externalAuthEmailMissing",
		],
		[
			"/login?external_auth=email_verification_invalid",
			"login.externalAuthEmailInvalid",
		],
		[
			"/login?external_auth=email_verification_expired",
			"login.externalAuthEmailExpired",
		],
		["/login?external_auth=error&message=provider%20down", "provider down"],
		["/login?external_auth=error", "login.externalAuthFailed"],
	] as const)(
		"shows external auth callback error for %s",
		async (entry, message) => {
			renderLoginPage(entry);

			await waitFor(() =>
				expect(toastMock.error).toHaveBeenCalledWith(message),
			);
			await waitFor(() =>
				expect(screen.getByTestId("location")).toHaveTextContent("/login"),
			);
		},
	);

	it("disables password login until required fields are valid", async () => {
		renderLoginPage();

		const submitButton = await screen.findByRole("button", {
			name: "nav.login",
		});
		expect(submitButton).toBeDisabled();

		fireEvent.change(screen.getByLabelText("login.identifier"), {
			target: { value: "alex" },
		});
		fireEvent.change(screen.getByLabelText("login.password"), {
			target: { value: "secret-password" },
		});
		expect(submitButton).not.toBeDisabled();
	});

	it("links the login form to password reset", async () => {
		renderLoginPage();

		expect(
			await screen.findByRole("link", { name: "login.forgotPassword" }),
		).toHaveAttribute("href", "/reset-password");
	});

	it.each([
		[
			"/login?contact_verification=register-activated",
			"login.activationSuccess",
			"success",
		],
		[
			"/login?contact_verification=invalid",
			"login.contactVerificationInvalid",
			"error",
		],
		[
			"/login?contact_verification=expired",
			"login.contactVerificationExpired",
			"error",
		],
		[
			"/login?contact_verification=missing",
			"login.contactVerificationMissing",
			"error",
		],
		["/login?password_reset=success", "login.passwordResetSuccess", "success"],
	] as const)("shows redirect toast for %s", async (entry, message, kind) => {
		renderLoginPage(entry);

		await waitFor(() => {
			if (kind === "success") {
				expect(toastMock.success).toHaveBeenCalledWith(message);
			} else {
				expect(toastMock.error).toHaveBeenCalledWith(message);
			}
		});
	});

	it("preserves caret position when editing login fields in the middle", async () => {
		renderLoginPage();

		const identifierInput = (await screen.findByLabelText(
			"login.identifier",
		)) as HTMLInputElement;
		fireEvent.change(identifierInput, {
			target: { value: "esap" },
		});
		identifierInput.focus();
		identifierInput.setSelectionRange(2, 2);
		fireEvent.change(identifierInput, {
			target: { selectionEnd: 3, selectionStart: 3, value: "esXap" },
		});

		await waitFor(() => {
			expect(identifierInput).toHaveValue("esXap");
			expect(identifierInput.selectionStart).toBe(3);
			expect(identifierInput.selectionEnd).toBe(3);
		});

		const passwordInput = screen.getByLabelText(
			"login.password",
		) as HTMLInputElement;
		fireEvent.change(passwordInput, {
			target: { value: "secret-password" },
		});
		passwordInput.focus();
		passwordInput.setSelectionRange(6, 6);
		fireEvent.change(passwordInput, {
			target: { selectionEnd: 7, selectionStart: 7, value: "secretX-password" },
		});

		await waitFor(() => {
			expect(passwordInput).toHaveValue("secretX-password");
			expect(passwordInput.selectionStart).toBe(7);
			expect(passwordInput.selectionEnd).toBe(7);
		});
	});

	it("shows a success toast after registration succeeds", async () => {
		renderLoginPage("/register");

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
		renderLoginPage("/register");

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
		renderLoginPage("/register");

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
		renderLoginPage("/register");

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
		renderLoginPage("/register");

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
		renderLoginPage("/register");

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
		renderLoginPage("/register");

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
