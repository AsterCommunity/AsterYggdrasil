import {
	fireEvent,
	render,
	screen,
	waitFor,
	within,
} from "@testing-library/react";
import { expect } from "vitest";
import MinecraftProfilesPage from "@/pages/account/MinecraftProfilesPage";
import type {
	MinecraftTextureMetadata,
	MinecraftWardrobeTextureMetadata,
	YggdrasilProfile,
	YggdrasilProfilePage,
} from "@/types/api";

/**
 * Shared fixtures and query helpers for the colocated Minecraft profiles
 * page tests. Mock wiring (vi.mock blocks) stays inside each test file.
 */

export function profile(id: string, name: string): YggdrasilProfile {
	return { id, name, properties: [] };
}

export function texture(
	overrides: Partial<MinecraftTextureMetadata> = {},
): MinecraftTextureMetadata {
	// Object.assign keeps the merged type assignable to the full metadata
	// shape even though this helper participates in app typecheck (unlike
	// colocated *.test.tsx files, which tsconfig excludes).
	return Object.assign(
		{
			created_at: "2026-01-01T00:00:00Z",
			file_size: 128,
			hash: "texture-hash",
			height: 64,
			id: 7,
			mime_type: "image/png",
			name: "skin-hash",
			preview_url: null,
			profile_id: 1,
			profile_name: "Profile One",
			profile_uuid: "profile-one",
			source: "bound",
			texture_id: 11,
			texture_model: "default",
			texture_type: "skin",
			updated_at: "2026-01-01T00:00:00Z",
			url: "/texture.png",
			visibility: "private",
			width: 64,
		},
		overrides,
	);
}

export function uploadedTexture(
	overrides: Partial<MinecraftWardrobeTextureMetadata> = {},
): MinecraftWardrobeTextureMetadata {
	return Object.assign(
		{
			created_at: "2026-01-01T00:00:00Z",
			file_size: 128,
			hash: "uploaded-texture-hash",
			height: 64,
			id: 42,
			library_review_note: null,
			library_reviewed_at: "2026-01-01T00:00:00Z",
			library_status: "private",
			library_submitted_at: "2026-01-01T00:00:00Z",
			mime_type: "image/png",
			name: "uploaded-skin-hash",
			preview_url: null,
			tags: [],
			texture_model: "default",
			texture_type: "skin",
			updated_at: "2026-01-01T00:00:00Z",
			url: "/uploaded.png",
			visibility: "private",
			width: 64,
		},
		overrides,
	);
}

export function pngFile(width = 64, height = 64, name = "skin.png") {
	const bytes = new Uint8Array(24);
	bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0);
	bytes.set([0, 0, 0, 13], 8);
	bytes.set([0x49, 0x48, 0x44, 0x52], 12);
	const view = new DataView(bytes.buffer);
	view.setUint32(16, width);
	view.setUint32(20, height);
	return new File([bytes], name, { type: "image/png" });
}

export function cursorPage(
	items: YggdrasilProfile[],
	limit = 5,
	total = items.length,
): YggdrasilProfilePage {
	const next_cursor =
		items.length > 0 && total > items.length
			? { id: Number(items.at(-1)?.id.replace(/\D/g, "")) || items.length }
			: null;
	return { items, limit, next_cursor, total };
}

export const baseProfiles = [
	profile("profile-one", "OldName"),
	profile("profile-two", "SecondName"),
];

export async function renderPage() {
	render(<MinecraftProfilesPage />);
	await screen.findByTestId("profile-textures-action-profile-one");
}

export function rowFor(name: string) {
	const nameCell = screen
		.getAllByText(name)
		.find((element) => element.closest("button"));
	expect(nameCell).toBeDefined();
	const row = (nameCell as HTMLElement).closest("button")?.closest(".grid");
	expect(row).not.toBeNull();
	return row as HTMLElement;
}

export function queryTableRowFor(name: string) {
	const nameCell = screen
		.queryAllByText(name)
		.find((element) => element.closest("button"));
	return nameCell?.closest("button")?.closest(".grid") ?? null;
}

export function tableHeader() {
	const nameHeader = screen
		.getAllByText("profiles.profileName")
		.find((element) => element.tagName.toLowerCase() === "span");
	expect(nameHeader).toBeDefined();
	const header = (nameHeader as HTMLElement).closest(".grid");
	expect(header).not.toBeNull();
	return header as HTMLElement;
}

export function dialogs() {
	return screen.getAllByRole("dialog", { hidden: true });
}

export function topDialog() {
	const dialog = dialogs()
		.filter((element) => !element.hasAttribute("hidden"))
		.at(-1);
	expect(dialog).toBeDefined();
	return dialog as HTMLElement;
}

export function firstEnabledButton(container: HTMLElement, name: RegExp) {
	const button = within(container)
		.getAllByRole("button", { hidden: true, name })
		.find((element) => !element.hasAttribute("disabled"));
	expect(button).toBeDefined();
	return button as HTMLElement;
}

export function buttonByText(container: HTMLElement, text: string) {
	const button = within(container).getByText(text).closest("button");
	expect(button).not.toBeNull();
	return button as HTMLButtonElement;
}

export async function openTextureManager(profileId = "profile-one") {
	fireEvent.click(screen.getByTestId(`profile-textures-action-${profileId}`));
	const dialog = topDialog();
	await within(dialog).findAllByText(/profiles\.textureSlot/);
	return dialog;
}

export function openRenameDialog(profileId = "profile-one") {
	fireEvent.click(screen.getByTestId(`profile-rename-action-${profileId}`));
	return topDialog();
}

export function openDeleteProfileDialog(profileId = "profile-one") {
	fireEvent.click(screen.getByTestId(`profile-delete-action-${profileId}`));
	return topDialog();
}

export { waitFor };
