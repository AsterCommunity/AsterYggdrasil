import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/stores/authStore";
import { usePublicSession } from "./usePublicSession";

const authServiceMock = vi.hoisted(() => ({
	me: vi.fn().mockResolvedValue(null),
	logout: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
	warning: vi.fn(),
}));

vi.mock("sonner", () => ({ toast: toastMock }));

vi.mock("react-i18next", () => ({
	useTranslation: () => ({ t: (key: string) => key }),
}));

function PublicSessionProbe() {
	const { logout } = usePublicSession();
	return (
		<button type="button" onClick={() => void logout()}>
			sign-out
		</button>
	);
}

describe("usePublicSession", () => {
	beforeEach(() => {
		toastMock.error.mockClear();
		toastMock.success.mockClear();
		toastMock.warning.mockClear();
		authServiceMock.logout.mockClear();
		useAuthStore.setState({ user: null, isAuthenticated: false });
	});

	it("does not warn when logout succeeds", async () => {
		authServiceMock.logout.mockResolvedValueOnce(undefined);
		render(<PublicSessionProbe />);

		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "sign-out" }));
		});

		expect(toastMock.warning).not.toHaveBeenCalled();
		expect(authServiceMock.logout).toHaveBeenCalledTimes(1);
	});

	it("warns when the server revoke fails", async () => {
		authServiceMock.logout.mockRejectedValueOnce(new Error("network gone"));
		render(<PublicSessionProbe />);

		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "sign-out" }));
		});

		expect(toastMock.warning).toHaveBeenCalledWith("shell.logoutIncomplete");
	});
});
