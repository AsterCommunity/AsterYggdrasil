import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import PublicTextureLibraryPage from "@/pages/PublicTextureLibraryPage";
import { useAuthStore } from "@/stores/authStore";
import type {
	MinecraftWardrobeTextureMetadata,
	PublicTextureLibraryPage as PublicTextureLibraryPageData,
	PublicTextureLibraryTextureMetadata,
} from "@/types/api";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const yggdrasilServiceMock = vi.hoisted(() => ({
	copyPublicTextureToWardrobe: vi.fn(),
	listPublicTextureLibraryTags: vi.fn(),
	listPublicTextureLibraryTextures: vi.fn(),
}));

const authServiceMock = vi.hoisted(() => ({
	me: vi.fn(),
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

vi.mock("@/services/authService", () => ({
	authService: authServiceMock,
}));

vi.mock("@/services/yggdrasilService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/yggdrasilService")>();
	return {
		...actual,
		yggdrasilService: yggdrasilServiceMock,
	};
});

vi.mock("@/components/yggdrasil/MinecraftPreviewPanel", () => ({
	MinecraftPreviewPanel: ({
		capeUrl,
		label,
		model,
		playerName,
		skinUrl,
	}: {
		capeUrl?: string | null;
		label: string;
		model?: string;
		playerName?: string | null;
		skinUrl?: string | null;
	}) => (
		<div data-testid="minecraft-preview-panel">
			<span>{label}</span>
			<span>{playerName}</span>
			<span>{model}</span>
			<span>{skinUrl}</span>
			<span>{capeUrl}</span>
		</div>
	),
}));

function publicTexture(
	overrides: Partial<PublicTextureLibraryTextureMetadata> = {},
): PublicTextureLibraryTextureMetadata {
	return {
		created_at: "2026-06-15T00:00:00Z",
		display_name: "Shared Slim",
		file_size: 2048,
		hash: "shared-slim-texture-hash",
		height: 64,
		id: 21,
		library_status: "private",
		mime_type: "image/png",
		name: "Shared Slim",
		tags: [
			{
				color: "#228855",
				created_at: "2026-06-15T00:00:00Z",
				id: 3,
				name: "Featured",
				sort_order: 1,
				updated_at: "2026-06-15T00:00:00Z",
			},
		],
		texture_model: "slim",
		texture_type: "skin",
		updated_at: "2026-06-15T00:00:00Z",
		uploader: {
			name: "Texture Artist",
			public_uuid: "user-public-uuid",
		},
		url: "/textures/shared-slim.png",
		visibility: "public",
		width: 64,
		...overrides,
	};
}

function copiedTexture(
	overrides: Partial<MinecraftWardrobeTextureMetadata> = {},
): MinecraftWardrobeTextureMetadata {
	return {
		created_at: "2026-06-15T00:00:00Z",
		display_name: "Shared Slim",
		file_size: 2048,
		hash: "shared-slim-texture-hash",
		height: 64,
		id: 31,
		library_status: "private",
		mime_type: "image/png",
		name: "Shared Slim",
		tags: [],
		texture_model: "slim",
		texture_type: "skin",
		updated_at: "2026-06-15T00:00:00Z",
		url: "/textures/shared-slim.png",
		visibility: "private",
		width: 64,
		...overrides,
	};
}

function page(
	items: PublicTextureLibraryTextureMetadata[],
	total = items.length,
): PublicTextureLibraryPageData {
	return { items, limit: 12, offset: 0, total };
}

async function renderPage() {
	render(
		<MemoryRouter>
			<PublicTextureLibraryPage />
		</MemoryRouter>,
	);
	await screen.findByText("Shared Slim");
}

describe("PublicTextureLibraryPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		useAuthStore.getState().clear();
		authServiceMock.me.mockRejectedValue(new Error("unauthenticated"));
		yggdrasilServiceMock.listPublicTextureLibraryTextures.mockResolvedValue(
			page([publicTexture()]),
		);
		yggdrasilServiceMock.listPublicTextureLibraryTags.mockResolvedValue({
			items: [
				{
					color: "#228855",
					created_at: "2026-06-15T00:00:00Z",
					id: 3,
					name: "Featured",
					sort_order: 1,
					updated_at: "2026-06-15T00:00:00Z",
				},
			],
			limit: 30,
			offset: 0,
			total: 1,
		});
		yggdrasilServiceMock.copyPublicTextureToWardrobe.mockResolvedValue(
			copiedTexture(),
		);
	});

	it("loads public textures and keeps copy actions out of list cards", async () => {
		await renderPage();

		expect(
			yggdrasilServiceMock.listPublicTextureLibraryTextures,
		).toHaveBeenCalledWith({
			limit: 12,
			offset: 0,
			keyword: undefined,
			tag_ids: undefined,
			tag_search_method: undefined,
			texture_type: undefined,
		});
		expect(
			yggdrasilServiceMock.listPublicTextureLibraryTags,
		).not.toHaveBeenCalled();
		expect(screen.getByText("Texture Artist")).toBeInTheDocument();
		expect(
			screen.queryByText("wardrobe.visibility.public"),
		).not.toBeInTheDocument();
		expect(screen.getByText("64x64")).toBeInTheDocument();
		expect(screen.getByText("2.0 KiB")).toBeInTheDocument();
		expect(
			screen.queryByRole("button", { name: "library.copyAction" }),
		).not.toBeInTheDocument();
		expect(screen.getByRole("link", { name: /Shared Slim/ })).toHaveAttribute(
			"href",
			"/textures/21",
		);
		expect(
			document.querySelector('img[src="/textures/shared-slim.png"]'),
		).toHaveAttribute("crossorigin", "anonymous");
		expect(
			document.querySelector('img[src="/textures/shared-slim.png"]'),
		).toHaveAttribute("draggable", "false");
		expect(
			document.querySelector('img[src="/textures/shared-slim.png"]')
				?.parentElement,
		).toHaveClass("grid", "place-items-center");
	});

	it("searches public textures without reloading the tag list", async () => {
		await renderPage();
		expect(
			yggdrasilServiceMock.listPublicTextureLibraryTags,
		).not.toHaveBeenCalled();

		fireEvent.change(screen.getByPlaceholderText("library.searchPlaceholder"), {
			target: { value: "nj" },
		});
		fireEvent.click(
			screen.getByRole("button", { name: "library.searchAction" }),
		);

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listPublicTextureLibraryTextures,
			).toHaveBeenLastCalledWith({
				limit: 12,
				offset: 0,
				keyword: "nj",
				tag_ids: undefined,
				tag_search_method: undefined,
				texture_type: undefined,
			});
		});
		expect(
			yggdrasilServiceMock.listPublicTextureLibraryTags,
		).not.toHaveBeenCalled();
	});

	it("filters public textures by administrator tag", async () => {
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.tagFilterButton" }),
		);
		expect(
			screen.getByTestId("library-tag-filter-popover"),
		).toBeInTheDocument();
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listPublicTextureLibraryTags,
			).toHaveBeenCalledWith({
				limit: 30,
				offset: 0,
				keyword: undefined,
			});
		});
		fireEvent.click(screen.getByRole("checkbox", { name: /Featured/ }));

		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listPublicTextureLibraryTextures,
			).toHaveBeenLastCalledWith({
				limit: 12,
				offset: 0,
				keyword: undefined,
				tag_ids: [3],
				tag_search_method: "all",
				texture_type: undefined,
			});
		});

		fireEvent.click(
			screen.getByRole("button", { name: "wardrobe.tagSearchMethod.any" }),
		);
		await waitFor(() => {
			expect(
				yggdrasilServiceMock.listPublicTextureLibraryTextures,
			).toHaveBeenLastCalledWith({
				limit: 12,
				offset: 0,
				keyword: undefined,
				tag_ids: [3],
				tag_search_method: "any",
				texture_type: undefined,
			});
		});
	});

	it("links cape cards to detail pages and shows fallback uploader text", async () => {
		yggdrasilServiceMock.listPublicTextureLibraryTextures.mockResolvedValueOnce(
			page([
				publicTexture({
					id: 22,
					name: "Fallback Cape",
					tags: [],
					texture_model: "default",
					texture_type: "cape",
					uploader: null,
				}),
			]),
		);
		render(
			<MemoryRouter>
				<PublicTextureLibraryPage />
			</MemoryRouter>,
		);
		await screen.findByText("Fallback Cape");

		expect(screen.getByRole("link", { name: /Fallback Cape/ })).toHaveAttribute(
			"href",
			"/textures/22",
		);
		expect(
			screen.getAllByText("library.unknownUploader").length,
		).toBeGreaterThan(0);
	});
});
