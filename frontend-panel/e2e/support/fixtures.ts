import path from "node:path";
import { deflateSync } from "node:zlib";

/**
 * Directory (inside Playwright's output dir) holding the storageState files
 * produced by the `setup` project. Cleared and recreated on every run.
 */
export const AUTH_STATE_DIR = path.join("test-results", ".auth");

/**
 * Base URL of the UI suite's embedded backend. Must stay in sync with
 * `playwright.config.ts`; helper API sessions use it because fresh request
 * contexts do not inherit the project's baseURL.
 */
export function e2eBaseURL() {
	const port = process.env.ASTER_E2E_PORT ?? "3310";
	return process.env.PLAYWRIGHT_BASE_URL ?? `http://localhost:${port}`;
}

/** Base URL of the protocol suite's backend (see playwright.protocol.config.ts). */
export function protocolBaseURL() {
	const port = process.env.ASTER_E2E_PROTOCOL_PORT ?? "3311";
	return process.env.PLAYWRIGHT_PROTOCOL_BASE_URL ?? `http://localhost:${port}`;
}

export const STORAGE_STATE_FILES = {
	admin: path.join(AUTH_STATE_DIR, "admin.json"),
	operator: path.join(AUTH_STATE_DIR, "operator.json"),
	player: path.join(AUTH_STATE_DIR, "player.json"),
} as const;

/**
 * E2E account credentials.
 *
 * Passwords never appear hardcoded in spec files: they come from the
 * `ASTER_E2E_*_PASSWORD` environment variables, falling back to disposable
 * dev-only values below. These accounts only ever exist inside the throwaway
 * runtime database created by `scripts/run-e2e-server.sh` for a single run.
 *
 * See `e2e/README.md` for the documented variable names.
 */
function resolvePassword(envName: string, devFallback: string) {
	return process.env[envName] ?? devFallback;
}

export const ADMIN = {
	email: "e2e-admin@example.test",
	password: resolvePassword("ASTER_E2E_ADMIN_PASSWORD", "Vq7-tEst-Admin9x!"),
	username: "e2e_admin",
} as const;

export const PLAYER = {
	email: "e2e-player@example.test",
	password: resolvePassword("ASTER_E2E_PLAYER_PASSWORD", "Pk3-tEst-Player5x!"),
	username: "e2e_player",
} as const;

export const OPERATOR = {
	email: "e2e-operator@example.test",
	password: resolvePassword(
		"ASTER_E2E_OPERATOR_PASSWORD",
		"Op5-tEst-Operat2x!",
	),
	username: "e2e_operator",
} as const;

const RUN_ID = Date.now().toString(36);

/** Unique-per-run suffix so parallel workers never collide on shared rows. */
export function uniqueName(prefix: string) {
	return `${prefix}-${RUN_ID}-${Math.random().toString(36).slice(2, 6)}`;
}

/**
 * Site username (4-16 chars: letters, numbers, underscores, hyphens).
 * Distinct from Minecraft profile names, which are produced by
 * `uniqueProfileName`.
 */
export function uniqueUsername(prefix: string) {
	return uniqueName(prefix).replaceAll("_", "-").slice(0, 16);
}

/** Minecraft profile name (3-16 ASCII letters, digits, underscore). */
export function uniqueProfileName(prefix = "player") {
	return `${prefix}${Math.random().toString(36).slice(2, 10)}`.slice(0, 16);
}

function crc32(bytes: Uint8Array) {
	let crc = 0xffffffff;
	for (const byte of bytes) {
		crc ^= byte;
		for (let bit = 0; bit < 8; bit += 1) {
			crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
		}
	}
	return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type: string, data: Uint8Array) {
	const length = Buffer.alloc(4);
	length.writeUInt32BE(data.length);
	const typeBytes = Buffer.from(type, "ascii");
	const crc = Buffer.alloc(4);
	crc.writeUInt32BE(crc32(new Uint8Array(Buffer.concat([typeBytes, data]))));
	return Buffer.concat([length, typeBytes, data, crc]);
}

/**
 * Builds a valid solid-color PNG of the requested size at runtime. Skins must
 * be multiples of 64x64 (classic) or 64x32 (legacy); capes are 64x32 or 22x17.
 * Generating the bytes in-process keeps fixtures free of embedded blobs.
 *
 * `seed` tints the first pixel so callers can produce content-unique images;
 * the backend deduplicates wardrobe textures per user by content hash, so
 * parallel specs uploading for the same account must not share bytes.
 */
export function createPng(width: number, height: number, seed = 0) {
	const raw = Buffer.alloc(height * (1 + width * 3));
	for (let row = 0; row < height; row += 1) {
		const offset = row * (1 + width * 3);
		raw[offset] = 0;
		for (let column = 0; column < width; column += 1) {
			const pixel = offset + 1 + column * 3;
			raw[pixel] = 0x30;
			raw[pixel + 1] = 0xb0;
			raw[pixel + 2] = 0x60;
		}
	}
	// Tint the first pixel (top-left, never part of validation-sensitive
	// regions) with the seed so each call hashes differently.
	raw[1] = (0x30 + (seed & 0xff)) & 0xff;
	raw[2] = (0xb0 + ((seed >>> 8) & 0xff)) & 0xff;
	raw[3] = (0x60 + ((seed >>> 16) & 0xff)) & 0xff;

	const ihdr = Buffer.alloc(13);
	ihdr.writeUInt32BE(width, 0);
	ihdr.writeUInt32BE(height, 4);
	ihdr[8] = 8; // bit depth
	ihdr[9] = 2; // color type RGB
	return Buffer.concat([
		Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
		pngChunk("IHDR", ihdr),
		pngChunk("IDAT", deflateSync(raw)),
		pngChunk("IEND", new Uint8Array()),
	]);
}

/** A classic 64x64 skin that passes backend texture validation. */
export function createSkinPng() {
	return createPng(64, 64);
}

/**
 * A classic 64x64 skin with per-call-unique bytes. Use this when several
 * parallel specs upload textures for the same account: identical bytes would
 * collapse into one deduplicated wardrobe row and race on its visibility.
 */
export function createUniqueSkinPng() {
	return createPng(64, 64, Math.floor(Math.random() * 0xffffff) + 1);
}
