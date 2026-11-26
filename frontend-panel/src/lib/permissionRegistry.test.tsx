import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { i18next } from "@/i18n";
import {
	DEFAULT_I18N_NAMESPACE,
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

	it("drops forbidden segments and serves backend text through t()", () => {
		// Direct installation keeps this test independent of the module-level
		// cache state left by earlier hook tests.
		installPermissionMessages("en-US", {
			// Forbidden segment: dropped before installation.
			"plugin.__proto__.escape": "evil",
			"admin.users.bans.scope.texture_upload": "Backend override label",
		});
		expect(
			i18next.t("admin.users.bans.scope.texture_upload", {
				ns: DEFAULT_I18N_NAMESPACE,
				lng: "en-US",
			}),
		).toBe("Backend override label");
		// The forbidden key never entered the tree.
		expect(
			i18next.t("plugin.__proto__.escape", {
				ns: DEFAULT_I18N_NAMESPACE,
				lng: "en-US",
				defaultValue: "MISSING",
			}),
		).toBe("MISSING");
	});

	it("backend-installed labels override the static bundle copy", () => {
		installPermissionMessages("en-US", {
			"admin.users.bans.scope.texture_upload": "Backend override label",
		});
		expect(
			i18next.t("admin.users.bans.scope.texture_upload", {
				ns: DEFAULT_I18N_NAMESPACE,
				lng: "en-US",
			}),
		).toBe("Backend override label");
	});

	it("skips keys that conflict with an installed prefix", () => {
		installPermissionMessages("en-US", {
			"com.example.capability": "Short text",
			"com.example.capability.detail": "Detail text",
		});
		// The backend rejects this shape at construction; the installer's
		// defense keeps the shorter key addressable and drops the longer one.
		expect(
			i18next.t("com.example.capability", {
				ns: DEFAULT_I18N_NAMESPACE,
				lng: "en-US",
			}),
		).toBe("Short text");
		expect(
			i18next.t("com.example.capability.detail", {
				ns: DEFAULT_I18N_NAMESPACE,
				lng: "en-US",
				defaultValue: "MISSING",
			}),
		).toBe("MISSING");
	});
});
