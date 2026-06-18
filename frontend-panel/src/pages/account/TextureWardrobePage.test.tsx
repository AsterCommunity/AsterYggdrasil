import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import TextureWardrobePage from "@/pages/account/TextureWardrobePage";
import type {
	MinecraftWardrobeTextureMetadata,
	MinecraftWardrobeTexturePage,
	YggdrasilProfile,
	YggdrasilProfilePage,
} from "@/types/api";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const yggdrasilServiceMock = vi.hoisted(() => ({
	bindProfileTexture: vi.fn(),
	deleteWardrobeTexture: vi.fn(),
	listTextureLibraryTagsPage: vi.fn(),
	listProfiles: vi.fn(),
	listWardrobeTextures: vi.fn(),
	replaceWardrobeTextureTags: vi.fn(),
	updateWardrobeTexture: vi.fn(),
	uploadWardrobeTexture: vi.fn(),
}));

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		i18n: { language: "en-US" },
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

vi.mock("@/components/yggdrasil/MinecraftPreview", () => ({
	MinecraftPreview: ({
		capeUrl,
		label,
		playerName,
		skinUrl,
	}: {
		capeUrl?: string | null;
		label: string;
		playerName?: string | null;
		skinUrl?: string | null;
	}) => (
		<div data-testid="minecraft-preview">
			<span>{label}</span>
			<span>{playerName}</span>
			<span>{skinUrl}</span>
			<span>{capeUrl}</span>
		</div>
	),
}));

function profile(id: string, name: string): YggdrasilProfile {
	return { id, name, properties: [] };
}

function texture(
	overrides: Partial<MinecraftWardrobeTextureMetadata> = {},
): MinecraftWardrobeTextureMetadata {
	return {
		created_at: "2026-01-01T00:00:00Z",
		display_name: null,
		file_size: 128,
		hash: "skin-texture-hash-0001",
		height: 64,
		id: 7,
		library_status: "private",
		mime_type: "image/png",
		name: "skin-texture-has",
		tags: [],
		texture_model: "default",
		texture_type: "skin",
		updated_at: "2026-01-01T00:00:00Z",
		url: "/textures/skin.png",
		visibility: "private",
		width: 64,
		...overrides,
	};
}

function textureTags() {
	return [
		{
			color: "#228855",
			created_at: "2026-01-01T00:00:00Z",
			id: 3,
			name: "Featured",
			sort_order: 1,
			updated_at: "2026-01-01T00:00:00Z",
		},
		{
			color: "#334455",
			created_at: "2026-01-01T00:00:00Z",
			id: 4,
			name: "Classic",
			sort_order: 2,
			updated_at: "2026-01-01T00:00:00Z",
		},
	];
}

function manyTextureTags(count = 25) {
	return Array.from({ length: count }, (_, index) => ({
		color: index === count - 1 ? "#663399" : "#334455",
		created_at: "2026-01-01T00:00:00Z",
		id: index + 1,
		name: index === count - 1 ? "Rare Nether" : `Classic ${index + 1}`,
		sort_order: index + 1,
		updated_at: "2026-01-01T00:00:00Z",
	}));
}

function rareTextureTag() {
	return manyTextureTags().at(-1) ?? textureTags()[0];
}

function profilePage(items: YggdrasilProfile[]): YggdrasilProfilePage {
	return { items, limit: 10, offset: 0, total: items.length };
}

function wardrobePage(
	items: MinecraftWardrobeTextureMetadata[],
	total = items.length,
): MinecraftWardrobeTexturePage {
	return { items, limit: 10, offset: 0, total };
}

function tagPage(items = textureTags(), total = items.length) {
	return { items, limit: 30, offset: 0, total };
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

async function renderPage() {
	render(<TextureWardrobePage />);
	await screen.findByTestId("minecraft-preview");
}

function topDialog() {
	const dialog = screen
		.getAllByRole("dialog", { hidden: true })
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

describe("TextureWardrobePage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		vi.useRealTimers();
		yggdrasilServiceMock.listProfiles.mockResolvedValue(
			profilePage([profile("profile-one", "ProfileOne")]),
		);
		yggdrasilServiceMock.listWardrobeTextures.mockResolvedValue(
			wardrobePage([texture()]),
		);
		yggdrasilServiceMock.listTextureLibraryTagsPage.mockResolvedValue(
			tagPage(),
		);
		yggdrasilServiceMock.uploadWardrobeTexture.mockResolvedValue(
			texture({
				hash: "uploaded-texture-hash",
				id: 9,
				url: "/textures/uploaded.png",
			}),
		);
		yggdrasilServiceMock.bindProfileTexture.mockResolvedValue(undefined);
		yggdrasilServiceMock.deleteWardrobeTexture.mockResolvedValue(undefined);
		yggdrasilServiceMock.updateWardrobeTexture.mockResolvedValue(
			texture({
				display_name: "Edited Skin",
				id: 7,
				name: "Edited Skin",
				visibility: "public",
			}),
		);
		yggdrasilServiceMock.replaceWardrobeTextureTags.mockResolvedValue(
			texture({
				display_name: "Edited Skin",
				id: 7,
				name: "Edited Skin",
				tags: [textureTags()[0]],
				visibility: "public",
			}),
		);
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("loads skin wardrobe textures through the backend filter and previews them with MinecraftPreview", async () => {
		await renderPage();

		expect(yggdrasilServiceMock.listWardrobeTextures).toHaveBeenCalledWith({
			limit: 10,
			offset: 0,
			texture_type: "skin",
			keyword: undefined,
			tag_ids: undefined,
			tag_search_method: undefined,
		});
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"/textures/skin.png",
		);
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"wardrobe.previewTitle",
		);
		const privateVisibilityBadges = screen.getAllByText(
			"wardrobe.visibility.private",
		);
		expect(privateVisibilityBadges.length).toBeGreaterThan(0);
		expect(privateVisibilityBadges[0]).toHaveClass("bg-muted/70");
		const previewImage = document.querySelector(
			'img[src="/textures/skin.png"]',
		);
		expect(previewImage).toHaveAttribute("crossorigin", "anonymous");
	});

	it("switches to cape filtering and passes the cape URL into MinecraftPreview", async () => {
		yggdrasilServiceMock.listWardrobeTextures
			.mockResolvedValueOnce(wardrobePage([texture()]))
			.mockResolvedValueOnce(
				wardrobePage([
					texture({
						hash: "cape-texture-hash-0001",
						height: 17,
						id: 8,
						texture_type: "cape",
						url: "/textures/cape.png",
						width: 22,
					}),
				]),
			);
		await renderPage();

		fireEvent.click(screen.getByRole("button", { name: "wardrobe.type.cape" }));

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listWardrobeTextures,
			).toHaveBeenLastCalledWith({
				limit: 10,
				offset: 0,
				texture_type: "cape",
				keyword: undefined,
				tag_ids: undefined,
				tag_search_method: undefined,
			});
		});
		await waitFor(() => {
			expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
				"/textures/cape.png",
			);
		});
	});

	it("searches wardrobe textures on the backend after debounce and shows loading feedback", async () => {
		await renderPage();
		expect(yggdrasilServiceMock.listProfiles).not.toHaveBeenCalled();
		yggdrasilServiceMock.listWardrobeTextures.mockResolvedValueOnce(
			wardrobePage([
				texture({
					hash: "searched-texture-hash",
					id: 12,
					url: "/textures/search-result.png",
				}),
			]),
		);

		fireEvent.change(
			screen.getByPlaceholderText("wardrobe.searchPlaceholder"),
			{
				target: { value: "searched" },
			},
		);

		expect(screen.getByTestId("wardrobe-search-spinner")).toBeInTheDocument();
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listWardrobeTextures,
			).toHaveBeenLastCalledWith({
				limit: 10,
				offset: 0,
				texture_type: "skin",
				keyword: "searched",
				tag_ids: undefined,
				tag_search_method: undefined,
			});
		});
		await waitFor(() => {
			expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
				"/textures/search-result.png",
			);
		});
		expect(yggdrasilServiceMock.listProfiles).not.toHaveBeenCalled();
		expect(screen.getByTestId("wardrobe-search-icon")).toBeInTheDocument();
	});

	it("filters wardrobe textures by administrator tags with all-match semantics", async () => {
		await renderPage();
		yggdrasilServiceMock.listWardrobeTextures.mockResolvedValue(
			wardrobePage([
				texture({
					id: 13,
					name: "Tagged Skin",
					tags: textureTags(),
					url: "/textures/tagged.png",
				}),
			]),
		);

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.tagFilterButton" }),
		);
		expect(
			screen.getByTestId("wardrobe-tag-filter-popover"),
		).toBeInTheDocument();
		await waitFor(() => {
			expect(
				screen.getByRole("checkbox", { name: /Featured/ }),
			).toBeInTheDocument();
		});
		fireEvent.click(screen.getByRole("checkbox", { name: /Featured/ }));
		fireEvent.click(screen.getByRole("checkbox", { name: /Classic/ }));

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listWardrobeTextures,
			).toHaveBeenLastCalledWith({
				limit: 10,
				offset: 0,
				texture_type: "skin",
				keyword: undefined,
				tag_ids: [3, 4],
				tag_search_method: "all",
			});
		});
		await waitFor(() => {
			expect(screen.getAllByText("Tagged Skin").length).toBeGreaterThan(0);
		});
		expect(screen.getByText("wardrobe.tagFilterHint.all")).toBeInTheDocument();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.tagSearchMethod.any" }),
		);
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listWardrobeTextures,
			).toHaveBeenLastCalledWith({
				limit: 10,
				offset: 0,
				texture_type: "skin",
				keyword: undefined,
				tag_ids: [3, 4],
				tag_search_method: "any",
			});
		});
	});

	it("closes the tag filter popover on outside click or Escape and debounces tag search", async () => {
		yggdrasilServiceMock.listTextureLibraryTagsPage
			.mockResolvedValueOnce(tagPage(manyTextureTags()))
			.mockResolvedValueOnce(tagPage([rareTextureTag()], 1));
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.tagFilterButton" }),
		);
		expect(
			screen.getByTestId("wardrobe-tag-filter-popover"),
		).toBeInTheDocument();
		expect(
			screen.queryByText(/wardrobe\.tagSearchMore/),
		).not.toBeInTheDocument();
		const popover = screen.getByTestId("wardrobe-tag-filter-popover");
		await waitFor(() => {
			expect(within(popover).getByText("Rare Nether")).toBeInTheDocument();
		});

		fireEvent.change(
			within(popover).getByPlaceholderText("wardrobe.tagSearchPlaceholder"),
			{
				target: { value: "rare" },
			},
		);
		expect(within(popover).getByText("Rare Nether")).toBeInTheDocument();

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listTextureLibraryTagsPage,
			).toHaveBeenLastCalledWith({
				limit: 30,
				offset: 0,
				keyword: "rare",
			});
		});
		await waitFor(() => {
			expect(within(popover).getByText("Rare Nether")).toBeInTheDocument();
			expect(screen.queryByText("Classic 1")).not.toBeInTheDocument();
		});

		fireEvent.keyDown(document, { key: "Escape" });
		expect(
			screen.queryByTestId("wardrobe-tag-filter-popover"),
		).not.toBeInTheDocument();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.tagFilterButton" }),
		);
		expect(
			screen.getByTestId("wardrobe-tag-filter-popover"),
		).toBeInTheDocument();
		fireEvent.pointerDown(document.body);
		expect(
			screen.queryByTestId("wardrobe-tag-filter-popover"),
		).not.toBeInTheDocument();
	});

	it("shows an empty tag picker when admins have not created tags", async () => {
		yggdrasilServiceMock.listTextureLibraryTagsPage.mockResolvedValue(
			tagPage([]),
		);
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.editAction" }),
		);

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listTextureLibraryTagsPage,
			).toHaveBeenLastCalledWith({
				limit: 30,
				offset: 0,
				keyword: undefined,
			});
		});
		await waitFor(() => {
			expect(
				within(topDialog()).getByText("wardrobe.noAvailableTags"),
			).toBeInTheDocument();
		});
	});

	it("searches edit tags with paged server results", async () => {
		yggdrasilServiceMock.listTextureLibraryTagsPage
			.mockResolvedValueOnce(tagPage(manyTextureTags(), 25))
			.mockResolvedValueOnce(tagPage([rareTextureTag()], 1))
			.mockResolvedValueOnce(tagPage([], 0));
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.editAction" }),
		);
		const dialog = topDialog();
		await waitFor(() => {
			expect(within(dialog).getByText("Rare Nether")).toBeInTheDocument();
		});
		expect(
			within(dialog).getByPlaceholderText("wardrobe.tagSearchPlaceholder"),
		).toBeInTheDocument();
		expect(
			within(dialog).queryByText(/wardrobe\.tagSearchMore/),
		).not.toBeInTheDocument();

		fireEvent.change(
			within(dialog).getByPlaceholderText("wardrobe.tagSearchPlaceholder"),
			{ target: { value: "rare" } },
		);

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listTextureLibraryTagsPage,
			).toHaveBeenLastCalledWith({
				limit: 30,
				offset: 0,
				keyword: "rare",
			});
		});
		await waitFor(() => {
			expect(within(dialog).getByText("Rare Nether")).toBeInTheDocument();
			expect(within(dialog).queryByText("Classic 1")).not.toBeInTheDocument();
		});

		fireEvent.change(
			within(dialog).getByPlaceholderText("wardrobe.tagSearchPlaceholder"),
			{ target: { value: "missing" } },
		);
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listTextureLibraryTagsPage,
			).toHaveBeenLastCalledWith({
				limit: 30,
				offset: 0,
				keyword: "missing",
			});
		});
		await waitFor(() => {
			expect(
				within(dialog).getByText("wardrobe.noTagSearchResults"),
			).toBeInTheDocument();
		});
	});

	it("validates uploaded files before calling the wardrobe upload API", async () => {
		await renderPage();

		fireEvent.click(screen.getByRole("button", { name: "wardrobe.uploadTab" }));
		const dialog = topDialog();
		fireEvent.change(within(dialog).getByLabelText("profiles.file"), {
			target: { files: [pngFile(63, 64, "bad-skin.png")] },
		});

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith(
				expect.stringContaining("profiles.textureUploadInvalidDimensions"),
			);
		});
		expect(yggdrasilServiceMock.uploadWardrobeTexture).not.toHaveBeenCalled();

		const validFile = pngFile();
		fireEvent.click(
			within(dialog).getByRole("radio", {
				name: "wardrobe.visibility.public",
			}),
		);
		fireEvent.change(within(dialog).getByLabelText("profiles.file"), {
			target: { files: [validFile] },
		});
		await within(dialog).findAllByText("profiles.selectedFileLabel");
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.upload" }),
		);

		await waitFor(() => {
			expect(yggdrasilServiceMock.uploadWardrobeTexture).toHaveBeenCalledWith({
				file: validFile,
				model: "default",
				name: "",
				textureType: "skin",
				visibility: "public",
			});
		});
	});

	it("uploads an optional texture name and shows backend fallback names", async () => {
		await renderPage();

		expect(screen.getAllByText("skin-texture-has").length).toBeGreaterThan(0);
		fireEvent.click(screen.getByRole("button", { name: "wardrobe.uploadTab" }));
		const dialog = topDialog();
		fireEvent.change(within(dialog).getByLabelText("profiles.file"), {
			target: { files: [pngFile()] },
		});
		await within(dialog).findAllByText("profiles.selectedFileLabel");
		fireEvent.change(within(dialog).getByLabelText("wardrobe.textureName"), {
			target: { value: "  Blue Jacket  " },
		});
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.upload" }),
		);

		await waitFor(() => {
			expect(yggdrasilServiceMock.uploadWardrobeTexture).toHaveBeenCalledWith(
				expect.objectContaining({
					name: "  Blue Jacket  ",
				}),
			);
		});
	});

	it("passes blank upload names through as optional input and keeps backend fallback labels", async () => {
		const uploaded = texture({
			display_name: null,
			hash: "blank-upload-hash-0001",
			id: 10,
			name: "blank-upload-has",
			url: "/textures/blank-upload.png",
		});
		yggdrasilServiceMock.uploadWardrobeTexture.mockResolvedValueOnce(uploaded);
		yggdrasilServiceMock.listWardrobeTextures
			.mockResolvedValueOnce(wardrobePage([texture()]))
			.mockResolvedValueOnce(wardrobePage([uploaded]));
		await renderPage();

		fireEvent.click(screen.getByRole("button", { name: "wardrobe.uploadTab" }));
		const dialog = topDialog();
		fireEvent.change(within(dialog).getByLabelText("profiles.file"), {
			target: { files: [pngFile()] },
		});
		await within(dialog).findAllByText("profiles.selectedFileLabel");
		fireEvent.change(within(dialog).getByLabelText("wardrobe.textureName"), {
			target: { value: "   " },
		});
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.upload" }),
		);

		await waitFor(() => {
			expect(yggdrasilServiceMock.uploadWardrobeTexture).toHaveBeenCalledWith(
				expect.objectContaining({
					name: "   ",
				}),
			);
		});
		await waitFor(() => {
			expect(screen.getAllByText("blank-upload-has").length).toBeGreaterThan(0);
		});
	});

	it("edits wardrobe texture name and visibility through the metadata dialog", async () => {
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.editAction" }),
		);
		const dialog = topDialog();
		expect(
			within(dialog).getByRole("radio", {
				name: "wardrobe.visibility.private",
			}),
		).toBeChecked();
		fireEvent.change(within(dialog).getByLabelText("wardrobe.textureName"), {
			target: { value: "Edited Skin" },
		});
		fireEvent.click(
			within(dialog).getByRole("radio", { name: "wardrobe.visibility.public" }),
		);
		await waitFor(() => {
			expect(
				within(dialog).getByRole("checkbox", { name: /Featured/ }),
			).toBeInTheDocument();
		});
		fireEvent.click(within(dialog).getByRole("checkbox", { name: /Featured/ }));
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(yggdrasilServiceMock.updateWardrobeTexture).toHaveBeenCalledWith(
				7,
				{
					display_name: "Edited Skin",
					visibility: "public",
				},
			);
		});
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.replaceWardrobeTextureTags,
			).toHaveBeenCalledWith(7, {
				tag_ids: [3],
			});
		});
		await waitFor(() => {
			expect(screen.getAllByText("Edited Skin").length).toBeGreaterThan(0);
		});
		expect(screen.getAllByText("wardrobe.visibility.public")[0]).toHaveClass(
			"bg-emerald-500/10",
		);
	});

	it("clears blank edited texture names so the backend fallback is used", async () => {
		yggdrasilServiceMock.listWardrobeTextures.mockResolvedValue(
			wardrobePage([
				texture({
					display_name: "Custom Skin",
					name: "Custom Skin",
					visibility: "public",
				}),
			]),
		);
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.editAction" }),
		);
		const dialog = topDialog();
		fireEvent.change(within(dialog).getByLabelText("wardrobe.textureName"), {
			target: { value: "   " },
		});
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(yggdrasilServiceMock.updateWardrobeTexture).toHaveBeenCalledWith(
				7,
				{
					display_name: null,
					visibility: "public",
				},
			);
		});
	});

	it("keeps the edit dialog open and does not replace local texture data when saving fails", async () => {
		yggdrasilServiceMock.updateWardrobeTexture.mockRejectedValueOnce(
			new Error("save failed"),
		);
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.editAction" }),
		);
		const dialog = topDialog();
		fireEvent.change(within(dialog).getByLabelText("wardrobe.textureName"), {
			target: { value: "Broken Save" },
		});
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith("save failed");
		});
		expect(
			within(topDialog()).getByText("wardrobe.editDialogTitle"),
		).toBeInTheDocument();
		expect(screen.queryByText("Broken Save")).not.toBeInTheDocument();
		expect(screen.getAllByText("skin-texture-has").length).toBeGreaterThan(0);
	});

	it("keeps the edit dialog open when tag replacement fails after metadata saves", async () => {
		yggdrasilServiceMock.replaceWardrobeTextureTags.mockRejectedValueOnce(
			new Error("tag save failed"),
		);
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.editAction" }),
		);
		const dialog = topDialog();
		await waitFor(() => {
			expect(
				within(dialog).getByRole("checkbox", { name: /Featured/ }),
			).toBeInTheDocument();
		});
		fireEvent.click(within(dialog).getByRole("checkbox", { name: /Featured/ }));
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(toastMock.error).toHaveBeenCalledWith("tag save failed");
		});
		expect(
			within(topDialog()).getByText("wardrobe.editDialogTitle"),
		).toBeInTheDocument();
		expect(screen.queryByText("Edited Skin")).not.toBeInTheDocument();
		expect(screen.getAllByText("skin-texture-has").length).toBeGreaterThan(0);
	});

	it("lets users choose 10 or 20 textures per page", async () => {
		yggdrasilServiceMock.listWardrobeTextures.mockReset();
		yggdrasilServiceMock.listWardrobeTextures.mockResolvedValue(
			wardrobePage([texture()], 21),
		);
		await renderPage();

		fireEvent.change(screen.getByLabelText("admin.pagination.pageSize"), {
			target: { value: "20" },
		});

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listWardrobeTextures,
			).toHaveBeenLastCalledWith({
				limit: 20,
				offset: 0,
				texture_type: "skin",
				keyword: undefined,
				tag_ids: undefined,
				tag_search_method: undefined,
			});
		});
		expect(
			screen.getByRole("option", {
				name: 'admin.pagination.pageSizeOption {"count":10}',
			}),
		).toBeInTheDocument();
		expect(
			screen.getByRole("option", {
				name: 'admin.pagination.pageSizeOption {"count":20}',
			}),
		).toBeInTheDocument();
	});

	it("does not expose profile UUIDs in the bind dialog", async () => {
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.bindToProfile" }),
		);
		const dialog = topDialog();

		await waitFor(() => {
			expect(within(dialog).getByText("ProfileOne")).toBeInTheDocument();
		});
		expect(within(dialog).queryByText("profile-one")).not.toBeInTheDocument();
	});
});
