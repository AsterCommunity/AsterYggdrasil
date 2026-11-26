import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { useAuthStore } from "@/stores/authStore";
import type {
	AccountOverview,
	AccountUserBanInfo,
	AuditLogEntry,
	AuthUserInfo,
} from "@/types/api";
import AccountOverviewPage from "./AccountOverviewPage";

const accountServiceMock = vi.hoisted(() => ({
	listBans: vi.fn(),
	overview: vi.fn(),
}));

vi.mock("@/services/permissionService", () => ({
	permissionService: {
		list: vi.fn().mockImplementation(async () => {
			const { REGISTRY_PERMISSIONS } = await import("@/test/permissions");
			return { permissions: REGISTRY_PERMISSIONS };
		}),
	},
}));

vi.mock("@/services/accountService", () => ({
	accountService: accountServiceMock,
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

const overview: AccountOverview = {
	profile_count: 0,
	recent_activity: [],
};

const activeBan: AccountUserBanInfo = {
	created_at: "2026-06-18T00:00:00.000Z",
	effective: true,
	effective_status: "active",
	expires_at: null,
	id: 11,
	public_reason: "Visible policy reason",
	reason: "Internal fallback reason",
	revoked_at: null,
	scopes: ["com.astercosm.yggdrasil.capability.texture.upload"],
	starts_at: "2026-06-18T00:00:00.000Z",
	status: "active",
	updated_at: "2026-06-18T00:00:00.000Z",
};

function auditEntry(entityName: string): AuditLogEntry {
	return {
		action: "user_login",
		created_at: "2026-06-15T08:00:00.000Z",
		details: null,
		entity_id: 7,
		entity_name: entityName,
		entity_type: "user",
		id: 1,
		ip_address: "127.0.0.1",
		presentation: null,
		user: {
			email: "alex@example.com",
			id: 7,
			role: "user",
			status: "active",
			username: "alex",
		},
		user_agent: "vitest",
		user_id: 7,
	};
}

function renderPage(user: AuthUserInfo) {
	useAuthStore.setState({
		user,
		checking: false,
		error: null,
		expiresAt: Date.now() + 60_000,
		isAuthStale: false,
		isAuthenticated: true,
		isAdmin: user.role === "admin",
	});

	return render(
		<MemoryRouter>
			<AccountOverviewPage />
		</MemoryRouter>,
	);
}

describe("AccountOverviewPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		accountServiceMock.overview.mockResolvedValue(overview);
		accountServiceMock.listBans.mockResolvedValue({
			items: [],
			limit: 8,
			next_cursor: null,
			total: 0,
		});
	});

	it("uses the display name in the welcome hero when one is set", () => {
		renderPage({
			...baseUser,
			profile: {
				...baseUser.profile,
				display_name: "Aster",
			},
		});

		expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
			"Welcome back, Aster",
		);
	});

	it("falls back to the username when the display name is blank", () => {
		renderPage({
			...baseUser,
			profile: {
				...baseUser.profile,
				display_name: "   ",
			},
		});

		expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
			"Welcome back, alex",
		);
	});

	it("keeps long recent activity target text truncated", async () => {
		const longTarget = `account-activity-${"x".repeat(96)}`;
		accountServiceMock.overview.mockResolvedValue({
			...overview,
			recent_activity: [auditEntry(longTarget)],
		});

		renderPage(baseUser);

		expect(await screen.findByText(new RegExp(longTarget))).toHaveClass(
			"truncate",
		);
	});

	it("shows active capability bans without internal admin fields", async () => {
		accountServiceMock.listBans.mockResolvedValue({
			items: [activeBan],
			limit: 8,
			next_cursor: null,
			total: 1,
		});

		renderPage(baseUser);

		expect(await screen.findByText("Capability status")).toBeInTheDocument();
		expect(
			await screen.findByText("1 capability is currently restricted."),
		).toBeInTheDocument();
		expect(
			await screen.findByText("Texture upload and wardrobe"),
		).toBeInTheDocument();
		expect(screen.getByText("Visible policy reason")).toBeInTheDocument();
		expect(
			screen.queryByText("Internal fallback reason"),
		).not.toBeInTheDocument();
		expect(accountServiceMock.listBans).toHaveBeenCalledWith({
			effective_only: true,
			limit: 8,
		});
	});

	it("keeps the overview usable when ban loading fails", async () => {
		accountServiceMock.listBans.mockRejectedValue(new Error("blocked"));

		renderPage(baseUser);

		expect(await screen.findByText("Capability status")).toBeInTheDocument();
		expect(
			screen.getByText("No capability restrictions are currently active."),
		).toBeInTheDocument();
		expect(
			screen.getByText("No account capabilities are restricted."),
		).toBeInTheDocument();
	});
});
