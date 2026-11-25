import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TextureLibraryTextureAvatar } from "./TextureLibraryTextureAvatar";

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (key: string) => key,
	}),
}));

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

describe("TextureLibraryTextureAvatar", () => {
	it("renders skin textures through the player avatar preview", async () => {
		render(
			<TextureLibraryTextureAvatar
				texture={{
					id: 12,
					name: "Slim Skin",
					preview_url: "/textures/slim-preview.png",
					texture_type: "skin",
					url: "/textures/slim.png",
				}}
				testId="texture-avatar"
				imageTestId="texture-avatar-image"
			/>,
		);

		expect(screen.getByTestId("texture-avatar")).toBeInTheDocument();
		expect(screen.getByTestId("texture-avatar-image")).toHaveAttribute(
			"src",
			"/textures/slim.png",
		);
		expect(screen.getByTestId("texture-avatar")).toHaveAttribute(
			"title",
			"Slim Skin",
		);
		// Skins never fetch the generated preview; the avatar renders the raw
		// texture through the player preview path.
		expect(screen.getByTestId("texture-avatar-image")).toHaveAttribute(
			"src",
			"/textures/slim.png",
		);
		await waitFor(() => {
			expect(globalThis.fetch).not.toHaveBeenCalled();
		});
	});

	it("renders cape textures as pixelated image previews fetched through the preview url", async () => {
		render(
			<TextureLibraryTextureAvatar
				texture={{
					id: 13,
					name: "Red Cape",
					preview_url: "/textures/cape-preview.png",
					texture_type: "cape",
					url: "/textures/cape.png",
				}}
				testId="texture-avatar"
				imageTestId="texture-avatar-image"
			/>,
		);

		expect(screen.getByTestId("texture-avatar")).toHaveAttribute(
			"title",
			"Red Cape",
		);
		const image = screen.getByTestId("texture-avatar-image");
		expect(image).toHaveAttribute("src", "/textures/cape.png");
		await waitFor(() => {
			expect(image.getAttribute("src")).toMatch(/^blob:/);
		});
		expect(image).toHaveAttribute("crossorigin", "anonymous");
		expect(screen.getByTestId("texture-avatar-image")).toHaveClass(
			"[image-rendering:pixelated]",
		);
	});
});
