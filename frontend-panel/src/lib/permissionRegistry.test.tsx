import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { i18next } from "@/i18n";
import {
	type RegistryPermissions,
	useRegistryPermissions,
} from "@/lib/permissionRegistry";

const mockedLocalizations = vi.hoisted(() => vi.fn());

vi.mock("@/services/permissionService", () => ({
	permissionService: {
		list: vi.fn().mockImplementation(async () => {
			const { REGISTRY_PERMISSIONS } = await import("@/test/permissions");
			return { permissions: REGISTRY_PERMISSIONS };
		}),
		localizations: mockedLocalizations,
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
		mockedLocalizations.mockResolvedValue({
			namespace: "permissions",
			requested_locale: "en",
			resolved_locale: "en",
			revision: "test",
			messages: {
				"admin.users.bans.scope.texture_upload": "Texture upload and wardrobe",
			},
		});

		const { unmount } = render(<Mount />);
		expect(await screen.findByText("ready")).toBeInTheDocument();
		expect(mockedLocalizations).toHaveBeenCalledTimes(1);
		expect(screen.getByText("5")).toBeInTheDocument();
		unmount();

		render(<Mount />);
		expect(screen.getByText("ready")).toBeInTheDocument();
		expect(screen.getByText("5")).toBeInTheDocument();
		expect(mockedLocalizations).toHaveBeenCalledTimes(1);
	});

	it("refetches the localization bundle when the language changes", async () => {
		await i18next.changeLanguage("en-US");
		mockedLocalizations.mockImplementation(async (locale: string) => ({
			namespace: "permissions",
			requested_locale: locale,
			resolved_locale: locale.startsWith("zh") ? "zh" : "en",
			revision: `test-${locale}`,
			messages: locale.startsWith("zh")
				? { "admin.users.bans.scope.texture_upload": "材质上传" }
				: { "admin.users.bans.scope.texture_upload": "Texture upload" },
		}));

		const { unmount } = render(<Mount />);
		expect(await screen.findByText("ready")).toBeInTheDocument();
		expect(mockedLocalizations).toHaveBeenLastCalledWith("en-US");
		unmount();

		await i18next.changeLanguage("zh-CN");
		render(<Mount />);
		expect(await screen.findByText("ready")).toBeInTheDocument();
		expect(mockedLocalizations).toHaveBeenLastCalledWith("zh-CN");
		await i18next.changeLanguage("en-US");
	});
});
