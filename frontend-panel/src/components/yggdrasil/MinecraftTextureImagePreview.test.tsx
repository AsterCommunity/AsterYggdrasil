import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MinecraftTextureImagePreview } from "./MinecraftTextureImagePreview";

describe("MinecraftTextureImagePreview", () => {
	it("renders the source texture with pixelated image constraints", () => {
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
	});

	it("uses a generated preview URL inside the image container when available", () => {
		render(
			<MinecraftTextureImagePreview
				alt="Cape texture preview"
				previewUrl="/texture-previews/cape.png"
				textureUrl="/textures/cape.png"
			/>,
		);

		const image = screen.getByRole("img", { name: "Cape texture preview" });
		expect(image).toHaveAttribute("src", "/texture-previews/cape.png");
		expect(image.parentElement).toHaveClass("aspect-square");
	});
});
