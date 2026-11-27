import { fireEvent, render, screen, waitFor } from "@testing-library/react";
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
			<span data-testid="bundle-status">{registry.bundleStatus}</span>
			<button
				type="button"
				data-testid="retry-bundle"
				onClick={registry.retryBundle}
			>
				retry-bundle
			</button>
			<span data-testid="count">{registry.permissions.length}</span>
		</div>
	);
}

function Mount() {
	return <Probe registry={useRegistryPermissions("capability")} />;
}

function DoubleMount() {
	const a = useRegistryPermissions("capability");
	const b = useRegistryPermissions("capability");
	return (
		<div>
			<span data-testid="a-bundle">{a.bundleStatus}</span>
			<span data-testid="b-bundle">{b.bundleStatus}</span>
			<button type="button" data-testid="a-retry" onClick={a.retryBundle}>
				a-retry
			</button>
		</div>
	);
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
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});
		expect(mockedLocalizations).toHaveBeenCalledTimes(1);
		expect(screen.getByTestId("count")).toHaveTextContent("5");
		unmount();

		render(<Mount />);
		expect(screen.getByTestId("status")).toHaveTextContent("ready");
		expect(screen.getByTestId("count")).toHaveTextContent("5");
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

	it("keeps descriptors available when the localization bundle fails", async () => {
		await i18next.changeLanguage("en-US");
		mockedLocalizations.mockRejectedValue(new Error("bundle down"));

		const { unmount } = render(<Mount />);
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});
		expect(screen.getByTestId("count")).toHaveTextContent("5");
		expect(screen.getByTestId("bundle-status")).toHaveTextContent("error");
		unmount();

		// The bundle-only retry path serves a cached-descriptor consumer.
		mockedLocalizations.mockResolvedValue({
			namespace: "permissions",
			requested_locale: "en-US",
			resolved_locale: "en",
			revision: "test",
			messages: {
				"admin.users.bans.scope.texture_upload": "Bundle retry label",
			},
		});
		render(<Mount />);
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});
		expect(flatLookup("admin.users.bans.scope.texture_upload")).toBe(
			"Bundle retry label",
		);
	});

	it("retryBundle restores localized labels after a failure", async () => {
		await i18next.changeLanguage("en-US");
		mockedLocalizations.mockRejectedValueOnce(new Error("bundle down"));
		mockedLocalizations.mockResolvedValue({
			namespace: "permissions",
			requested_locale: "en-US",
			resolved_locale: "en",
			revision: "test",
			messages: { "admin.users.bans.scope.texture_upload": "Retry label" },
		});

		const { unmount } = render(<Mount />);
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});
		expect(screen.getByTestId("bundle-status")).toHaveTextContent("error");

		fireEvent.click(screen.getByTestId("retry-bundle"));
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});
		expect(flatLookup("admin.users.bans.scope.texture_upload")).toBe(
			"Retry label",
		);
		unmount();
	});

	it("discards a stale bundle response after a language switch", async () => {
		await i18next.changeLanguage("en-US");
		let enRequests = 0;
		let releaseEn: (value: {
			namespace: string;
			requested_locale: string;
			resolved_locale: string;
			revision: string;
			messages: Record<string, string>;
		}) => void = () => {};
		mockedLocalizations.mockImplementation((locale: string) => {
			if (locale.startsWith("zh")) {
				return Promise.resolve({
					namespace: "permissions",
					requested_locale: locale,
					resolved_locale: "zh",
					revision: "test-zh",
					messages: { "admin.users.bans.scope.texture_upload": "材质上传" },
				});
			}
			enRequests += 1;
			if (enRequests === 1) {
				return Promise.resolve({
					namespace: "permissions",
					requested_locale: locale,
					resolved_locale: "en",
					revision: "test-en-1",
					messages: {
						"admin.users.bans.scope.texture_upload": "Texture upload",
					},
				});
			}
			// A later en request hangs so the test can land it stale.
			return new Promise((resolve) => {
				releaseEn = resolve;
			});
		});

		const { unmount } = render(<Mount />);
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});

		// Hang an en bundle request via the exposed retry...
		fireEvent.click(screen.getByTestId("retry-bundle"));
		expect(await screen.findByTestId("bundle-status")).toHaveTextContent(
			"loading",
		);
		// ...then switch languages, invalidating the in-flight generation.
		await i18next.changeLanguage("zh-CN");
		await waitFor(() => {
			expect(flatLookup("admin.users.bans.scope.texture_upload")).toBe(
				"材质上传",
			);
		});

		// Watch installs directly: a stale response must not call
		// addResourceBundle at all, regardless of what the zh lookup shows.
		const addSpy = vi.spyOn(i18next, "addResourceBundle");
		addSpy.mockClear();
		releaseEn({
			namespace: "permissions",
			requested_locale: "en-US",
			resolved_locale: "en-US",
			revision: "stale",
			messages: { "admin.users.bans.scope.texture_upload": "STALE EN" },
		});
		await new Promise((resolve) => setTimeout(resolve, 50));
		expect(addSpy).not.toHaveBeenCalled();
		addSpy.mockRestore();
		expect(flatLookup("admin.users.bans.scope.texture_upload")).toBe(
			"材质上传",
		);
		unmount();
		await i18next.changeLanguage("en-US");
	});

	it("syncs bundleStatus across mounted consumers when one retries", async () => {
		await i18next.changeLanguage("en-US");
		mockedLocalizations.mockRejectedValueOnce(new Error("bundle down"));
		mockedLocalizations.mockResolvedValue({
			namespace: "permissions",
			requested_locale: "en-US",
			resolved_locale: "en",
			revision: "test",
			messages: { "admin.users.bans.scope.texture_upload": "Retry label" },
		});

		render(<DoubleMount />);
		await waitFor(() => {
			expect(screen.getByTestId("a-bundle")).toHaveTextContent("error");
		});
		expect(screen.getByTestId("b-bundle")).toHaveTextContent("error");

		// A's successful retry broadcasts through the shared cache; B syncs
		// without refetching descriptors.
		fireEvent.click(screen.getByTestId("a-retry"));
		await waitFor(() => {
			expect(screen.getByTestId("a-bundle")).toHaveTextContent("ready");
		});
		expect(screen.getByTestId("b-bundle")).toHaveTextContent("ready");
	});

	it("refetches the localization bundle for a mounted consumer on language change", async () => {
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

		// Keep the consumer mounted across the switch: this pins the
		// language-tick subscription, so a regression where mounted hooks
		// stop refetching cannot pass.
		render(<Mount />);
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});
		expect(mockedLocalizations).toHaveBeenLastCalledWith("en-US");

		await i18next.changeLanguage("zh-CN");
		await waitFor(() => {
			expect(mockedLocalizations).toHaveBeenLastCalledWith("zh-CN");
		});
		await waitFor(() => {
			expect(screen.getByTestId("status")).toHaveTextContent("ready");
		});
		expect(mockedLocalizations).toHaveBeenCalledTimes(2);
		await i18next.changeLanguage("en-US");
	});
});
