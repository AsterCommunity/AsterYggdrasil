import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UserDetailPanel } from "@/components/admin/admin-users-page/UserDetailPanel";
import { i18next } from "@/i18n";
import type { AdminUserInfo } from "@/types/api";

const user = {
	active_session_count: 2,
	created_at: "2026-06-15T00:00:00.000Z",
	email: "alex@example.com",
	email_verified_at: null,
	id: 7,
	must_change_password: false,
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
		<UserDetailPanel
			user={{ ...user, ...overrides }}
			onBack={onBack}
			onRevokeSessions={onRevokeSessions}
			onUpdate={onUpdate}
		/>,
	);

	return {
		onBack,
		onRevokeSessions,
		onUpdate,
	};
}

describe("UserDetailPanel", () => {
	beforeEach(async () => {
		await i18next.changeLanguage("zh-CN");
	});

	it("renders page detail sections without internal session version", () => {
		renderPanel();

		expect(screen.getByText("用户详情")).toBeInTheDocument();
		expect(screen.getByText("账号资料")).toBeInTheDocument();
		expect(screen.getByText("安全操作")).toBeInTheDocument();
		expect(screen.getByText("alex@example.com")).toBeInTheDocument();
		expect(screen.getByText("角色档案")).toBeInTheDocument();
		expect(screen.getByText("活跃会话")).toBeInTheDocument();
		expect(screen.queryByText("会话版本")).not.toBeInTheDocument();
		expect(screen.getByRole("button", { name: "保存更改" })).toBeDisabled();
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
