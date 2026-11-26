import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { i18next } from "@/i18n";
import {
	installPermissionMessages,
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

function flatLookup(key: string): string {
	const backendT = i18next.getFixedT(null, "permissions_backend");
	const result = backendT(key, {
		keySeparator: "",
		nsSeparator: "\u0000",
		defaultValue: "MISS",
	});
	console.error(
		`FLATPROBE: ${key} -> ${JSON.stringify(result)} | lng-en=${JSON.stringify(i18next.t(key, { ns: "permissions_backend", lng: "en", keySeparator: "", nsSeparator: "\u0000", defaultValue: "MISS" }))} | exists-plain=${JSON.stringify(i18next.exists(key, { ns: "permissions_backend" }))} | exists-flat=${JSON.stringify(i18next.exists(key, { ns: "permissions_backend", keySeparator: "" }))} | hasBundle(en,backend)=${i18next.hasResourceBundle("en", "permissions_backend")}`,
	);
	return result;
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

	it("installs the bundle verbatim and resolves flat keys, prefix pairs, and prototype segments", () => {
		// Direct installation keeps this test independent of the module-level
		// cache left by earlier hook tests (the probe matrix proved the flat
		// namespace resolves all these shapes on i18next 27).
		installPermissionMessages("en", {
			"plugin.__proto__.escape": "evil",
			"com.example.capability": "Short text",
			"com.example.capability.detail": "Detail text",
			"plugin.toString.name": "Proto segment label",
			"admin.users.bans.scope.texture_upload": "Backend override label",
		});

		expect(flatLookup("com.example.capability")).toBe("Short text");
		expect(flatLookup("com.example.capability.detail")).toBe("Detail text");
		expect(flatLookup("plugin.toString.name")).toBe("Proto segment label");
		expect(flatLookup("admin.users.bans.scope.texture_upload")).toBe(
			"Backend override label",
		);
		// The forbidden segment never entered the store.
		expect(flatLookup("plugin.__proto__.escape")).toBe("MISS");
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
