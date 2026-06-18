import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AdminMinecraftProfilePage from "@/pages/admin/AdminMinecraftProfilePage";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const adminMinecraftProfileServiceMock = vi.hoisted(() => ({
	delete: vi.fn(),
	deleteTexture: vi.fn(),
	get: vi.fn(),
	listTextures: vi.fn(),
	rename: vi.fn(),
}));

const adminUserServiceMock = vi.hoisted(() => ({
	get: vi.fn(),
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

vi.mock("@/hooks/useApiError", () => ({
	handleApiError: (error: unknown) => {
		const message = error instanceof Error ? error.message : String(error);
		toastMock.error(message);
	},
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

vi.mock("@/components/yggdrasil/MinecraftPreview", () => ({
	MinecraftPreview: ({
		capeUrl,
		label,
		model,
		playerName,
		skinUrl,
	}: {
		capeUrl?: string | null;
		label: string;
		model?: "default" | "slim";
		playerName?: string | null;
		skinUrl?: string | null;
	}) => (
		<div
			data-testid="minecraft-preview"
			data-cape-url={capeUrl ?? ""}
			data-model={model ?? ""}
			data-player-name={playerName ?? ""}
			data-skin-url={skinUrl ?? ""}
		>
			{label}
		</div>
	),
}));

const baseProfile = {
	created_at: "2026-06-15T00:00:00Z",
	id: 7,
	name: "AdminOld",
	texture_model: "default" as const,
	updated_at: "2026-06-15T00:00:00Z",
	uploadable_textures: "skin,cape",
	user_id: 1,
	uuid: "profile-uuid",
};

const ownerUser = {
	active_session_count: 1,
	created_at: "2026-06-14T00:00:00Z",
	email: "owner@example.com",
	email_verified_at: null,
	id: 1,
	must_change_password: false,
	pending_email: null,
	profile: {
		avatar: {
			source: "custom",
			url_1024: "/admin/avatars/users/1/1024",
			url_512: "/admin/avatars/users/1/512",
			version: 1,
		},
		display_name: "Owner Display",
	},
	profile_count: 1,
	role: "admin",
	session_version: 1,
	status: "active",
	updated_at: "2026-06-14T01:00:00Z",
	username: "owner",
};

function texture(overrides: Record<string, unknown> = {}) {
	return {
		created_at: "2026-06-15T00:00:00Z",
		file_size: 128,
		hash: "texture-hash",
		height: 64,
		id: 7,
		mime_type: "image/png",
		profile_id: 7,
		profile_name: "AdminOld",
		profile_uuid: "profile-uuid",
		source: "bound",
		texture_model: "default",
		texture_type: "skin",
		updated_at: "2026-06-15T00:00:00Z",
		url: "/textures/skin.png",
		visibility: "private",
		width: 64,
		...overrides,
	};
}

function renderPage(
	initialEntry:
		| string
		| {
				pathname: string;
				state?: Record<string, unknown>;
		  } = "/admin/minecraft-profiles/profile-uuid",
) {
	return render(
		<MemoryRouter initialEntries={[initialEntry]}>
			<Routes>
				<Route
					path="/admin/minecraft-profiles/:uuid"
					element={<AdminMinecraftProfilePage />}
				/>
				<Route path="/admin/users" element={<div>users page</div>} />
				<Route path="/admin/users/:id" element={<div>user detail page</div>} />
			</Routes>
		</MemoryRouter>,
	);
}

describe("AdminMinecraftProfilePage rename workflow", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		adminMinecraftProfileServiceMock.get.mockResolvedValue(baseProfile);
		adminMinecraftProfileServiceMock.listTextures.mockResolvedValue([]);
		adminUserServiceMock.get.mockResolvedValue(ownerUser);
		adminMinecraftProfileServiceMock.rename.mockResolvedValue({
			...baseProfile,
			name: "AdminNew",
			updated_at: "2026-06-15T00:01:00Z",
		});
	});

	it("renames an admin profile and refreshes the page state", async () => {
		renderPage();

		await screen.findByRole("heading", { level: 1, name: "AdminOld" });
		fireEvent.click(
			screen.getByRole("button", {
				name: "admin.minecraftProfilePage.renameAction",
			}),
		);

		const dialog = screen.getByRole("dialog");
		const input = within(dialog).getByLabelText(
			"admin.minecraftProfilePage.profileName",
		);
		expect(input).toHaveValue("AdminOld");
		fireEvent.change(input, { target: { value: " AdminNew " } });
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(adminMinecraftProfileServiceMock.rename).toHaveBeenCalledWith(
				"profile-uuid",
				{ name: "AdminNew" },
			);
		});
		expect(
			await screen.findByRole("heading", { level: 1, name: "AdminNew" }),
		).toBeInTheDocument();
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.minecraftProfilePage.renameSuccess",
		);
	});

	it("uses the shared preview panel with bound skin and cape textures", async () => {
		adminMinecraftProfileServiceMock.listTextures.mockResolvedValueOnce([
			texture({ texture_type: "skin", url: "/textures/skin.png" }),
			texture({
				hash: "cape-hash",
				id: 8,
				texture_type: "cape",
				url: "/textures/cape.png",
			}),
		]);

		renderPage();

		const preview = await screen.findByTestId("minecraft-preview");
		expect(preview).toHaveTextContent("admin.minecraftProfilePage.preview");
		expect(preview).toHaveAttribute("data-player-name", "AdminOld");
		expect(preview).toHaveAttribute("data-skin-url", "/textures/skin.png");
		expect(preview).toHaveAttribute("data-cape-url", "/textures/cape.png");
		expect(preview).toHaveAttribute("data-model", "default");
		expect(
			screen.getByText("admin.minecraftProfilePage.recordTitle"),
		).toBeInTheDocument();
		expect(
			screen.getByText("admin.minecraftProfilePage.textureList"),
		).toBeInTheDocument();
	});

	it("shows the owner user like the admin users table identity cell", async () => {
		renderPage();

		await screen.findByRole("heading", { level: 1, name: "AdminOld" });
		expect(adminUserServiceMock.get).toHaveBeenCalledWith(1);
		expect(
			document.querySelector('img[src*="/api/v1/admin/avatars/users/1/512"]'),
		).toBeInTheDocument();
		expect(screen.getByText("Owner Display")).toBeInTheDocument();
		expect(screen.getByText("@owner · #1")).toBeInTheDocument();
		expect(screen.getByRole("link", { name: /Owner Display/ })).toHaveAttribute(
			"href",
			"/admin/users/1",
		);
	});

	it("formats profile timestamps instead of showing raw ISO strings", async () => {
		renderPage();

		await screen.findByRole("heading", { level: 1, name: "AdminOld" });
		expect(screen.queryByText(baseProfile.created_at)).not.toBeInTheDocument();
		expect(screen.queryByText(baseProfile.updated_at)).not.toBeInTheDocument();
		for (const time of screen.getAllByTitle(baseProfile.created_at)) {
			expect(time).toHaveAttribute("datetime", baseProfile.created_at);
		}
	});

	it("uses the bound skin model for slim profile previews", async () => {
		adminMinecraftProfileServiceMock.listTextures.mockResolvedValueOnce([
			texture({
				texture_model: "slim",
				texture_type: "skin",
				url: "/textures/slim-skin.png",
			}),
		]);

		renderPage();

		expect(await screen.findByTestId("minecraft-preview")).toHaveAttribute(
			"data-model",
			"slim",
		);
	});

	it("opens profile deletion from the preview detail panel", async () => {
		renderPage();

		await screen.findByRole("heading", { level: 1, name: "AdminOld" });
		fireEvent.click(
			screen.getByRole("button", {
				name: "admin.minecraftProfilePage.deleteProfileAction",
			}),
		);

		expect(
			screen.getByRole("dialog", {
				name: "admin.minecraftProfilePage.deleteTitle",
			}),
		).toBeInTheDocument();
	});

	it("returns to the owner user detail page when opened from a user", async () => {
		renderPage({
			pathname: "/admin/minecraft-profiles/profile-uuid",
			state: { returnTo: "/admin/users/1" },
		});

		await screen.findByRole("heading", { level: 1, name: "AdminOld" });
		fireEvent.click(
			screen.getByRole("link", {
				name: "admin.minecraftProfilePage.backToOwnerUser",
			}),
		);

		expect(await screen.findByText("user detail page")).toBeInTheDocument();
	});

	it("keeps long profile labels constrained on narrow layouts", async () => {
		const longName = "AdminProfileWithAnAbsurdlyLongUnbrokenNameForMobile";
		const longUuid = "16eb7a7fa2124230959738ebe4e1b2d0";
		adminMinecraftProfileServiceMock.get.mockResolvedValueOnce({
			...baseProfile,
			name: longName,
			uuid: longUuid,
		});

		renderPage();

		expect(
			await screen.findByRole("heading", { level: 1, name: longName }),
		).toHaveClass("break-words");
		expect(
			screen.getByRole("heading", { level: 2, name: longName }),
		).toHaveClass("break-words");
		for (const uuidText of screen.getAllByText(longUuid)) {
			expect(uuidText).toHaveClass("break-all");
		}
		expect(
			screen.getByRole("button", {
				name: "admin.minecraftProfilePage.renameAction",
			}),
		).toHaveClass("w-full", "sm:w-auto");
	});

	it("does not submit admin rename when cancelled or blank", async () => {
		renderPage();

		await screen.findByRole("heading", { level: 1, name: "AdminOld" });
		fireEvent.click(
			screen.getByRole("button", {
				name: "admin.minecraftProfilePage.renameAction",
			}),
		);

		let dialog = screen.getByRole("dialog");
		fireEvent.change(
			within(dialog).getByLabelText("admin.minecraftProfilePage.profileName"),
			{
				target: { value: "" },
			},
		);
		expect(
			within(dialog).getByRole("button", { name: "common.save" }),
		).toBeDisabled();
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.cancel" }),
		);

		await waitFor(() => {
			expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
		});
		expect(adminMinecraftProfileServiceMock.rename).not.toHaveBeenCalled();

		fireEvent.click(
			screen.getByRole("button", {
				name: "admin.minecraftProfilePage.renameAction",
			}),
		);
		dialog = screen.getByRole("dialog");
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.cancel" }),
		);

		expect(adminMinecraftProfileServiceMock.rename).not.toHaveBeenCalled();
	});

	it("keeps admin rename dialog open and surfaces API errors", async () => {
		adminMinecraftProfileServiceMock.rename.mockRejectedValueOnce(
			new Error("profile name already exists"),
		);
		renderPage();

		await screen.findByRole("heading", { level: 1, name: "AdminOld" });
		fireEvent.click(
			screen.getByRole("button", {
				name: "admin.minecraftProfilePage.renameAction",
			}),
		);
		const dialog = screen.getByRole("dialog");
		fireEvent.change(
			within(dialog).getByLabelText("admin.minecraftProfilePage.profileName"),
			{
				target: { value: "TakenName" },
			},
		);
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith(
				"profile name already exists",
			);
		});
		expect(screen.getByRole("dialog")).toBeInTheDocument();
	});
});
