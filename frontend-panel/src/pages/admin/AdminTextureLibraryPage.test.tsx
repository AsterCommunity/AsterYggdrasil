import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AdminTextureLibraryPage, {
	tagColorForName,
} from "@/pages/admin/AdminTextureLibraryPage";

const toastMock = vi.hoisted(() => ({
	error: vi.fn(),
	success: vi.fn(),
}));

const adminTextureLibraryServiceMock = vi.hoisted(() => ({
	createTag: vi.fn(),
	deleteTag: vi.fn(),
	listTags: vi.fn(),
	updateTag: vi.fn(),
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

function tag(overrides: Record<string, unknown> = {}) {
	return {
		color: "#228855",
		created_at: "2026-06-15T00:00:00Z",
		id: 3,
		name: "Featured",
		sort_order: 10,
		updated_at: "2026-06-15T00:00:00Z",
		...overrides,
	};
}

function tagPage(items = [tag()]) {
	return {
		items,
		limit: 20,
		offset: 0,
		total: items.length,
	};
}

async function renderPage() {
	render(<AdminTextureLibraryPage />);
	await screen.findByText("Featured");
}

function topDialog() {
	const dialog = screen
		.getAllByRole("dialog", { hidden: true })
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

describe("AdminTextureLibraryPage", () => {
	beforeEach(() => {
		vi.clearAllMocks();
		adminTextureLibraryServiceMock.listTags.mockResolvedValue(tagPage());
		adminTextureLibraryServiceMock.createTag.mockResolvedValue(
			tag({ id: 4, name: "Classic" }),
		);
		adminTextureLibraryServiceMock.updateTag.mockResolvedValue(
			tag({ name: "Updated" }),
		);
		adminTextureLibraryServiceMock.deleteTag.mockResolvedValue(undefined);
	});

	it("loads paginated administrator texture tags", async () => {
		await renderPage();

		expect(adminTextureLibraryServiceMock.listTags).toHaveBeenCalledWith({
			limit: 20,
			offset: 0,
		});
		expect(screen.getByText("Featured")).toBeInTheDocument();
		expect(screen.getByText("#228855")).toBeInTheDocument();
	});

	it("creates a tag with a deterministic hash color from its name", async () => {
		await renderPage();

		fireEvent.change(
			screen.getAllByLabelText("admin.textureLibraryPage.name")[0],
			{
				target: { value: "  Classic  " },
			},
		);
		fireEvent.change(
			screen.getAllByLabelText("admin.textureLibraryPage.sortOrder")[0],
			{
				target: { value: "5" },
			},
		);
		fireEvent.click(
			screen.getByRole("button", {
				name: /admin.textureLibraryPage.createAction/,
			}),
		);

		await waitFor(() => {
			expect(adminTextureLibraryServiceMock.createTag).toHaveBeenCalledWith({
				color: tagColorForName("Classic"),
				name: "Classic",
				sort_order: 5,
			});
		});
		expect(toastMock.success).toHaveBeenCalledWith(
			"admin.textureLibraryPage.createSuccess",
		);
	});

	it("blocks blank tag names before creating", async () => {
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", {
				name: /admin.textureLibraryPage.createAction/,
			}),
		);

		expect(adminTextureLibraryServiceMock.createTag).not.toHaveBeenCalled();
		expect(toastMock.error).toHaveBeenCalledWith(
			"admin.textureLibraryPage.nameRequired",
		);
	});

	it("updates a tag through the edit dialog", async () => {
		await renderPage();

		fireEvent.click(
			screen.getByRole("button", {
				name: /admin.textureLibraryPage.editAction/,
			}),
		);
		const dialog = topDialog();
		fireEvent.change(
			within(dialog).getByLabelText("admin.textureLibraryPage.name"),
			{
				target: { value: "  Updated  " },
			},
		);
		fireEvent.click(
			within(dialog).getByRole("button", { name: "common.save" }),
		);

		await waitFor(() => {
			expect(adminTextureLibraryServiceMock.updateTag).toHaveBeenCalledWith(3, {
				color: tagColorForName("Updated"),
				name: "Updated",
				sort_order: 10,
			});
		});
	});

	it("uses stable colors for equivalent tag names", () => {
		expect(tagColorForName("  Classic  ")).toBe(tagColorForName("classic"));
		expect(tagColorForName("")).toMatch(/^#[0-9a-f]{6}$/);
	});

	it("deletes a tag through confirmation", async () => {
		await renderPage();

		fireEvent.click(screen.getByRole("button", { name: /common.delete/ }));
		fireEvent.click(
			within(topDialog()).getByRole("button", { name: "common.delete" }),
		);

		await waitFor(() => {
			expect(adminTextureLibraryServiceMock.deleteTag).toHaveBeenCalledWith(3);
		});
	});
});
