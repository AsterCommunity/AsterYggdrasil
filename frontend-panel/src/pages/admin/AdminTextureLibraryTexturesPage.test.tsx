import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import AdminTextureLibraryTexturesPage from "@/pages/admin/AdminTextureLibraryTexturesPage";
import type { AdminTextureLibraryPage } from "@/types/api";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const adminTextureLibraryServiceMock = vi.hoisted(() => ({
	approveTexture: vi.fn(),
	deleteTexture: vi.fn(),
	getTexture: vi.fn(),
	listTextures: vi.fn(),
	rejectTexture: vi.fn(),
	unpublishTexture: vi.fn(),
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

// Texture avatars fetch their generated preview; the preview URL resolves to
// a blob object URL in tests. The raw texture is shown until it resolves.
let previewObjectUrlCounter = 0;
const previewObjectUrls: string[] = [];
let previewFetchSpy: ReturnType<typeof vi.spyOn> | undefined;
const originalCreateObjectURL = URL.createObjectURL;
const originalRevokeObjectURL = URL.revokeObjectURL;

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

vi.mock("@/services/adminService", async (importOriginal) => {
	const actual =
		await importOriginal<typeof import("@/services/adminService")>();
	return {
		...actual,
		adminTextureLibraryService: adminTextureLibraryServiceMock,
	};
});

vi.mock("@/hooks/useApiError", () => ({
	handleApiError: (error: unknown) => {
		toastMock.error(error instanceof Error ? error.message : String(error));
	},
}));

function texturePage(
	items: AdminTextureLibraryPage["items"] = [texture()],
): AdminTextureLibraryPage {
	return {
		items,
		limit: 20,
		total: items.length,
	};
}

function texture(
	overrides: Partial<AdminTextureLibraryPage["items"][number]> = {},
): AdminTextureLibraryPage["items"][number] {
	return {
		created_at: "2026-06-15T00:00:00Z",
		display_name: "Review Skin",
		file_size: 128,
		hash: "hash-review",
		height: 64,
		id: 12,
		library_review_note: null,
		library_reviewed_at: "2026-06-15T01:00:00Z",
		library_status: "pending_review",
		library_submitted_at: "2026-06-15T00:30:00Z",
		mime_type: "image/png",
		name: "Review Skin",
		preview_url: "/textures/hash-review-preview.png",
		tags: [],
		texture_model: "slim",
		texture_type: "skin",
		updated_at: "2026-06-15T00:00:00Z",
		uploader: {
			avatar: {
				source: "upload",
				url_1024: "/admin/avatars/users/1/1024?v=2",
				url_512: "/admin/avatars/users/1/512?v=2",
				version: 2,
			},
			id: 1,
			name: "Steve",
			public_uuid: "user-public-uuid",
			username: "steve",
		},
		url: "/textures/hash-review.png",
		visibility: "public",
		width: 64,
		...overrides,
	};
}

function LocationProbe() {
	const location = useLocation();
	return <div data-testid="current-path">{location.pathname}</div>;
}

function renderPage(mode: "all" | "reviews" = "reviews") {
	render(
		<MemoryRouter>
			<AdminTextureLibraryTexturesPage mode={mode} />
		</MemoryRouter>,
	);
}

function renderListRoute(mode: "all" | "reviews" = "reviews") {
	render(
		<MemoryRouter initialEntries={["/admin/texture-library"]}>
			<LocationProbe />
			<Routes>
				<Route
					path="/admin/texture-library"
					element={<AdminTextureLibraryTexturesPage mode={mode} />}
				/>
				<Route
					path="/admin/texture-library/:textureId"
					element={<div data-testid="texture-detail-route" />}
				/>
			</Routes>
		</MemoryRouter>,
	);
}

function renderDetailRoute(textureId = "12") {
	render(
		<MemoryRouter initialEntries={[`/admin/texture-library/${textureId}`]}>
			<Routes>
				<Route
					path="/admin/texture-library"
					element={<AdminTextureLibraryTexturesPage mode="all" />}
				/>
				<Route
					path="/admin/texture-library/:textureId"
					element={<AdminTextureLibraryTexturesPage mode="detail" />}
				/>
			</Routes>
		</MemoryRouter>,
	);
}

function topDialog() {
	const dialog = screen
		.getAllByRole("dialog", { hidden: true })
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

afterEach(() => {
	previewFetchSpy?.mockRestore();
	URL.createObjectURL = originalCreateObjectURL;
	URL.revokeObjectURL = originalRevokeObjectURL;
});

beforeEach(() => {
	previewObjectUrls.length = 0;
	// Patch the static methods only: replacing the URL constructor entirely
	// breaks router code that calls `new URL(...)`.
	URL.createObjectURL = vi.fn(() => {
		previewObjectUrlCounter += 1;
		const url = `blob:preview-${previewObjectUrlCounter}`;
		previewObjectUrls.push(url);
		return url;
	});
	URL.revokeObjectURL = vi.fn();
	previewFetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
		new Response(new Blob(["png-bytes"], { type: "image/png" }), {
			status: 200,
		}),
	);
});

describe("AdminTextureLibraryTexturesPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		adminTextureLibraryServiceMock.listTextures.mockResolvedValue(
			texturePage(),
		);
		adminTextureLibraryServiceMock.approveTexture.mockResolvedValue(
			texture({ library_status: "published" }),
		);
		adminTextureLibraryServiceMock.rejectTexture.mockResolvedValue(
			texture({
				library_review_note: "not acceptable",
				library_status: "rejected",
			}),
		);
		adminTextureLibraryServiceMock.deleteTexture.mockResolvedValue(undefined);
		adminTextureLibraryServiceMock.unpublishTexture.mockResolvedValue(
			texture({ library_status: "private" }),
		);
	});

	it("loads the review queue with pending public unpublished filters", async () => {
		renderPage("reviews");

		await screen.findByText("Review Skin");

		expect(adminTextureLibraryServiceMock.listTextures).toHaveBeenCalledWith({
			after_id: undefined,
			after_updated_at: undefined,
			keyword: undefined,
			library_status: "pending_review",
			limit: 20,
			published: false,
			texture_type: undefined,
			visibility: "public",
		});
		expect(
			screen.getByText(
				"admin.textureLibraryTexturesPage.libraryStatus.pending_review",
			),
		).toBeInTheDocument();
	});

	it("uses the shared texture avatar preview rules in the texture table", async () => {
		adminTextureLibraryServiceMock.listTextures.mockResolvedValue(
			texturePage([
				texture({
					display_name: "Review Skin",
					id: 12,
					name: "Review Skin",
					preview_url: "/textures/hash-review-preview.png",
					texture_type: "skin",
					url: "/textures/hash-review.png",
				}),
				texture({
					display_name: "Review Cape",
					id: 13,
					name: "Review Cape",
					preview_url: "/textures/cape-preview.png",
					texture_model: "default",
					texture_type: "cape",
					url: "/textures/cape.png",
				}),
			]),
		);

		renderPage("reviews");
		await screen.findByText("Review Cape");

		expect(screen.getByTestId("admin-texture-preview-12")).toHaveAttribute(
			"title",
			"Review Skin",
		);
		expect(
			screen.getByTestId("admin-texture-preview-image-12"),
		).toHaveAttribute("src", "/textures/hash-review.png");
		expect(screen.getByTestId("admin-texture-preview-13")).toHaveAttribute(
			"title",
			"Review Cape",
		);
		await screen.findByTestId("admin-texture-preview-image-13");
		await waitFor(() => {
			expect(
				screen
					.getByTestId("admin-texture-preview-image-13")
					.getAttribute("src"),
			).toMatch(/^blob:/);
		});
		expect(
			document.querySelector(
				'img[src="/api/v1/admin/avatars/users/1/512?v=2"]',
			),
		).toBeInTheDocument();
		expect(screen.getAllByText("@steve · #1")).not.toHaveLength(0);
	});

	it("opens the texture detail route when a texture row is clicked", async () => {
		renderListRoute("reviews");
		await screen.findByText("Review Skin");

		fireEvent.click(screen.getByText("Review Skin").closest("tr") as Element);

		expect(screen.getByTestId("current-path")).toHaveTextContent(
			"/admin/texture-library/12",
		);
		expect(screen.getByTestId("texture-detail-route")).toBeInTheDocument();
	});

	it("does not navigate when a review action button is clicked", async () => {
		renderListRoute("reviews");
		await screen.findByText("Review Skin");

		fireEvent.click(
			screen.getByRole("button", {
				name: "admin.textureLibraryTexturesPage.rejectAction",
			}),
		);

		expect(screen.getByTestId("current-path")).toHaveTextContent(
			"/admin/texture-library",
		);
		expect(
			screen.queryByTestId("texture-detail-route"),
		).not.toBeInTheDocument();
		expect(
			screen.getByText("admin.textureLibraryTexturesPage.rejectTitle"),
		).toBeInTheDocument();
	});

	it("confirms and deletes a texture from the table without navigating", async () => {
		renderListRoute("reviews");
		await screen.findByText("Review Skin");

		fireEvent.click(
			screen.getByRole("button", {
				name: "common.delete",
			}),
		);

		expect(screen.getByTestId("current-path")).toHaveTextContent(
			"/admin/texture-library",
		);
		const dialog = topDialog();
		expect(
			within(dialog).getByText("admin.textureLibraryTexturesPage.deleteTitle"),
		).toBeInTheDocument();
		expect(
			within(dialog).getByText(
				'admin.textureLibraryTexturesPage.deleteDescription {"name":"Review Skin"}',
			),
		).toBeInTheDocument();

		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.delete" }),
		);

		await screen.findByText("Review Skin");
		expect(adminTextureLibraryServiceMock.deleteTexture).toHaveBeenCalledWith(
			12,
		);
		expect(adminTextureLibraryServiceMock.listTextures).toHaveBeenCalledTimes(
			2,
		);
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.textureLibraryTexturesPage.deleteSuccess",
		);
		expect(screen.getByTestId("current-path")).toHaveTextContent(
			"/admin/texture-library",
		);
	});

	it("loads a texture detail with the shared page module", async () => {
		adminTextureLibraryServiceMock.getTexture.mockResolvedValue(
			texture({
				display_name: "Detail Skin",
				file_size: 1536,
				tags: [
					{
						color: "#6ee7b7",
						created_at: "2026-06-15T00:00:00Z",
						id: 7,
						name: "Featured",
						sort_order: 0,
						updated_at: "2026-06-15T00:00:00Z",
					},
				],
			}),
		);

		renderDetailRoute("12");

		await screen.findByRole("heading", { level: 1, name: "Detail Skin" });
		expect(adminTextureLibraryServiceMock.getTexture).toHaveBeenCalledWith(12);
		expect(adminTextureLibraryServiceMock.listTextures).not.toHaveBeenCalled();
		expect(screen.getByText("1.5 KiB")).toBeInTheDocument();
		expect(
			screen.getByTestId("admin-texture-detail-avatar-12"),
		).toHaveAttribute("title", "Review Skin");
		expect(
			screen.getByTestId("admin-texture-detail-avatar-image-12"),
		).toHaveAttribute("src", "/textures/hash-review.png");
		expect(
			document.querySelector(
				'img[src="/api/v1/admin/avatars/users/1/512?v=2"]',
			),
		).toBeInTheDocument();
		expect(screen.getByText("Steve")).toBeInTheDocument();
		expect(screen.getByText("@steve · #1")).toBeInTheDocument();
		expect(screen.getByText("Featured")).toBeInTheDocument();
	});

	it("deletes a texture from the detail page and returns to the texture list", async () => {
		adminTextureLibraryServiceMock.getTexture.mockResolvedValue(
			texture({ display_name: "Detail Skin" }),
		);

		renderDetailRoute("12");

		await screen.findByRole("heading", { level: 1, name: "Detail Skin" });
		fireEvent.click(
			screen.getByRole("button", {
				name: "common.delete",
			}),
		);
		const dialog = topDialog();
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.delete" }),
		);

		expect(adminTextureLibraryServiceMock.deleteTexture).toHaveBeenCalledWith(
			12,
		);
		expect(
			await screen.findByText("admin.textureLibraryTexturesPage.title"),
		).toBeInTheDocument();
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.textureLibraryTexturesPage.deleteSuccess",
		);
	});

	it("requires a review note before rejecting a pending texture", async () => {
		renderPage("reviews");
		await screen.findByText("Review Skin");

		fireEvent.click(
			screen.getByRole("button", {
				name: "admin.textureLibraryTexturesPage.rejectAction",
			}),
		);
		const dialog = topDialog();
		fireEvent.click(
			within(dialog).getByRole("button", {
				name: "admin.textureLibraryTexturesPage.rejectAction",
			}),
		);

		expect(toastMock.error).toHaveBeenCalledWith(
			"admin.textureLibraryTexturesPage.reviewNoteRequired",
		);
		expect(adminTextureLibraryServiceMock.rejectTexture).not.toHaveBeenCalled();
	});
});
