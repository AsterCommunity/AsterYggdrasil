import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UserDetailPanel } from "@/components/admin/admin-users-page/UserDetailPanel";
import { i18next } from "@/i18n";
import type { AdminUserInfo } from "@/types/api";

const adminMinecraftProfileServiceMock = vi.hoisted(() => ({
	listByUserPage: vi.fn(),
	listTextures: vi.fn(),
}));

vi.mock("@/services/adminService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/adminService")>();
	return {
		...actual,
		adminMinecraftProfileService: adminMinecraftProfileServiceMock,
	};
});

const user = {
	active_session_count: 2,
	created_at: "2026-06-15T00:00:00.000Z",
	email: "alex@example.com",
	email_verified_at: null,
	id: 7,
	must_change_password: false,
	operator_scopes: [],
	profile: {
		avatar: { source: "none", url_1024: null, url_512: null, version: 0 },
		display_name: "Alex",
	},
	profile_count: 3,
	role: "user",
	session_version: 1,
	status: "active",
	updated_at: "2026-06-15T01:00:00.000Z",
	username: "alex",
} satisfies AdminUserInfo;

function renderPanel(overrides: Partial<AdminUserInfo> = {}) {
	const onBack = vi.fn();
	const onRevokeSessions = vi.fn().mockResolvedValue(2);
	const onUpdate = vi.fn().mockResolvedValue(undefined);

	render(
		<MemoryRouter>
			<UserDetailPanel
				user={{ ...user, ...overrides }}
				onBack={onBack}
				onRevokeSessions={onRevokeSessions}
				onUpdate={onUpdate}
			/>
		</MemoryRouter>,
	);

	return {
		onBack,
		onRevokeSessions,
		onUpdate,
	};
}

function LocationStateProbe() {
	const location = useLocation();
	return <div>{JSON.stringify(location.state)}</div>;
}

describe("UserDetailPanel", () => {
	beforeEach(async () => {
		vi.clearAllMocks();
		await i18next.changeLanguage("zh-CN");
		adminMinecraftProfileServiceMock.listByUserPage.mockResolvedValue({
			items: [],
			limit: 5,
			offset: 0,
			total: 0,
		});
		adminMinecraftProfileServiceMock.listTextures.mockResolvedValue([]);
	});

	it("renders page detail sections without internal session version", () => {
		renderPanel();

		expect(screen.getByText("用户详情")).toBeInTheDocument();
		expect(screen.getByText("账号资料")).toBeInTheDocument();
		expect(screen.getByText("安全操作")).toBeInTheDocument();
		expect(screen.getByText("@alex · alex@example.com")).toBeInTheDocument();
		expect(screen.getByText("角色档案")).toBeInTheDocument();
		expect(screen.getByText("活跃会话")).toBeInTheDocument();
		expect(screen.queryByText("会话版本")).not.toBeInTheDocument();
		expect(screen.getByRole("button", { name: "保存更改" })).toBeDisabled();
	});

	it("renders localized role and status select values plus the configured avatar", async () => {
		renderPanel({
			id: 1,
			profile: {
				avatar: {
					source: "custom",
					url_1024: "/admin/avatars/users/7/1024",
					url_512: "/admin/avatars/users/7/512",
					version: 2,
				},
				display_name: "ESAP",
			},
			role: "admin",
			status: "active",
		});

		expect(screen.getByRole("img", { name: "ESAP" })).toHaveAttribute(
			"src",
			expect.stringContaining("/api/v1/admin/avatars/users/7/512"),
		);
		expect(screen.getByText("@alex · alex@example.com")).toBeInTheDocument();
		expect(
			within(screen.getByRole("combobox", { name: "角色" })).getByText(
				"管理员",
			),
		).toBeInTheDocument();
		expect(
			within(screen.getByRole("combobox", { name: "状态" })).getByText("活跃"),
		).toBeInTheDocument();
		expect(screen.queryByText("admin")).not.toBeInTheDocument();
		expect(screen.queryByText("active")).not.toBeInTheDocument();
	});

	it("loads minecraft profiles with offset pagination", async () => {
		adminMinecraftProfileServiceMock.listByUserPage
			.mockResolvedValueOnce({
				items: Array.from({ length: 5 }, (_, index) => ({
					id: `profile-${index + 1}`,
					name: `角色 ${index + 1}`,
				})),
				limit: 5,
				offset: 0,
				total: 6,
			})
			.mockResolvedValueOnce({
				items: [{ id: "profile-6", name: "角色 6" }],
				limit: 5,
				offset: 5,
				total: 6,
			});

		renderPanel();

		expect(await screen.findByText("角色 1")).toBeInTheDocument();
		expect(
			adminMinecraftProfileServiceMock.listByUserPage,
		).toHaveBeenCalledWith(7, { limit: 5, offset: 0 });
		fireEvent.click(screen.getByRole("button", { name: "下一页" }));

		expect(await screen.findByText("角色 6")).toBeInTheDocument();
		expect(
			adminMinecraftProfileServiceMock.listByUserPage,
		).toHaveBeenLastCalledWith(7, { limit: 5, offset: 5 });
		expect(screen.getByText("共 6 条 · 第 2 / 2 页")).toBeInTheDocument();
	});

	it("renders minecraft profile skin avatars from admin texture metadata", async () => {
		adminMinecraftProfileServiceMock.listByUserPage.mockResolvedValueOnce({
			items: [
				{ id: "profile-one", name: "角色 1" },
				{ id: "profile-two", name: "角色 2" },
			],
			limit: 5,
			offset: 0,
			total: 2,
		});
		adminMinecraftProfileServiceMock.listTextures.mockImplementation(
			(uuid: string) => {
				if (uuid === "profile-one") {
					return Promise.resolve([
						{
							source: "bound",
							texture_type: "skin",
							url: "/textures/profile-one-skin.png",
						},
					]);
				}
				return Promise.resolve([
					{
						source: "default",
						texture_type: "skin",
						url: "/textures/default-skin.png",
					},
				]);
			},
		);

		renderPanel();

		const firstAvatarImage = await screen.findByTestId(
			"admin-user-profile-avatar-image-profile-one",
		);
		expect(firstAvatarImage).toHaveAttribute(
			"src",
			"/textures/profile-one-skin.png",
		);
		expect(firstAvatarImage).toHaveAttribute("draggable", "false");
		expect(
			screen.getByTestId("admin-user-profile-avatar-profile-two"),
		).toBeInTheDocument();
		expect(
			screen.queryByTestId("admin-user-profile-avatar-image-profile-two"),
		).not.toBeInTheDocument();
		expect(adminMinecraftProfileServiceMock.listTextures).toHaveBeenCalledWith(
			"profile-one",
		);
		expect(adminMinecraftProfileServiceMock.listTextures).toHaveBeenCalledWith(
			"profile-two",
		);
	});

	it("keeps long minecraft profile names and ids inside responsive rows", async () => {
		const longUuid = "16eb7a7fa2124230959738ebe4e1b2d0".repeat(2);
		const longName = "VeryLongMinecraftProfileNameWithoutSpaces".repeat(2);
		adminMinecraftProfileServiceMock.listByUserPage.mockResolvedValueOnce({
			items: [{ id: longUuid, name: longName }],
			limit: 5,
			offset: 0,
			total: 1,
		});

		renderPanel();

		expect(await screen.findByText(longName)).toHaveClass("truncate");
		expect(screen.getByText(longUuid)).toHaveClass("break-all");
		expect(screen.getByText("打开档案").closest("a")).toHaveClass(
			"w-full",
			"sm:w-auto",
		);
	});

	it("opens minecraft profile details with the current user detail as return target", async () => {
		adminMinecraftProfileServiceMock.listByUserPage.mockResolvedValueOnce({
			items: [{ id: "profile-one", name: "角色 1" }],
			limit: 5,
			offset: 0,
			total: 1,
		});

		const onRevokeSessions = vi.fn().mockResolvedValue(0);
		const onUpdate = vi.fn().mockResolvedValue(undefined);
		render(
			<MemoryRouter initialEntries={["/admin/users/7"]}>
				<Routes>
					<Route
						path="/admin/users/:id"
						element={
							<UserDetailPanel
								user={user}
								onRevokeSessions={onRevokeSessions}
								onUpdate={onUpdate}
							/>
						}
					/>
					<Route
						path="/admin/minecraft-profiles/:uuid"
						element={<LocationStateProbe />}
					/>
				</Routes>
			</MemoryRouter>,
		);

		expect(await screen.findByText("角色 1")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("link", { name: "打开档案" }));

		expect(
			await screen.findByText('{"returnTo":"/admin/users/7"}'),
		).toBeInTheDocument();
	});

	it("falls back when the current minecraft profile page becomes empty", async () => {
		adminMinecraftProfileServiceMock.listByUserPage
			.mockResolvedValueOnce({
				items: Array.from({ length: 5 }, (_, index) => ({
					id: `profile-${index + 1}`,
					name: `角色 ${index + 1}`,
				})),
				limit: 5,
				offset: 0,
				total: 6,
			})
			.mockResolvedValueOnce({
				items: [],
				limit: 5,
				offset: 5,
				total: 5,
			})
			.mockResolvedValueOnce({
				items: [{ id: "profile-5", name: "角色 5" }],
				limit: 5,
				offset: 0,
				total: 5,
			});

		renderPanel();

		expect(await screen.findByText("角色 1")).toBeInTheDocument();
		fireEvent.click(screen.getByRole("button", { name: "下一页" }));

		expect(await screen.findByText("角色 5")).toBeInTheDocument();
		expect(
			adminMinecraftProfileServiceMock.listByUserPage,
		).toHaveBeenLastCalledWith(7, { limit: 5, offset: 0 });
		expect(screen.getByText("共 5 条 · 第 1 / 1 页")).toBeInTheDocument();
	});

	it("saves only changed profile fields", async () => {
		const { onUpdate } = renderPanel();

		fireEvent.change(screen.getByDisplayValue("alex"), {
			target: { value: "alex_next" },
		});

		const saveButton = screen.getByRole("button", { name: "保存更改" });
		fireEvent.click(saveButton);

		expect(onUpdate).toHaveBeenCalledWith(7, { username: "alex_next" });
	});

	it("saves operator scope changes without rewriting unchanged profile fields", async () => {
		const { onUpdate } = renderPanel({
			role: "operator",
			operator_scopes: ["users"],
		});

		fireEvent.click(screen.getByRole("switch", { name: "材质库" }));
		fireEvent.click(screen.getByRole("button", { name: "保存更改" }));

		expect(onUpdate).toHaveBeenCalledWith(7, {
			operator_scopes: ["users", "texture_library"],
		});
	});

	it("locks super admin role and status controls", () => {
		renderPanel({ id: 1, role: "admin", status: "active" });

		expect(screen.getByRole("combobox", { name: "角色" })).toBeDisabled();
		expect(screen.getByRole("combobox", { name: "状态" })).toBeDisabled();
	});

	it("saves super admin profile fields without role or status payload", () => {
		const { onUpdate } = renderPanel({
			id: 1,
			role: "admin",
			status: "active",
		});

		fireEvent.change(screen.getByDisplayValue("alex@example.com"), {
			target: { value: "root@example.com" },
		});
		fireEvent.click(screen.getByRole("button", { name: "保存更改" }));

		expect(onUpdate).toHaveBeenCalledWith(1, { email: "root@example.com" });
	});

	it("does not save profile changes when a required field is blank", () => {
		const { onUpdate } = renderPanel();

		fireEvent.change(screen.getByDisplayValue("alex"), {
			target: { value: " " },
		});

		const saveButton = screen.getByRole("button", { name: "保存更改" });
		expect(saveButton).toBeDisabled();
		fireEvent.click(saveButton);

		expect(onUpdate).not.toHaveBeenCalled();
	});

	it("blocks password reset outside the allowed length", () => {
		const { onUpdate } = renderPanel();

		fireEvent.change(screen.getByLabelText("新密码"), {
			target: { value: "1234567" },
		});
		fireEvent.change(screen.getByLabelText("确认密码"), {
			target: { value: "1234567" },
		});
		fireEvent.click(screen.getByRole("button", { name: "重置密码" }));

		expect(screen.getByText("密码长度需为 8-128 个字符。")).toBeInTheDocument();
		expect(onUpdate).not.toHaveBeenCalled();

		fireEvent.change(screen.getByLabelText("新密码"), {
			target: { value: "a".repeat(129) },
		});
		fireEvent.change(screen.getByLabelText("确认密码"), {
			target: { value: "a".repeat(129) },
		});
		fireEvent.click(screen.getByRole("button", { name: "重置密码" }));

		expect(screen.getByText("密码长度需为 8-128 个字符。")).toBeInTheDocument();
		expect(onUpdate).not.toHaveBeenCalled();
	});

	it("blocks password reset when confirmation does not match", () => {
		const { onUpdate } = renderPanel();

		fireEvent.change(screen.getByLabelText("新密码"), {
			target: { value: "12345678" },
		});
		fireEvent.change(screen.getByLabelText("确认密码"), {
			target: { value: "abcdefgh" },
		});
		fireEvent.click(screen.getByRole("button", { name: "重置密码" }));

		expect(screen.getByText("两次输入的密码不一致。")).toBeInTheDocument();
		expect(onUpdate).not.toHaveBeenCalled();
	});

	it("resets password with a password-only update payload", () => {
		const { onUpdate } = renderPanel();

		fireEvent.change(screen.getByLabelText("新密码"), {
			target: { value: "12345678" },
		});
		fireEvent.change(screen.getByLabelText("确认密码"), {
			target: { value: "12345678" },
		});
		fireEvent.click(screen.getByRole("button", { name: "重置密码" }));

		expect(onUpdate).toHaveBeenCalledWith(7, { password: "12345678" });
	});

	it("updates the force password change flag from the security action", () => {
		const { onUpdate } = renderPanel();

		fireEvent.click(
			screen.getByRole("switch", { name: "下次登录必须修改密码" }),
		);

		expect(onUpdate).toHaveBeenCalledWith(7, { must_change_password: true });
	});

	it("can clear an existing force password change flag", () => {
		const { onUpdate } = renderPanel({ must_change_password: true });

		fireEvent.click(
			screen.getByRole("switch", { name: "下次登录必须修改密码" }),
		);

		expect(onUpdate).toHaveBeenCalledWith(7, { must_change_password: false });
	});

	it("revokes sessions from the security action", () => {
		const { onRevokeSessions } = renderPanel();
		const securitySection = screen
			.getByText("安全操作")
			.closest("section") as HTMLElement;

		fireEvent.click(
			within(securitySection).getByRole("button", { name: "注销会话" }),
		);

		expect(onRevokeSessions).toHaveBeenCalledWith(7);
	});

	it("disables session revocation when there are no active sessions", () => {
		renderPanel({ active_session_count: 0 });
		const securitySection = screen
			.getByText("安全操作")
			.closest("section") as HTMLElement;

		expect(
			within(securitySection).getByRole("button", { name: "注销会话" }),
		).toBeDisabled();
	});

	it("navigates back to the user list from the page action", () => {
		const { onBack } = renderPanel();

		fireEvent.click(screen.getByRole("button", { name: "返回用户列表" }));

		expect(onBack).toHaveBeenCalledTimes(1);
	});
});
