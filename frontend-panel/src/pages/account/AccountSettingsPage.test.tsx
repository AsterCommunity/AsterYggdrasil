import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useAuthStore } from "@/stores/authStore";
import type {
	AuthSessionPage,
	AuthUserInfo,
	ExternalAuthLinkPage,
} from "@/types/api";
import AccountSettingsPage from "./AccountSettingsPage";

const authServiceMock = vi.hoisted(() => ({
	me: vi.fn(),
	requestEmailChange: vi.fn(),
	resendEmailChange: vi.fn(),
	revokeOtherSessions: vi.fn(),
	revokeSession: vi.fn(),
	sessions: vi.fn(),
	sessionsPage: vi.fn(),
	setAvatarSource: vi.fn(),
	updateProfile: vi.fn(),
	uploadAvatar: vi.fn(),
}));

const externalAuthServiceMock = vi.hoisted(() => ({
	deleteLink: vi.fn(),
	listLinksPage: vi.fn(),
}));

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
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

vi.mock("@/components/settings/SecurityPasskeysSection", () => ({
	SecurityPasskeysSection: () => <div data-testid="passkeys-section" />,
}));

vi.mock("@/components/settings/AvatarCropDialog", () => ({
	AvatarCropDialog: ({
		file,
		onConfirm,
		open,
	}: {
		file: File | null;
		onConfirm: (file: File) => Promise<boolean>;
		open: boolean;
	}) =>
		open ? (
			<button
				type="button"
				onClick={() =>
					void onConfirm(
						file ??
							new File(["cropped"], "cropped.webp", { type: "image/webp" }),
					)
				}
			>
				mock-crop-confirm
			</button>
		) : null,
}));

const baseUser: AuthUserInfo = {
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

const emptyAuthSessionPage: AuthSessionPage = {
	items: [],
	limit: 50,
	offset: 0,
	total: 0,
};

const emptyExternalAuthLinkPage: ExternalAuthLinkPage = {
	items: [],
	limit: 20,
	offset: 0,
	total: 0,
};

function renderPage(
	user: AuthUserInfo = baseUser,
	initialEntry = "/account/settings",
) {
	useAuthStore.setState({
		user,
		checking: false,
		error: null,
		expiresAt: Date.now() + 60_000,
		isAuthStale: false,
		isAuthenticated: true,
		isAdmin: false,
	});

	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<AccountSettingsPage />
		</MemoryRouter>,
	);
}

describe("AccountSettingsPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		authServiceMock.me.mockResolvedValue(baseUser);
		authServiceMock.requestEmailChange.mockResolvedValue({
			...baseUser,
			pending_email: "next@example.com",
		});
		authServiceMock.resendEmailChange.mockResolvedValue({
			message: "sent",
		});
		authServiceMock.revokeOtherSessions.mockResolvedValue({ removed: 0 });
		authServiceMock.revokeSession.mockResolvedValue(undefined);
		authServiceMock.sessions.mockResolvedValue([]);
		authServiceMock.sessionsPage.mockResolvedValue(emptyAuthSessionPage);
		authServiceMock.updateProfile.mockResolvedValue(baseUser.profile);
		authServiceMock.setAvatarSource.mockResolvedValue(baseUser.profile);
		authServiceMock.uploadAvatar.mockResolvedValue(baseUser.profile);
		externalAuthServiceMock.deleteLink.mockResolvedValue(undefined);
		externalAuthServiceMock.listLinksPage.mockResolvedValue(
			emptyExternalAuthLinkPage,
		);
	});

	it("saves display name changes into the shared auth state", async () => {
		const nextProfile = {
			...baseUser.profile,
			display_name: "Aster",
		};
		authServiceMock.updateProfile.mockResolvedValue(nextProfile);
		renderPage();

		fireEvent.change(screen.getByRole("textbox", { name: "Display name" }), {
			target: { value: "Aster" },
		});
		fireEvent.click(screen.getByRole("button", { name: /Save/ }));

		await waitFor(() =>
			expect(authServiceMock.updateProfile).toHaveBeenCalledWith({
				display_name: "Aster",
			}),
		);
		expect(authServiceMock.me).not.toHaveBeenCalled();
		expect(useAuthStore.getState().user?.profile).toEqual(nextProfile);
		expect(toastMock.success).toHaveBeenCalledWith("Profile saved");
	});

	it("switches to Gravatar through the avatar source endpoint", async () => {
		const nextProfile = {
			...baseUser.profile,
			avatar: {
				source: "gravatar",
				url_512: "https://www.gravatar.com/avatar/hash?s=512",
				url_1024: "https://www.gravatar.com/avatar/hash?s=1024",
				version: 1,
			},
		} as const;
		authServiceMock.setAvatarSource.mockResolvedValue(nextProfile);
		renderPage();

		fireEvent.click(
			screen.getByRole("button", {
				name: /Use Gravatar/,
			}),
		);

		await waitFor(() =>
			expect(authServiceMock.setAvatarSource).toHaveBeenCalledWith({
				source: "gravatar",
			}),
		);
		expect(authServiceMock.me).not.toHaveBeenCalled();
		expect(useAuthStore.getState().user?.profile).toEqual(nextProfile);
		expect(toastMock.success).toHaveBeenCalledWith("Avatar source updated");
	});

	it("uploads the cropped avatar file selected by the hidden file input", async () => {
		const nextProfile = {
			...baseUser.profile,
			avatar: {
				source: "upload",
				url_512: "/auth/profile/avatar/512?v=2",
				url_1024: "/auth/profile/avatar/1024?v=2",
				version: 2,
			},
		} as const;
		authServiceMock.uploadAvatar.mockResolvedValue(nextProfile);
		const { container } = renderPage();
		const input = container.querySelector(
			'input[type="file"]',
		) as HTMLInputElement;
		const file = new File(["raw"], "raw.png", { type: "image/png" });

		fireEvent.change(input, { target: { files: [file] } });
		fireEvent.click(screen.getByRole("button", { name: "mock-crop-confirm" }));

		await waitFor(() =>
			expect(authServiceMock.uploadAvatar).toHaveBeenCalledWith(file),
		);
		expect(authServiceMock.me).not.toHaveBeenCalled();
		expect(useAuthStore.getState().user?.profile).toEqual(nextProfile);
		expect(toastMock.success).toHaveBeenCalledWith("Avatar updated");
	});

	it("renders login devices inline in the security section", async () => {
		renderPage();

		expect(
			screen.getByRole("heading", { name: "Login devices" }),
		).toBeInTheDocument();
		expect(
			await screen.findByText("No browser sessions returned"),
		).toBeInTheDocument();
	});

	it("requests an email change and refreshes the shared auth state", async () => {
		const refreshedUser = {
			...baseUser,
			pending_email: "next@example.com",
		};
		authServiceMock.me.mockResolvedValueOnce(refreshedUser);
		renderPage();

		fireEvent.change(screen.getByRole("textbox", { name: "New email" }), {
			target: { value: "  next@example.com  " },
		});
		fireEvent.click(screen.getByRole("button", { name: "Send confirmation" }));

		await waitFor(() =>
			expect(authServiceMock.requestEmailChange).toHaveBeenCalledWith({
				new_email: "next@example.com",
			}),
		);
		expect(authServiceMock.me).toHaveBeenCalledTimes(1);
		expect(useAuthStore.getState().user?.pending_email).toBe(
			"next@example.com",
		);
		expect(toastMock.success).toHaveBeenCalledWith(
			"Confirmation email sent. Check the new inbox.",
		);
	});

	it("blocks email change submission for invalid, current, and pending emails", async () => {
		renderPage({
			...baseUser,
			pending_email: "pending@example.com",
		});

		const input = screen.getByRole("textbox", { name: "New email" });
		const submit = screen.getByRole("button", { name: "Send confirmation" });

		expect(submit).toBeDisabled();
		fireEvent.change(input, { target: { value: "bad-email" } });
		expect(submit).toBeDisabled();

		fireEvent.change(input, { target: { value: "alex@example.com" } });
		expect(submit).toBeDisabled();

		fireEvent.change(input, { target: { value: "pending@example.com" } });
		expect(submit).toBeDisabled();

		fireEvent.change(input, { target: { value: "next@example.com" } });
		expect(submit).not.toBeDisabled();
		expect(authServiceMock.requestEmailChange).not.toHaveBeenCalled();
	});

	it("resends a pending email change confirmation", async () => {
		renderPage({
			...baseUser,
			pending_email: "pending@example.com",
		});

		fireEvent.click(
			screen.getByRole("button", { name: "Resend confirmation" }),
		);

		await waitFor(() =>
			expect(authServiceMock.resendEmailChange).toHaveBeenCalledTimes(1),
		);
		expect(toastMock.success).toHaveBeenCalledWith(
			"Confirmation email resent.",
		);
	});

	it("handles email change confirmation redirects and refreshes the user", async () => {
		const refreshedUser = {
			...baseUser,
			email: "next@example.com",
			pending_email: null,
		};
		authServiceMock.me.mockResolvedValueOnce(refreshedUser);
		renderPage(
			{
				...baseUser,
				pending_email: "next@example.com",
			},
			"/settings/security?contact_verification=email-changed&email=next%40example.com",
		);

		await waitFor(() =>
			expect(toastMock.success).toHaveBeenCalledWith(
				"Email updated to next@example.com.",
			),
		);
		expect(authServiceMock.me).toHaveBeenCalledTimes(1);
		expect(useAuthStore.getState().user?.email).toBe("next@example.com");
	});
});
