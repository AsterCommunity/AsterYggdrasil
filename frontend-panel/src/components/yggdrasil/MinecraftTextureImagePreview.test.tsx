import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MinecraftTextureImagePreview } from "./MinecraftTextureImagePreview";

const objectUrls: string[] = [];
let objectUrlCounter = 0;

beforeEach(() => {
	objectUrls.length = 0;
	vi.stubGlobal("URL", {
		...URL,
		createObjectURL: vi.fn(() => {
			objectUrlCounter += 1;
			const url = `blob:test-${objectUrlCounter}`;
			objectUrls.push(url);
			return url;
		}),
		revokeObjectURL: vi.fn(),
	});
	vi.spyOn(globalThis, "fetch").mockResolvedValue(
		new Response(new Blob(["png-bytes"], { type: "image/png" }), {
			status: 200,
		}),
	);
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

describe("MinecraftTextureImagePreview", () => {
	it("renders the source texture with pixelated image constraints", async () => {
		render(
			<MinecraftTextureImagePreview
				alt="Skin texture preview"
				aspect="portrait"
				textureUrl="/textures/skin.png"
			/>,
		);

		const image = screen.getByRole("img", { name: "Skin texture preview" });
		expect(image).toHaveAttribute("src", "/textures/skin.png");
		expect(image).toHaveAttribute("crossorigin", "anonymous");
		expect(image).toHaveClass("[image-rendering:pixelated]");
		expect(image.parentElement).toHaveClass("aspect-[4/5]");
		expect(image.parentElement).toHaveClass("overflow-hidden");
		await waitFor(() => {
			expect(globalThis.fetch).not.toHaveBeenCalled();
		});
	});

	it("fetches the generated preview and swaps it in over the source texture", async () => {
		render(
			<MinecraftTextureImagePreview
				alt="Cape texture preview"
				previewUrl="/texture-previews/cape.png"
				textureUrl="/textures/cape.png"
			/>,
		);

		// Before the fetch resolves the raw texture is shown, so a cold cache
		// never renders an empty frame.
		const image = screen.getByRole("img", { name: "Cape texture preview" });
		expect(image).toHaveAttribute("src", "/textures/cape.png");
		expect(image.parentElement).toHaveClass("aspect-square");

		await waitFor(() => {
			expect(image.getAttribute("src")).toMatch(/^blob:/);
		});
		expect(objectUrls).toHaveLength(1);
		expect(globalThis.fetch).toHaveBeenCalledWith(
			"/texture-previews/cape.png",
			expect.objectContaining({ headers: { Accept: "image/png" } }),
		);
	});
});
