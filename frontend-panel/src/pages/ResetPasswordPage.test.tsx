import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { ApiError } from "@/services/http";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import ResetPasswordPage from "./ResetPasswordPage";

const authServiceMock = vi.hoisted(() => ({
	confirmPasswordReset: vi.fn(),
	requestPasswordReset: vi.fn(),
}));

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

function renderResetPasswordPage(initialEntry = "/reset-password") {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<Routes>
				<Route path="/reset-password" element={<ResetPasswordPage />} />
				<Route path="/login" element={<div>login route</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

describe("ResetPasswordPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useFrontendConfigStore.getState().invalidate();
		authServiceMock.requestPasswordReset.mockResolvedValue({
			message: "accepted",
		});
		authServiceMock.confirmPasswordReset.mockResolvedValue({
			message: "changed",
		});
	});

	it("requests a reset email with a normalized address", async () => {
		renderResetPasswordPage();

		fireEvent.change(await screen.findByLabelText("Email"), {
			target: { value: "  alex@example.com  " },
		});
		fireEvent.click(screen.getByRole("button", { name: "Send reset email" }));

		await waitFor(() =>
			expect(authServiceMock.requestPasswordReset).toHaveBeenCalledWith({
				email: "alex@example.com",
			}),
		);
		expect(toastMock.success).toHaveBeenCalledWith(
			"If the email exists, a reset message will be sent to that address.",
		);
		expect(await screen.findByText("login route")).toBeInTheDocument();
	});

	it("blocks reset email requests until the email is valid", async () => {
		renderResetPasswordPage();

		const submit = await screen.findByRole("button", {
			name: "Send reset email",
		});
		expect(submit).toBeDisabled();

		fireEvent.change(screen.getByLabelText("Email"), {
			target: { value: "bad-email" },
		});
		expect(submit).toBeDisabled();
		expect(authServiceMock.requestPasswordReset).not.toHaveBeenCalled();

		fireEvent.change(screen.getByLabelText("Email"), {
			target: { value: "alex@example.com" },
		});
		expect(submit).not.toBeDisabled();
	});

	it("shows API failures while requesting a reset email", async () => {
		authServiceMock.requestPasswordReset.mockRejectedValueOnce(
			new ApiError("rate_limited", "too many requests"),
		);
		renderResetPasswordPage();

		fireEvent.change(await screen.findByLabelText("Email"), {
			target: { value: "alex@example.com" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Send reset email" }));

		await waitFor(() =>
			expect(toastMock.error).toHaveBeenCalledWith("too many requests"),
		);
		expect(screen.queryByText("login route")).not.toBeInTheDocument();
	});

	it("confirms a password reset token and returns to login", async () => {
		renderResetPasswordPage("/reset-password?token=reset-token");

		fireEvent.change(await screen.findByLabelText("Password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Update password" }));

		await waitFor(() =>
			expect(authServiceMock.confirmPasswordReset).toHaveBeenCalledWith({
				token: "reset-token",
				new_password: "secret-password",
			}),
		);
		expect(await screen.findByText("login route")).toBeInTheDocument();
	});

	it("validates password reset confirmation fields", async () => {
		renderResetPasswordPage("/reset-password?token=reset-token");

		const submit = await screen.findByRole("button", {
			name: "Update password",
		});
		expect(submit).toBeDisabled();

		fireEvent.change(screen.getByLabelText("Password"), {
			target: { value: "short" },
		});
		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "different" },
		});
		expect(
			screen.getByText("Password must be 8-128 characters."),
		).toBeInTheDocument();
		expect(screen.getByText("Passwords do not match.")).toBeInTheDocument();
		expect(submit).toBeDisabled();
		expect(authServiceMock.confirmPasswordReset).not.toHaveBeenCalled();

		fireEvent.change(screen.getByLabelText("Password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "secret-password" },
		});
		expect(submit).not.toBeDisabled();
	});

	it("shows invalid and expired token states from API errors", async () => {
		authServiceMock.confirmPasswordReset.mockRejectedValueOnce(
			new ApiError("auth.contact_verification_invalid", "invalid reset link"),
		);
		const { unmount } = renderResetPasswordPage(
			"/reset-password?token=invalid-token",
		);

		fireEvent.change(await screen.findByLabelText("Password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Update password" }));

		expect(await screen.findByText("Invalid reset link")).toBeInTheDocument();
		unmount();

		authServiceMock.confirmPasswordReset.mockRejectedValueOnce(
			new ApiError("auth.contact_verification_expired", "expired reset link"),
		);
		renderResetPasswordPage("/reset-password?token=expired-token");

		fireEvent.change(await screen.findByLabelText("Password"), {
			target: { value: "secret-password" },
		});
		fireEvent.change(screen.getByLabelText("Confirm password"), {
			target: { value: "secret-password" },
		});
		fireEvent.click(screen.getByRole("button", { name: "Update password" }));

		expect(await screen.findByText("Reset link expired")).toBeInTheDocument();
	});
});
