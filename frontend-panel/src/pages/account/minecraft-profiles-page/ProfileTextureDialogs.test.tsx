import { fireEvent, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
	baseProfiles,
	buttonByText,
	cursorPage,
	firstEnabledButton,
	openTextureManager,
	pngFile,
	renderPage,
	texture,
	topDialog,
	uploadedTexture,
} from "@/pages/account/minecraft-profiles-page/testSupport";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const yggdrasilServiceMock = vi.hoisted(() => ({
	bindProfileTexture: vi.fn(),
	createProfile: vi.fn(),
	deleteProfile: vi.fn(),
	listProfileSkinTextureUrls: vi.fn(),
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

describe("MinecraftProfilesPage texture dialogs", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		yggdrasilServiceMock.listProfiles.mockResolvedValue(
			cursorPage(baseProfiles),
		);
		yggdrasilServiceMock.listProfileSkinTextureUrls.mockResolvedValue({
			"profile-one": "/textures/profile-one-skin.png",
			"profile-two": null,
		});
		yggdrasilServiceMock.listProfileTextures.mockResolvedValue([]);
		yggdrasilServiceMock.uploadWardrobeTexture.mockResolvedValue(
			uploadedTexture(),
		);
		yggdrasilServiceMock.bindProfileTexture.mockResolvedValue(texture());
		yggdrasilServiceMock.unbindProfileTexture.mockResolvedValue(undefined);
	});

	it("uploads a skin through the texture dialog and binds it to the selected profile", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfileSkinTextureUrls.mockClear();
		const manager = await openTextureManager("profile-one");

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
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listProfileSkinTextureUrls,
			).toHaveBeenLastCalledWith(["profile-one", "profile-two"]);
		});
	});

	it("uploads a cape from the cape slot without allowing texture type switching", async () => {
		yggdrasilServiceMock.uploadWardrobeTexture.mockResolvedValueOnce(
			uploadedTexture({ texture_type: "cape" }),
		);
		await renderPage();
		const manager = await openTextureManager("profile-one");

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
		const manager = await openTextureManager("profile-one");

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
		const manager = await openTextureManager("profile-one");

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
		const manager = await openTextureManager("profile-one");

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
		const manager = await openTextureManager("profile-one");

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
		const manager = await openTextureManager("profile-one");

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
		yggdrasilServiceMock.listProfileSkinTextureUrls.mockClear();
		const manager = await openTextureManager("profile-one");

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
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listProfileSkinTextureUrls,
			).toHaveBeenLastCalledWith(["profile-one", "profile-two"]);
		});
	});
});
