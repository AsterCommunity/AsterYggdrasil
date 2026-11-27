import { beforeEach, describe, expect, it, vi } from "vitest";
import { textureTagStore } from "./textureTagStore";

function tagPageLoader(marker: string) {
	return async () => ({
		items: [
			{
				color: "#228855",
				created_at: "2026-06-15T00:00:00Z",
				id: 1,
				name: marker,
				sort_order: 0,
				updated_at: "2026-06-15T00:00:00Z",
			},
		],
		limit: 30,
		next_cursor: null,
		total: 1,
	});
}

describe("textureTagStore", () => {
	beforeEach(() => {
		textureTagStore.clearTextureTagPages();
		textureTagStore.clearPublicTextureTagPages();
	});

	it("dedupes concurrent page loads for the same normalized query", async () => {
		const loader = vi.fn(tagPageLoader("one"));

		const [first, second] = await Promise.all([
			textureTagStore.getTextureTagPage({ keyword: "Nova " }, loader),
			textureTagStore.getTextureTagPage({ keyword: "Nova" }, loader),
		]);

		expect(second).toBe(first);
		expect(loader).toHaveBeenCalledTimes(1);
	});

	it("keeps wardrobe and public-library caches separate", async () => {
		const wardrobeLoader = vi.fn(tagPageLoader("wardrobe"));
		const publicLoader = vi.fn(tagPageLoader("public"));

		await textureTagStore.getTextureTagPage({}, wardrobeLoader);
		await textureTagStore.getPublicTextureTagPage({}, publicLoader);

		expect(wardrobeLoader).toHaveBeenCalledTimes(1);
		expect(publicLoader).toHaveBeenCalledTimes(1);
	});

	it("clear() forces the next load to refetch", async () => {
		const loader = vi.fn(tagPageLoader("first"));

		await textureTagStore.getTextureTagPage({}, loader);
		textureTagStore.clearTextureTagPages();
		await textureTagStore.getTextureTagPage({}, tagPageLoader("second"));

		expect(loader).toHaveBeenCalledTimes(1);
	});
});
