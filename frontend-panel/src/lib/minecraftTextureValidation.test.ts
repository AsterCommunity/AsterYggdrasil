import { describe, expect, it } from "vitest";
import { validateMinecraftTextureFile } from "@/lib/minecraftTextureValidation";
import type { MinecraftTextureType } from "@/types/api";

const defaultPolicy = {
	max_texture_pixels: 4096 * 4096,
	max_texture_upload_bytes: 4 * 1024 * 1024,
};

function pngFile(
	width: number,
	height: number,
	options: { name?: string; size?: number; type?: string } = {},
) {
	const bytes = new Uint8Array(Math.max(options.size ?? 24, 24));
	bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0);
	bytes.set([0, 0, 0, 13], 8);
	bytes.set([0x49, 0x48, 0x44, 0x52], 12);
	const view = new DataView(bytes.buffer);
	view.setUint32(16, width);
	view.setUint32(20, height);
	return new File([bytes], options.name ?? "texture.png", {
		type: options.type ?? "image/png",
	});
}

function invalidPngFile() {
	return new File([new Uint8Array([1, 2, 3])], "texture.png", {
		type: "image/png",
	});
}

async function validate(
	width: number,
	height: number,
	type: MinecraftTextureType,
) {
	return validateMinecraftTextureFile(
		pngFile(width, height),
		type,
		defaultPolicy,
	);
}

describe("validateMinecraftTextureFile", () => {
	it("accepts skin dimensions matching backend rules", async () => {
		await expect(validate(64, 32, "skin")).resolves.toMatchObject({
			ok: true,
			width: 64,
			height: 32,
		});
		await expect(validate(64, 64, "skin")).resolves.toMatchObject({
			ok: true,
			width: 64,
			height: 64,
		});
		await expect(validate(128, 64, "skin")).resolves.toMatchObject({
			ok: true,
			width: 128,
			height: 64,
		});
	});

	it("accepts cape dimensions including legacy cape canvases", async () => {
		await expect(validate(64, 32, "cape")).resolves.toMatchObject({
			ok: true,
			width: 64,
			height: 32,
		});
		await expect(validate(22, 17, "cape")).resolves.toMatchObject({
			ok: true,
			width: 22,
			height: 17,
		});
		await expect(validate(44, 34, "cape")).resolves.toMatchObject({
			ok: true,
			width: 44,
			height: 34,
		});
	});

	it("rejects unsupported content types before reading PNG dimensions", async () => {
		await expect(
			validateMinecraftTextureFile(
				pngFile(64, 64, { type: "image/webp" }),
				"skin",
				defaultPolicy,
			),
		).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadInvalidType",
		});
	});

	it("rejects files larger than the public runtime upload limit", async () => {
		await expect(
			validateMinecraftTextureFile(pngFile(64, 64, { size: 25 }), "skin", {
				...defaultPolicy,
				max_texture_upload_bytes: 24,
			}),
		).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadTooLarge",
		});
	});

	it("rejects invalid or zero-sized PNG headers", async () => {
		await expect(
			validateMinecraftTextureFile(invalidPngFile(), "skin", defaultPolicy),
		).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadInvalidPng",
		});
		await expect(validate(0, 64, "skin")).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadInvalidPng",
		});
	});

	it("rejects dimensions above the public runtime pixel limit", async () => {
		await expect(
			validateMinecraftTextureFile(pngFile(64, 64), "skin", {
				...defaultPolicy,
				max_texture_pixels: 4095,
			}),
		).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadTooManyPixels",
		});
	});

	it("rejects texture dimensions that do not match backend rules", async () => {
		await expect(validate(63, 64, "skin")).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadInvalidDimensions",
		});
		await expect(validate(22, 17, "skin")).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadInvalidDimensions",
		});
		await expect(validate(64, 64, "cape")).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadInvalidDimensions",
		});
		await expect(validate(23, 17, "cape")).resolves.toMatchObject({
			ok: false,
			key: "profiles.textureUploadInvalidDimensions",
		});
	});

	it("falls back to backend defaults when cached public policy is missing or invalid", async () => {
		await expect(
			validateMinecraftTextureFile(pngFile(64, 64), "skin", null),
		).resolves.toMatchObject({ ok: true });
		await expect(
			validateMinecraftTextureFile(pngFile(64, 64), "skin", {
				max_texture_pixels: 0,
				max_texture_upload_bytes: Number.NaN,
			}),
		).resolves.toMatchObject({ ok: true });
	});
});
