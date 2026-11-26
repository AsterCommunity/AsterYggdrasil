import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { i18next } from "@/i18n";
import "@/i18n";

const mockedList = vi.hoisted(() => vi.fn());

vi.mock("@/services/permissionService", () => ({
	permissionService: {
		list: mockedList,
	},
}));

async function loadSelector() {
	vi.resetModules();
	const module = await import(
		"@/components/admin/admin-users-page/OperatorScopeSelector"
	);
	return module.OperatorScopeSelector;
}

async function registryFixture() {
	const { REGISTRY_PERMISSIONS } = await import("@/test/permissions");
	return { permissions: REGISTRY_PERMISSIONS };
}

describe("OperatorScopeSelector", () => {
	beforeEach(async () => {
		await i18next.changeLanguage("zh-CN");
		mockedList.mockReset();
	});

	it("renders admin-area switches from the registry", async () => {
		mockedList.mockResolvedValue(await registryFixture());
		const OperatorScopeSelector = await loadSelector();
		render(<OperatorScopeSelector value={[]} onChange={vi.fn()} />);

		expect(
			await screen.findByRole("switch", { name: "材质库" }),
		).toBeInTheDocument();
	});

	it("shows a retry affordance on failure and recovers on retry", async () => {
		mockedList.mockRejectedValueOnce(new Error("network down"));
		const OperatorScopeSelector = await loadSelector();
		render(<OperatorScopeSelector value={[]} onChange={vi.fn()} />);

		expect(await screen.findByText("权限列表加载失败。")).toBeInTheDocument();
		expect(
			screen.queryByRole("switch", { name: "材质库" }),
		).not.toBeInTheDocument();

		mockedList.mockResolvedValue(await registryFixture());
		fireEvent.click(screen.getByRole("button", { name: "重试" }));
		expect(
			await screen.findByRole("switch", { name: "材质库" }),
		).toBeInTheDocument();
		expect(screen.queryByText("权限列表加载失败。")).not.toBeInTheDocument();
	});
});
