import { render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { i18next } from "@/i18n";
import { createAdminUser, createUserBanPage } from "@/test/fixtures";
import AdminUserDetailPage from "./AdminUserDetailPage";

const adminUserServiceMock = vi.hoisted(() => ({
	get: vi.fn(),
	listBans: vi.fn(),
	revokeSessions: vi.fn(),
	update: vi.fn(),
}));

const adminMinecraftProfileServiceMock = vi.hoisted(() => ({
	listByUserPage: vi.fn(),
}));

vi.mock("@/services/adminService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/adminService")>();
	return {
		...actual,
		adminMinecraftProfileService: adminMinecraftProfileServiceMock,
		adminUserService: adminUserServiceMock,
	};
});

const user = createAdminUser({
	id: 1,
	username: "esap",
	email: "esaps@esaps.net",
	email_verified_at: null,
	role: "admin",
	profile_count: 1,
	profile: {
		avatar: { source: "none", url_1024: null, url_512: null, version: 0 },
		display_name: "AptS:1547",
	},
});

function renderPage() {
	return render(
		<MemoryRouter initialEntries={["/admin/users/1"]}>
			<Routes>
				<Route path="/admin/users/:id" element={<AdminUserDetailPage />} />
				<Route path="/admin/users" element={<div>users page</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

describe("AdminUserDetailPage", () => {
	beforeEach(async () => {
		vi.clearAllMocks();
		await i18next.changeLanguage("zh-CN");
		adminUserServiceMock.get.mockResolvedValue(user);
		adminUserServiceMock.listBans.mockResolvedValue(createUserBanPage([]));
		adminUserServiceMock.revokeSessions.mockResolvedValue({ removed: 0 });
		adminUserServiceMock.update.mockResolvedValue(user);
		adminMinecraftProfileServiceMock.listByUserPage.mockResolvedValue({
			items: [],
			limit: 5,
			total: 0,
		});
	});

	it("uses the account username in the page title", async () => {
		renderPage();

		expect(
			await screen.findByRole("heading", { name: "用户 esap" }),
		).toBeInTheDocument();
		expect(screen.getByText("AptS:1547")).toBeInTheDocument();
		expect(screen.getByText("@esap · esaps@esaps.net")).toBeInTheDocument();
	});
});
