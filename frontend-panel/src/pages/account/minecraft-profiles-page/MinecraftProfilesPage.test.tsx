import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import MinecraftProfilesPage from "@/pages/account/MinecraftProfilesPage";
import {
	baseProfiles,
	cursorPage,
	openDeleteProfileDialog,
	openRenameDialog,
	openTextureManager,
	profile,
	queryTableRowFor,
	renderPage,
	rowFor,
	tableHeader,
	texture,
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

describe("MinecraftProfilesPage", () => {
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
			after_id: undefined,
			limit: 5,
		});
		await waitFor(() => {
			expect(yggdrasilServiceMock.listProfileTextures).toHaveBeenCalledWith(
				"profile-one",
			);
		});
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listProfileSkinTextureUrls,
			).toHaveBeenCalledWith(["profile-one", "profile-two"]);
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
		await waitFor(() => {
			const avatarImage = within(firstRow).getByTestId(
				"profile-skin-avatar-image-profile-one",
			);
			expect(avatarImage).toHaveAttribute(
				"src",
				"/textures/profile-one-skin.png",
			);
			expect(avatarImage).toHaveAttribute("draggable", "false");
		});
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

		const secondRow = rowFor("SecondName");
		expect(
			within(secondRow).queryByTestId("profile-skin-avatar-image-profile-two"),
		).not.toBeInTheDocument();
	});

	it("falls back to the profile icon when skin avatar metadata fails to load", async () => {
		const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
		yggdrasilServiceMock.listProfileSkinTextureUrls.mockRejectedValueOnce(
			new Error("avatar metadata unavailable"),
		);

		await renderPage();

		await waitFor(() => {
			expect(warnSpy).toHaveBeenCalledWith(
				"Failed to load Minecraft profile skin avatars",
				expect.any(Error),
			);
		});
		expect(rowFor("OldName")).toBeInTheDocument();
		expect(
			within(rowFor("OldName")).queryByTestId(
				"profile-skin-avatar-image-profile-one",
			),
		).not.toBeInTheDocument();
		expect(toastMock.error).not.toHaveBeenCalledWith(
			"avatar metadata unavailable",
		);

		warnSpy.mockRestore();
	});

	it("searches profiles on the server after debounce and shows the empty state", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			cursorPage([baseProfiles[1]], 5, 1),
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
				after_id: undefined,
				limit: 5,
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
			cursorPage([], 5, 0),
		);
		fireEvent.change(
			screen.getByPlaceholderText("profiles.searchPlaceholder"),
			{
				target: { value: "missing" },
			},
		);
		await waitFor(() => {
			expect(yggdrasilServiceMock.listProfiles).toHaveBeenLastCalledWith({
				after_id: undefined,
				limit: 5,
				query: "missing",
			});
		});
		expect(
			await screen.findByText("profiles.noSearchResults"),
		).toBeInTheDocument();
		expect(screen.queryByText("SecondName")).not.toBeInTheDocument();
	});

	it("shows the empty profile state without pagination when the account has no profiles", async () => {
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(cursorPage([]));

		render(<MinecraftProfilesPage />);

		expect(await screen.findByText("profiles.noProfiles")).toBeInTheDocument();
		expect(
			screen.getByText("profiles.noProfilesDescription"),
		).toBeInTheDocument();
		expect(
			screen.queryByText(/admin\.pagination\.entriesPage/),
		).not.toBeInTheDocument();
		expect(yggdrasilServiceMock.listProfileTextures).not.toHaveBeenCalled();
		expect(
			yggdrasilServiceMock.listProfileSkinTextureUrls,
		).not.toHaveBeenCalled();
	});

	it("creates a profile, reloads the first page, and selects the created profile", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			cursorPage(
				[profile("created-profile", "CreatedName"), ...baseProfiles],
				5,
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
				after_id: undefined,
				limit: 5,
			});
		});
		expect(screen.getByLabelText("profiles.profileName")).toHaveValue("");
		await waitFor(() =>
			expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
				"CreatedName",
			),
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

		const dialog = await openTextureManager("profile-one");

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

		const manager = await openTextureManager("profile-one");

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

	it("renames a profile from the row action, trims whitespace, and reloads the list", async () => {
		await renderPage();
		yggdrasilServiceMock.listProfiles.mockResolvedValueOnce(
			cursorPage([
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
			cursorPage([profile("profile-two", "SecondName")], 5, 1),
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
			after_id: undefined,
			limit: 5,
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
