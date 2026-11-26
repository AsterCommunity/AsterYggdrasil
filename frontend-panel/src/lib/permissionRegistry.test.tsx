import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "@/i18n";
import {
	type RegistryPermissions,
	useRegistryPermissions,
} from "@/lib/permissionRegistry";
import { REGISTRY_PERMISSIONS } from "@/test/permissions";

const mockedList = vi.hoisted(() => vi.fn());

vi.mock("@/services/permissionService", () => ({
	permissionService: {
		list: mockedList,
	},
}));

function Probe({ registry }: { registry: RegistryPermissions }) {
	return (
		<div>
			<span data-testid="status">{registry.status}</span>
			<span data-testid="count">{registry.permissions.length}</span>
		</div>
	);
}

function Mount() {
	return <Probe registry={useRegistryPermissions("capability")} />;
}

describe("useRegistryPermissions", () => {
	it("serves remounts from the process cache without refetching", async () => {
		mockedList.mockResolvedValue({ permissions: REGISTRY_PERMISSIONS });

		const { unmount } = render(<Mount />);
		expect(await screen.findByText("ready")).toBeInTheDocument();
		expect(mockedList).toHaveBeenCalledTimes(1);
		expect(screen.getByText("5")).toBeInTheDocument();
		unmount();

		render(<Mount />);
		expect(screen.getByText("ready")).toBeInTheDocument();
		expect(screen.getByText("5")).toBeInTheDocument();
		expect(mockedList).toHaveBeenCalledTimes(1);
	});
});
