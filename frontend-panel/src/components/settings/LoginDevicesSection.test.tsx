import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { useAuthStore } from "@/stores/authStore";
import { createAuthSession, createAuthSessionPage } from "@/test/fixtures";
import type { AuthSessionInfo } from "@/types/api";
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
	return createAuthSession(overrides);
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
			createAuthSessionPage(activeSessions()),
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
			createAuthSessionPage(activeSessions(), { total: 12 }),
		);
		render(<LoginDevicesSection />);

		expect(await screen.findByText("OtherBrowser/1.0")).toBeInTheDocument();
		expect(authServiceMock.sessionsPage).toHaveBeenCalledWith({
			after_id: undefined,
			after_last_seen_at: undefined,
			limit: 5,
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

	it("shows only active login devices in the settings section", async () => {
		const now = Date.now();
		authServiceMock.sessionsPage.mockResolvedValue(
			createAuthSessionPage([
				session({
					id: "session-active",
					is_current: true,
					user_agent: "ActiveBrowser/1.0",
					created_at: new Date(now - 60_000).toISOString(),
					last_seen_at: new Date(now - 30_000).toISOString(),
				}),
				session({
					id: "session-revoked",
					is_current: false,
					user_agent: "RevokedBrowser/1.0",
					revoked: true,
					created_at: new Date(now - 120_000).toISOString(),
					last_seen_at: new Date(now - 90_000).toISOString(),
				}),
				session({
					id: "session-expired",
					is_current: false,
					user_agent: "ExpiredBrowser/1.0",
					refresh_expires_at: new Date(now - 1_000).toISOString(),
					created_at: new Date(now - 180_000).toISOString(),
					last_seen_at: new Date(now - 150_000).toISOString(),
				}),
			]),
		);

		render(<LoginDevicesSection />);

		expect(await screen.findByText("ActiveBrowser/1.0")).toBeInTheDocument();
		expect(screen.queryByText("RevokedBrowser/1.0")).not.toBeInTheDocument();
		expect(screen.queryByText("ExpiredBrowser/1.0")).not.toBeInTheDocument();
		expect(
			screen.queryByText("sessions.status.revoked"),
		).not.toBeInTheDocument();
		expect(
			screen.queryByText("sessions.status.expired"),
		).not.toBeInTheDocument();
		expect(
			screen.queryByText("sessions.revokedSessionTitle"),
		).not.toBeInTheDocument();
	});
});
