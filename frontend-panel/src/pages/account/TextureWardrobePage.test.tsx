import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
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
	listProfiles: vi.fn(),
	listWardrobeTextures: vi.fn(),
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
		file_size: 128,
		hash: "skin-texture-hash-0001",
		height: 64,
		id: 7,
		mime_type: "image/png",
		texture_model: "default",
		texture_type: "skin",
		updated_at: "2026-01-01T00:00:00Z",
		url: "/textures/skin.png",
		visibility: "private",
		width: 64,
		...overrides,
	};
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
		yggdrasilServiceMock.listProfiles.mockResolvedValue(
			profilePage([profile("profile-one", "ProfileOne")]),
		);
		yggdrasilServiceMock.listWardrobeTextures.mockResolvedValue(
			wardrobePage([texture()]),
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
	});

	it("loads skin wardrobe textures through the backend filter and previews them with MinecraftPreview", async () => {
		await renderPage();

		expect(yggdrasilServiceMock.listWardrobeTextures).toHaveBeenCalledWith({
			limit: 10,
			offset: 0,
			texture_type: "skin",
			keyword: undefined,
		});
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"/textures/skin.png",
		);
		expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
			"wardrobe.previewTitle",
		);
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
			});
		});
		await waitFor(() => {
			expect(screen.getByTestId("minecraft-preview")).toHaveTextContent(
				"/textures/search-result.png",
			);
		});
		expect(screen.getByTestId("wardrobe-search-icon")).toBeInTheDocument();
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
				textureType: "skin",
				visibility: "public",
			});
		});
	});

	it("lets users choose 10 or 20 textures per page", async () => {
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

		expect(within(dialog).getByText("ProfileOne")).toBeInTheDocument();
		expect(within(dialog).queryByText("profile-one")).not.toBeInTheDocument();
	});
});
