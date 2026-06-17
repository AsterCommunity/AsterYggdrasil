import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { useAuthStore } from "@/stores/authStore";
import type { AuthSessionInfo, AuthSessionPage } from "@/types/api";
import { LoginDevicesSection } from "./LoginDevicesSection";

const authServiceMock = vi.hoisted(() => ({
	revokeOtherSessions: vi.fn(),
	revokeSession: vi.fn(),
	sessions: vi.fn(),
	sessionsPage: vi.fn(),
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

function session(overrides: Partial<AuthSessionInfo> = {}): AuthSessionInfo {
	const now = Date.now();
	return {
		created_at: new Date(now - 60_000).toISOString(),
		id: "session-1",
		ip_address: "127.0.0.1",
		is_current: false,
		last_seen_at: new Date(now - 30_000).toISOString(),
		refresh_expires_at: new Date(now + 86_400_000).toISOString(),
		revoked: false,
		user_agent: "TestBrowser/1.0",
		...overrides,
	};
}

function activeSessions() {
	return [
		session({
			id: "current-session",
			is_current: true,
			user_agent: "CurrentBrowser/1.0",
		}),
		session({
			id: "other-session",
			is_current: false,
			user_agent: "OtherBrowser/1.0",
		}),
	];
}

function sessionsPage(
	items: AuthSessionInfo[],
	total = items.length,
): AuthSessionPage {
	return {
		items,
		limit: 5,
		offset: 0,
		total,
	};
}

describe("LoginDevicesSection", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		useAuthStore.setState({
			checking: false,
			isAuthenticated: true,
		});
		authServiceMock.sessions.mockResolvedValue(activeSessions());
		authServiceMock.sessionsPage.mockResolvedValue(
			sessionsPage(activeSessions()),
		);
		authServiceMock.revokeSession.mockResolvedValue(undefined);
		authServiceMock.revokeOtherSessions.mockResolvedValue({ removed: 1 });
	});

	it("shows a toast after revoking another login session", async () => {
		render(<LoginDevicesSection />);

		expect(await screen.findByText("OtherBrowser/1.0")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Revoke" }));

		await waitFor(() =>
			expect(authServiceMock.revokeSession).toHaveBeenCalledWith(
				"other-session",
			),
		);
		expect(toastMock.success).toHaveBeenCalledWith("Session revoked");
	});

	it("loads login devices with 5 sessions per page and exposes 5 or 10 page sizes", async () => {
		authServiceMock.sessionsPage.mockResolvedValue(
			sessionsPage(activeSessions(), 12),
		);
		render(<LoginDevicesSection />);

		expect(await screen.findByText("OtherBrowser/1.0")).toBeInTheDocument();
		expect(authServiceMock.sessionsPage).toHaveBeenCalledWith({
			limit: 5,
			offset: 0,
		});

		const pageSize = screen.getByRole("combobox", { name: "Page size" });
		expect(pageSize).toHaveTextContent("5 / page");
		fireEvent.click(pageSize);
		expect(
			await screen.findByRole("option", { name: "5 / page" }),
		).toBeVisible();
		expect(screen.getByRole("option", { name: "10 / page" })).toBeVisible();
	});

	it("shows a toast after revoking all other login sessions", async () => {
		render(<LoginDevicesSection />);

		expect(await screen.findByText("OtherBrowser/1.0")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Revoke others" }));

		await waitFor(() =>
			expect(authServiceMock.revokeOtherSessions).toHaveBeenCalled(),
		);
		expect(toastMock.success).toHaveBeenCalledWith("Other sessions revoked");
	});

	it("shows a toast before clearing auth after revoking the current session", async () => {
		render(<LoginDevicesSection />);

		expect(await screen.findByText("CurrentBrowser/1.0")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "Revoke current" }));

		await waitFor(() =>
			expect(authServiceMock.revokeSession).toHaveBeenCalledWith(
				"current-session",
			),
		);
		expect(toastMock.success).toHaveBeenCalledWith("Current session revoked");
		expect(useAuthStore.getState().isAuthenticated).toBe(false);
	});
});
