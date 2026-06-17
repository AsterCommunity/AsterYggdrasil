import type { MinecraftTextureType, PublicYggdrasilConfig } from "@/types/api";

const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a] as const;
const PNG_HEADER_BYTES = 24;

export const DEFAULT_TEXTURE_UPLOAD_POLICY = {
	max_texture_upload_bytes: 4 * 1024 * 1024,
	max_texture_pixels: 4096 * 4096,
} as const;

export type TextureUploadPolicy = Pick<
	PublicYggdrasilConfig,
	"max_texture_upload_bytes" | "max_texture_pixels"
>;

export type TextureUploadValidationError = {
	key:
		| "profiles.textureUploadInvalidType"
		| "profiles.textureUploadTooLarge"
		| "profiles.textureUploadInvalidPng"
		| "profiles.textureUploadTooManyPixels"
		| "profiles.textureUploadInvalidDimensions";
	values?: Record<string, string | number>;
};

export type TextureUploadValidationResult =
	| { ok: true; width: number; height: number }
	| ({ ok: false } & TextureUploadValidationError);

export async function validateMinecraftTextureFile(
	file: File,
	textureType: MinecraftTextureType,
	policy: TextureUploadPolicy | null | undefined,
): Promise<TextureUploadValidationResult> {
	const limits = normalizeTextureUploadPolicy(policy);
	if (file.type !== "image/png") {
		return { ok: false, key: "profiles.textureUploadInvalidType" };
	}
	if (file.size > limits.max_texture_upload_bytes) {
		return {
			ok: false,
			key: "profiles.textureUploadTooLarge",
			values: {
				max: formatFileSize(limits.max_texture_upload_bytes),
				size: formatFileSize(file.size),
			},
		};
	}

	const dimensions = await readPngDimensions(file);
	if (!dimensions) {
		return { ok: false, key: "profiles.textureUploadInvalidPng" };
	}

	const pixels = dimensions.width * dimensions.height;
	if (pixels > limits.max_texture_pixels) {
		return {
			ok: false,
			key: "profiles.textureUploadTooManyPixels",
			values: {
				height: dimensions.height,
				limit: limits.max_texture_pixels,
				pixels,
				width: dimensions.width,
			},
		};
	}

	if (
		!isValidTextureDimensions(textureType, dimensions.width, dimensions.height)
	) {
		return {
			ok: false,
			key: "profiles.textureUploadInvalidDimensions",
			values: {
				width: dimensions.width,
				height: dimensions.height,
				type: textureType,
			},
		};
	}

	return { ok: true, ...dimensions };
}

function normalizeTextureUploadPolicy(
	policy: TextureUploadPolicy | null | undefined,
): TextureUploadPolicy {
	return {
		max_texture_upload_bytes: positiveNumberOrDefault(
			policy?.max_texture_upload_bytes,
			DEFAULT_TEXTURE_UPLOAD_POLICY.max_texture_upload_bytes,
		),
		max_texture_pixels: positiveNumberOrDefault(
			policy?.max_texture_pixels,
			DEFAULT_TEXTURE_UPLOAD_POLICY.max_texture_pixels,
		),
	};
}

function positiveNumberOrDefault(value: unknown, fallback: number) {
	return typeof value === "number" && Number.isFinite(value) && value > 0
		? value
		: fallback;
}

async function readPngDimensions(file: File) {
	const bytes = new Uint8Array(
		await file.slice(0, PNG_HEADER_BYTES).arrayBuffer(),
	);
	if (bytes.length < PNG_HEADER_BYTES) return null;
	if (!PNG_SIGNATURE.every((byte, index) => bytes[index] === byte)) return null;
	const chunkType = String.fromCharCode(
		bytes[12],
		bytes[13],
		bytes[14],
		bytes[15],
	);
	if (chunkType !== "IHDR") return null;
	const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
	const width = view.getUint32(16);
	const height = view.getUint32(20);
	if (width === 0 || height === 0) return null;
	return { width, height };
}

function isValidTextureDimensions(
	textureType: MinecraftTextureType,
	width: number,
	height: number,
) {
	if (textureType === "skin") {
		return (
			isMultipleTextureSize(width, height, 64, 32) ||
			isMultipleTextureSize(width, height, 64, 64)
		);
	}
	return (
		isMultipleTextureSize(width, height, 64, 32) ||
		isMultipleTextureSize(width, height, 22, 17)
	);
}

function isMultipleTextureSize(
	width: number,
	height: number,
	unitWidth: number,
	unitHeight: number,
) {
	return (
		width >= unitWidth &&
		height >= unitHeight &&
		width % unitWidth === 0 &&
		height % unitHeight === 0 &&
		width / unitWidth === height / unitHeight
	);
}

function formatFileSize(bytes: number) {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
