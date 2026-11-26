import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";

const mockedList = vi.hoisted(() => vi.fn());
const mockedLocalizations = vi.hoisted(() => vi.fn());

vi.mock("@/services/permissionService", () => ({
	permissionService: {
		list: mockedList,
		localizations: mockedLocalizations,
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
	beforeEach(() => {
		mockedList.mockReset();
		mockedLocalizations.mockReset();
		mockedLocalizations.mockImplementation(async () => ({
			namespace: "permissions",
			requested_locale: "en",
			resolved_locale: "en",
			revision: "test",
			messages: {
				"admin.users.operatorScope.texture_library": "Texture library",
			},
		}));
	});

	it("renders admin-area switches from the registry", async () => {
		mockedList.mockResolvedValue(await registryFixture());
		const OperatorScopeSelector = await loadSelector();
		render(<OperatorScopeSelector value={[]} onChange={vi.fn()} />);

		expect(
			await screen.findByRole("switch", { name: "Texture library" }),
		).toBeInTheDocument();
	});

	it("shows a retry affordance on failure and recovers on retry", async () => {
		mockedList.mockRejectedValueOnce(new Error("network down"));
		const OperatorScopeSelector = await loadSelector();
		render(<OperatorScopeSelector value={[]} onChange={vi.fn()} />);

		expect(
			await screen.findByText("Failed to load the permission registry."),
		).toBeInTheDocument();
		expect(
			screen.queryByRole("switch", { name: "Texture library" }),
		).not.toBeInTheDocument();

		mockedList.mockResolvedValue(await registryFixture());
		fireEvent.click(screen.getByRole("button", { name: "Retry" }));
		expect(
			await screen.findByRole("switch", { name: "Texture library" }),
		).toBeInTheDocument();
		expect(
			screen.queryByText("Failed to load the permission registry."),
		).not.toBeInTheDocument();
	});

	it("backend-installed labels override the static bundle copy", async () => {
		mockedList.mockResolvedValue(await registryFixture());
		// Deliberately different from the static en-US bundle text so the
		// assertion proves the backend message won.
		mockedLocalizations.mockResolvedValue({
			namespace: "permissions",
			requested_locale: "en",
			resolved_locale: "en",
			revision: "test-override",
			messages: {
				"admin.users.operatorScope.texture_library": "Backend override label",
			},
		});
		const OperatorScopeSelector = await loadSelector();
		render(<OperatorScopeSelector value={[]} onChange={vi.fn()} />);

		expect(
			await screen.findByRole("switch", { name: "Backend override label" }),
		).toBeInTheDocument();
		expect(
			screen.queryByRole("switch", { name: "Texture library" }),
		).not.toBeInTheDocument();
	});

	it("emits removals when a selected switch is toggled off", async () => {
		mockedList.mockResolvedValue(await registryFixture());
		const OperatorScopeSelector = await loadSelector();
		const onChange = vi.fn();
		render(
			<OperatorScopeSelector
				value={["com.astercosm.yggdrasil.permission.admin.texture_library"]}
				onChange={onChange}
			/>,
		);

		const textureSwitch = await screen.findByRole("switch", {
			name: "Texture library",
		});
		// The parent owns `value` and does not round-trip through the mock, so
		// the switch stays checked; one click exercises the removal path.
		fireEvent.click(textureSwitch);
		expect(onChange).toHaveBeenCalledTimes(1);
		expect(onChange).toHaveBeenCalledWith([]);
	});
});
