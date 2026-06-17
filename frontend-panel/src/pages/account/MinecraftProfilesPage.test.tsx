import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import MinecraftProfilesPage from "@/pages/account/MinecraftProfilesPage";
import type {
	MinecraftTextureMetadata,
	MinecraftWardrobeTextureMetadata,
	YggdrasilProfile,
	YggdrasilProfilePage,
} from "@/types/api";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const yggdrasilServiceMock = vi.hoisted(() => ({
	bindProfileTexture: vi.fn(),
	createProfile: vi.fn(),
	deleteProfile: vi.fn(),
	listProfileTextures: vi.fn(),
	listProfiles: vi.fn(),
	renameProfile: vi.fn(),
	unbindProfileTexture: vi.fn(),
	uploadWardrobeTexture: vi.fn(),
}));

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (
			key: string,
			values?: Record<string, string | number | null | undefined>,
		) => {
			const suffix =
				values && Object.keys(values).length > 0
					? ` ${JSON.stringify(values)}`
					: "";
			return `${key}${suffix}`;
		},
	}),
}));

vi.mock("sonner", () => ({
	toast: toastMock,
}));

vi.mock("@/services/yggdrasilService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/yggdrasilService")>();
	return {
		...actual,
		yggdrasilService: yggdrasilServiceMock,
	};
});

vi.mock("@/components/ui/tooltip", () => ({
	Tooltip: ({ children }: { children: React.ReactNode }) => (
		<div data-testid="tooltip">{children}</div>
	),
	TooltipContent: ({ children }: { children: React.ReactNode }) => (
		<span role="tooltip">{children}</span>
	),
	TooltipProvider: ({ children }: { children: React.ReactNode }) => (
		<>{children}</>
	),
	TooltipTrigger: ({
		children,
		render,
	}: {
		children: React.ReactNode;
		render: React.ReactElement;
	}) => render ?? <>{children}</>,
}));

vi.mock("@/components/yggdrasil/MinecraftPreview", () => ({
	MinecraftPreview: ({
		label,
		playerName,
	}: {
		label: string;
		playerName?: string | null;
	}) => (
		<div data-testid="minecraft-preview">
			<span>{label}</span>
			<span>{playerName}</span>
		</div>
	),
}));

function profile(id: string, name: string): YggdrasilProfile {
	return { id, name, properties: [] };
}

function texture(
	overrides: Partial<MinecraftTextureMetadata> = {},
): MinecraftTextureMetadata {
	return {
		file_size: 128,
		hash: "texture-hash",
		height: 64,
		id: 7,
		mime_type: "image/png",
		profile_id: 1,
		profile_name: "Profile One",
		profile_uuid: "profile-one",
		source: "bound",
		texture_model: "default",
		texture_type: "skin",
		updated_at: "2026-01-01T00:00:00Z",
		created_at: "2026-01-01T00:00:00Z",
		visibility: "private",
		url: "/texture.png",
		width: 64,
		...overrides,
	};
}

function uploadedTexture(
	overrides: Partial<MinecraftWardrobeTextureMetadata> = {},
): MinecraftWardrobeTextureMetadata {
	return {
		created_at: "2026-01-01T00:00:00Z",
		file_size: 128,
		hash: "uploaded-texture-hash",
		height: 64,
		id: 42,
		texture_model: "default",
		texture_type: "skin",
		url: "/uploaded.png",
		width: 64,
		...overrides,
	};
}

function pngFile(width = 64, height = 64, name = "skin.png") {
	const bytes = new Uint8Array(24);
	bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0);
	bytes.set([0, 0, 0, 13], 8);
	bytes.set([0x49, 0x48, 0x44, 0x52], 12);
	const view = new DataView(bytes.buffer);
	view.setUint32(16, width);
	view.setUint32(20, height);
	return new File([bytes], name, { type: "image/png" });
}

function offsetPage(
	items: YggdrasilProfile[],
	limit = 5,
	offset = 0,
	total = items.length,
): YggdrasilProfilePage {
	return { items, limit, offset, total };
}

const baseProfiles = [
	profile("profile-one", "OldName"),
	profile("profile-two", "SecondName"),
];

async function renderPage() {
	render(<MinecraftProfilesPage />);
	await screen.findByTestId("profile-textures-action-profile-one");
}

function rowFor(name: string) {
	const nameCell = screen
		.getAllByText(name)
		.find((element) => element.closest("button"));
	expect(nameCell).toBeDefined();
	const row = (nameCell as HTMLElement).closest("button")?.closest(".grid");
	expect(row).not.toBeNull();
	return row as HTMLElement;
}

function queryTableRowFor(name: string) {
	const nameCell = screen
		.queryAllByText(name)
		.find((element) => element.closest("button"));
	return nameCell?.closest("button")?.closest(".grid") ?? null;
}

function tableHeader() {
	const nameHeader = screen
		.getAllByText("profiles.profileName")
		.find((element) => element.tagName.toLowerCase() === "span");
	expect(nameHeader).toBeDefined();
	const header = (nameHeader as HTMLElement).closest(".grid");
	expect(header).not.toBeNull();
	return header as HTMLElement;
}

function dialogs() {
	return screen.getAllByRole("dialog", { hidden: true });
}

function topDialog() {
	const dialog = dialogs()
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

function firstEnabledButton(container: HTMLElement, name: RegExp) {
	const button = within(container)
		.getAllByRole("button", { hidden: true, name })
		.find((element) => !element.hasAttribute("disabled"));
	expect(button).toBeDefined();
	return button as HTMLElement;
}

function buttonByText(container: HTMLElement, text: string) {
	const button = within(container).getByText(text).closest("button");
	expect(button).not.toBeNull();
	return button as HTMLButtonElement;
}

function openTextureManager(profileId = "profile-one") {
	fireEvent.click(screen.getByTestId(`profile-textures-action-${profileId}`));
	return topDialog();
}

function openRenameDialog(profileId = "profile-one") {
	fireEvent.click(screen.getByTestId(`profile-rename-action-${profileId}`));
	return topDialog();
}

function openDeleteProfileDialog(profileId = "profile-one") {
	fireEvent.click(screen.getByTestId(`profile-delete-action-${profileId}`));
	return topDialog();
}

describe("MinecraftProfilesPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		yggdrasilServiceMock.listProfiles.mockResolvedValue(
			offsetPage(baseProfiles),
		);
		yggdrasilServiceMock.listProfileTextures.mockResolvedValue([]);
		yggdrasilServiceMock.createProfile.mockResolvedValue(
			profile("created-profile", "CreatedName"),
		);
		yggdrasilServiceMock.renameProfile.mockResolvedValue(
			profile("profile-one", "NewName"),
		);
		yggdrasilServiceMock.deleteProfile.mockResolvedValue(undefined);
		yggdrasilServiceMock.uploadWardrobeTexture.mockResolvedValue(
			uploadedTexture(),
		);
		yggdrasilServiceMock.bindProfileTexture.mockResolvedValue(texture());
		yggdrasilServiceMock.unbindProfileTexture.mockResolvedValue(undefined);
	});

	it("loads the first page with default page size 5 and selects the first profile", async () => {
		await renderPage();

		expect(yggdrasilServiceMock.listProfiles).toHaveBeenCalledWith({
			limit: 5,
			offset: 0,
		});
		await waitFor(() => {
			expect(yggdrasilServiceMock.listProfileTextures).toHaveBeenCalledWith(
				"profile-one",
			);
		});
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"OldName",
		);
		expect(
			screen.getByText(/admin\.pagination\.entriesPage/),
		).toBeInTheDocument();
		expect(
			screen.getByText('admin.pagination.pageSizeOption {"count":5}'),
		).toBeInTheDocument();
	});

	it("keeps the left table scoped to profile names and row actions", async () => {
		await renderPage();

		const header = tableHeader();
		expect(within(header).getByText("common.actions")).toBeVisible();
		expect(screen.queryByText("profiles.uuid")).not.toBeInTheDocument();
		expect(screen.queryByText("profiles.textures")).not.toBeInTheDocument();
		expect(screen.queryByText("profile-one")).not.toBeInTheDocument();
		expect(
			screen.queryByText("profiles.totalProfiles"),
		).not.toBeInTheDocument();

		const firstRow = rowFor("OldName");
		expect(
			within(firstRow).getByTestId("profile-textures-action-profile-one"),
		).toBeVisible();
		expect(
			within(firstRow).getByTestId("profile-rename-action-profile-one"),
		).toBeVisible();
		expect(
			within(firstRow).getByTestId("profile-delete-action-profile-one"),
		).toBeVisible();
		expect(
			screen.getAllByText("profiles.manageTexturesAction")[0],
		).toHaveAttribute("role", "tooltip");
		expect(
			screen.getAllByText("profiles.renameShortAction")[0],
		).toHaveAttribute("role", "tooltip");
		expect(
			screen.getAllByText("profiles.deleteProfileAction")[0],
		).toHaveAttribute("role", "tooltip");
	});

	it("searches profiles on the server after debounce and shows the empty state", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			offsetPage([baseProfiles[1]], 5, 0, 1),
		);

		fireEvent.change(
			screen.getByPlaceholderText("profiles.searchPlaceholder"),
			{
				target: { value: "second" },
			},
		);
		expect(screen.getByTestId("profile-search-spinner")).toBeInTheDocument();
		await waitFor(() => {
			expect(yggdrasilServiceMock.listProfiles).toHaveBeenLastCalledWith({
				limit: 5,
				offset: 0,
				query: "second",
			});
		});
		await waitFor(() => {
			expect(queryTableRowFor("OldName")).not.toBeInTheDocument();
			expect(rowFor("SecondName")).toBeInTheDocument();
		});
		expect(screen.getByTestId("profile-search-icon")).toBeInTheDocument();
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"SecondName",
		);

		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			offsetPage([], 5, 0, 0),
		);
		fireEvent.change(
			screen.getByPlaceholderText("profiles.searchPlaceholder"),
			{
				target: { value: "missing" },
			},
		);
		await waitFor(() => {
			expect(yggdrasilServiceMock.listProfiles).toHaveBeenLastCalledWith({
				limit: 5,
				offset: 0,
				query: "missing",
			});
		});
		expect(
			await screen.findByText("profiles.noSearchResults"),
		).toBeInTheDocument();
		expect(screen.queryByText("SecondName")).not.toBeInTheDocument();
	});

	it("shows the empty profile state without pagination when the account has no profiles", async () => {
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(offsetPage([]));

		render(<MinecraftProfilesPage />);

		expect(await screen.findByText("profiles.noProfiles")).toBeInTheDocument();
		expect(
			screen.getByText("profiles.noProfilesDescription"),
		).toBeInTheDocument();
		expect(
			screen.queryByText(/admin\.pagination\.entriesPage/),
		).not.toBeInTheDocument();
		expect(yggdrasilServiceMock.listProfileTextures).not.toHaveBeenCalled();
	});

	it("creates a profile, reloads the first page, and selects the created profile", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			offsetPage(
				[profile("created-profile", "CreatedName"), ...baseProfiles],
				5,
				0,
				3,
			),
		);

		fireEvent.change(screen.getByLabelText("profiles.profileName"), {
			target: { value: "CreatedName" },
		});
		fireEvent.click(screen.getByRole("button", { name: /common.create/ }));

		await waitFor(() => {
			expect(yggdrasilServiceMock.createProfile).toHaveBeenCalledWith({
				name: "CreatedName",
			});
		});
		await waitFor(() => {
			expect(yggdrasilServiceMock.listProfiles).toHaveBeenLastCalledWith({
				limit: 5,
				offset: 0,
			});
		});
		expect(screen.getByLabelText("profiles.profileName")).toHaveValue("");
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"CreatedName",
		);
	});

	it("surfaces create errors and keeps the typed profile name", async () => {
		yggdrasilServiceMock.createProfile.mockRejectedValueOnce(
			new Error("name already exists"),
		);
		await renderPage();

		fireEvent.change(screen.getByLabelText("profiles.profileName"), {
			target: { value: "TakenName" },
		});
		fireEvent.click(screen.getByRole("button", { name: /common.create/ }));

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith("name already exists");
		});
		expect(screen.getByLabelText("profiles.profileName")).toHaveValue(
			"TakenName",
		);
	});

	it("opens texture management from the selected table row without exposing it as a standalone panel", async () => {
		yggdrasilServiceMock.listProfileTextures.mockResolvedValueOnce([
			texture({ texture_type: "skin" }),
			texture({
				hash: "cape-hash",
				id: 8,
				texture_model: "default",
				texture_type: "cape",
			}),
		]);
		await renderPage();

		const dialog = openTextureManager("profile-one");

		expect(
			within(dialog).getByText("profiles.textureTitle"),
		).toBeInTheDocument();
		expect(
			within(dialog).getByText(
				'profiles.textureManageDialogDescription {"name":"OldName"}',
			),
		).toBeInTheDocument();
		expect(screen.getAllByText("profiles.textureTitle")).toHaveLength(1);
		expect(
			within(dialog).getByText("home.textureTypeSkin"),
		).toBeInTheDocument();
		expect(
			within(dialog).getByText("home.textureTypeCape"),
		).toBeInTheDocument();
	});

	it("treats default fallback textures as unbound in the texture manager", async () => {
		yggdrasilServiceMock.listProfileTextures.mockResolvedValueOnce([
			texture({
				id: 0,
				hash: "default-skin-hash",
				source: "default",
				texture_type: "skin",
			}),
		]);
		await renderPage();

		const manager = openTextureManager("profile-one");

		expect(
			within(manager).getByText(
				'profiles.textureSlotDefault {"type":"wardrobe.type.skin"}',
			),
		).toBeInTheDocument();
		expect(
			within(manager).queryByText(
				'profiles.textureSlotReady {"type":"wardrobe.type.skin"}',
			),
		).not.toBeInTheDocument();
		expect(
			within(manager).getAllByRole("button", {
				hidden: true,
				name: /profiles.unbindTextureAction/,
			})[0],
		).toBeDisabled();
		expect(
			within(manager).getAllByRole("button", {
				hidden: true,
				name: /profiles.uploadTextureAction/,
			})[0],
		).toBeEnabled();
	});

	it("uploads a skin through the texture dialog and binds it to the selected profile", async () => {
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			within(manager).getAllByRole("button", {
				name: /profiles.uploadTextureAction/,
			})[0],
		);
		const uploadDialog = topDialog();
		const file = pngFile();
		expect(
			within(uploadDialog).queryByRole("radiogroup", {
				hidden: true,
				name: "profiles.textureType",
			}),
		).not.toBeInTheDocument();
		expect(
			within(uploadDialog).queryByText("home.textureTypeCape"),
		).not.toBeInTheDocument();
		expect(
			within(uploadDialog).getByText(
				'profiles.uploadDialogDescription {"name":"OldName","type":"wardrobe.type.skin"}',
			),
		).toBeInTheDocument();
		fireEvent.change(within(uploadDialog).getByLabelText("profiles.file"), {
			target: { files: [file] },
		});
		await within(uploadDialog).findByText("profiles.selectedFileLabel");
		fireEvent.click(
			within(uploadDialog).getByRole("radio", {
				hidden: true,
				name: "wardrobe.visibility.public",
			}),
		);
		fireEvent.click(buttonByText(uploadDialog, "profiles.uploadAndBindAction"));

		await waitFor(() => {
			expect(yggdrasilServiceMock.uploadWardrobeTexture).toHaveBeenCalledWith({
				file,
				model: "default",
				textureType: "skin",
				visibility: "public",
			});
		});
		expect(yggdrasilServiceMock.bindProfileTexture).toHaveBeenCalledWith({
			textureId: 42,
			textureType: "skin",
			uuid: "profile-one",
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"profiles.uploadAndBindToast",
		);
	});

	it("uploads a cape from the cape slot without allowing texture type switching", async () => {
		yggdrasilServiceMock.uploadWardrobeTexture.mockResolvedValueOnce(
			uploadedTexture({ texture_type: "cape" }),
		);
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			within(manager).getAllByRole("button", {
				name: /profiles.uploadTextureAction/,
			})[1],
		);
		const uploadDialog = topDialog();
		const file = pngFile(64, 32, "cape.png");
		expect(
			within(uploadDialog).queryByRole("radiogroup", {
				hidden: true,
				name: "profiles.textureType",
			}),
		).not.toBeInTheDocument();
		expect(
			within(uploadDialog).queryByText("home.textureTypeSkin"),
		).not.toBeInTheDocument();
		expect(
			within(uploadDialog).getByText(
				'profiles.uploadDialogDescription {"name":"OldName","type":"wardrobe.type.cape"}',
			),
		).toBeInTheDocument();

		fireEvent.change(within(uploadDialog).getByLabelText("profiles.file"), {
			target: { files: [file] },
		});
		await within(uploadDialog).findByText("profiles.selectedFileLabel");
		fireEvent.click(buttonByText(uploadDialog, "profiles.uploadAndBindAction"));

		await waitFor(() => {
			expect(yggdrasilServiceMock.uploadWardrobeTexture).toHaveBeenCalledWith({
				file,
				model: "default",
				textureType: "cape",
				visibility: "private",
			});
		});
		expect(yggdrasilServiceMock.bindProfileTexture).toHaveBeenCalledWith({
			textureId: 42,
			textureType: "cape",
			uuid: "profile-one",
		});
	});

	it("rejects skin-only dimensions when uploading from the cape slot", async () => {
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			within(manager).getAllByRole("button", {
				name: /profiles.uploadTextureAction/,
			})[1],
		);
		const uploadDialog = topDialog();
		fireEvent.change(within(uploadDialog).getByLabelText("profiles.file"), {
			target: { files: [pngFile(64, 64, "skin-shape.png")] },
		});

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith(
				'profiles.textureUploadInvalidDimensions {"width":64,"height":64,"type":"cape"}',
			);
		});
		expect(yggdrasilServiceMock.uploadWardrobeTexture).not.toHaveBeenCalled();
	});

	it("rejects legacy cape dimensions when uploading from the skin slot", async () => {
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			within(manager).getAllByRole("button", {
				name: /profiles.uploadTextureAction/,
			})[0],
		);
		const uploadDialog = topDialog();
		fireEvent.change(within(uploadDialog).getByLabelText("profiles.file"), {
			target: { files: [pngFile(22, 17, "legacy-cape.png")] },
		});

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith(
				'profiles.textureUploadInvalidDimensions {"width":22,"height":17,"type":"skin"}',
			);
		});
		expect(yggdrasilServiceMock.uploadWardrobeTexture).not.toHaveBeenCalled();
	});

	it("uploads a skin through the drag-and-drop area and shows drop hover feedback", async () => {
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			within(manager).getAllByRole("button", {
				name: /profiles.uploadTextureAction/,
			})[0],
		);
		const uploadDialog = topDialog();
		const dropZone = within(uploadDialog)
			.getByText("profiles.fileDropTitle")
			.closest("label");
		expect(dropZone).not.toBeNull();
		const file = pngFile();
		const dataTransfer = {
			files: {
				0: file,
				length: 1,
				item: (index: number) => (index === 0 ? file : null),
			},
		};

		fireEvent.dragEnter(dropZone as HTMLLabelElement, { dataTransfer });
		expect(
			within(uploadDialog).getByText("profiles.fileDropActiveTitle"),
		).toBeInTheDocument();
		fireEvent.dragLeave(dropZone as HTMLLabelElement);
		expect(
			within(uploadDialog).getByText("profiles.fileDropTitle"),
		).toBeInTheDocument();

		fireEvent.dragEnter(dropZone as HTMLLabelElement, { dataTransfer });
		fireEvent.drop(dropZone as HTMLLabelElement, { dataTransfer });
		expect(
			await within(uploadDialog).findByText("profiles.selectedFileLabel"),
		).toBeInTheDocument();
		expect(within(uploadDialog).getByText("skin.png")).toBeInTheDocument();
		fireEvent.click(buttonByText(uploadDialog, "profiles.uploadAndBindAction"));

		await waitFor(() => {
			expect(yggdrasilServiceMock.uploadWardrobeTexture).toHaveBeenCalledWith({
				file,
				model: "default",
				textureType: "skin",
				visibility: "private",
			});
		});
		expect(yggdrasilServiceMock.bindProfileTexture).toHaveBeenCalledWith({
			textureId: 42,
			textureType: "skin",
			uuid: "profile-one",
		});
	});

	it("does not submit texture upload until a file is selected", async () => {
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			within(manager).getAllByRole("button", {
				name: /profiles.uploadTextureAction/,
			})[0],
		);
		const uploadDialog = topDialog();
		expect(
			buttonByText(uploadDialog, "profiles.uploadAndBindAction"),
		).toBeDisabled();
		expect(yggdrasilServiceMock.uploadWardrobeTexture).not.toHaveBeenCalled();
	});

	it("rejects invalid texture dimensions before uploading", async () => {
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			within(manager).getAllByRole("button", {
				name: /profiles.uploadTextureAction/,
			})[0],
		);
		const uploadDialog = topDialog();
		fireEvent.change(within(uploadDialog).getByLabelText("profiles.file"), {
			target: { files: [pngFile(63, 64)] },
		});

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith(
				'profiles.textureUploadInvalidDimensions {"width":63,"height":64,"type":"skin"}',
			);
		});
		expect(
			buttonByText(uploadDialog, "profiles.uploadAndBindAction"),
		).toBeDisabled();
		expect(yggdrasilServiceMock.uploadWardrobeTexture).not.toHaveBeenCalled();
	});

	it("unbinds an existing texture and reloads profile textures", async () => {
		yggdrasilServiceMock.listProfileTextures.mockResolvedValueOnce([
			texture({ id: 7, texture_type: "skin" }),
		]);
		await renderPage();
		const manager = openTextureManager("profile-one");

		fireEvent.click(
			firstEnabledButton(manager, /profiles.unbindTextureAction/),
		);
		const deleteDialog = topDialog();
		fireEvent.click(buttonByText(deleteDialog, "profiles.unbindTextureAction"));

		await waitFor(() => {
			expect(yggdrasilServiceMock.unbindProfileTexture).toHaveBeenCalledWith({
				textureType: "skin",
				uuid: "profile-one",
			});
		});
		expect(toastMock.success).toHaveBeenCalledWith("profiles.deleteSuccess");
		expect(yggdrasilServiceMock.listProfileTextures).toHaveBeenLastCalledWith(
			"profile-one",
		);
	});

	it("renames a profile from the row action, trims whitespace, and reloads the list", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			offsetPage([
				profile("profile-one", "NewName"),
				profile("profile-two", "SecondName"),
			]),
		);
		const dialog = openRenameDialog("profile-one");

		const input = within(dialog).getByLabelText("profiles.profileName");
		expect(input).toHaveValue("OldName");
		fireEvent.change(input, { target: { value: " NewName " } });
		fireEvent.click(
			within(dialog).getByRole("button", { name: /common.save/ }),
		);

		await waitFor(() => {
			expect(yggdrasilServiceMock.renameProfile).toHaveBeenCalledWith(
				"profile-one",
				{ name: "NewName" },
			);
		});
		expect(toastMock.success).toHaveBeenCalledWith("profiles.renameToast");
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"NewName",
		);
	});

	it("keeps rename dialog open on API errors and disables blank submissions", async () => {
		yggdrasilServiceMock.renameProfile.mockRejectedValueOnce(
			new Error("invalid profile name"),
		);
		await renderPage();
		let dialog = openRenameDialog("profile-one");

		fireEvent.change(within(dialog).getByLabelText("profiles.profileName"), {
			target: { value: "   " },
		});
		expect(
			within(dialog).getByRole("button", { name: /common.save/ }),
		).toBeDisabled();
		fireEvent.change(within(dialog).getByLabelText("profiles.profileName"), {
			target: { value: "BadName" },
		});
		fireEvent.click(
			within(dialog).getByRole("button", { name: /common.save/ }),
		);

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith("invalid profile name");
		});
		dialog = screen.getByRole("dialog");
		expect(within(dialog).getByLabelText("profiles.profileName")).toHaveValue(
			"BadName",
		);
	});

	it("deletes the selected profile from a row action and reloads from the first page", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			offsetPage([profile("profile-two", "SecondName")], 5, 0, 1),
		);
		const dialog = openDeleteProfileDialog("profile-one");

		expect(
			within(dialog).getByText(
				'profiles.deleteProfileDescription {"name":"OldName"}',
			),
		).toBeInTheDocument();
		fireEvent.click(
			within(dialog).getByRole("button", {
				name: /profiles.deleteProfileAction/,
			}),
		);

		await waitFor(() => {
			expect(yggdrasilServiceMock.deleteProfile).toHaveBeenCalledWith(
				"profile-one",
			);
		});
		expect(yggdrasilServiceMock.listProfiles).toHaveBeenLastCalledWith({
			limit: 5,
			offset: 0,
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"profiles.deleteProfileToast",
		);
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"SecondName",
		);
	});

	it("keeps the delete dialog open and shows an error when profile deletion fails", async () => {
		yggdrasilServiceMock.deleteProfile.mockRejectedValueOnce(
			new Error("profile is locked"),
		);
		await renderPage();
		const dialog = openDeleteProfileDialog("profile-one");

		fireEvent.click(
			within(dialog).getByRole("button", {
				name: /profiles.deleteProfileAction/,
			}),
		);

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith("profile is locked");
		});
		expect(screen.getByRole("dialog")).toBeInTheDocument();
	});

	it("shows texture load errors without breaking the profile list", async () => {
		yggdrasilServiceMock.listProfileTextures.mockRejectedValueOnce(
			new Error("texture metadata unavailable"),
		);

		await renderPage();

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith(
				"texture metadata unavailable",
			);
		});
		expect(rowFor("OldName")).toBeInTheDocument();
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"OldName",
		);
	});
});
