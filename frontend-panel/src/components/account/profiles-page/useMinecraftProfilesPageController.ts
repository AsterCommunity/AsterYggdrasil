import {
	type DragEvent,
	type FormEvent,
	useCallback,
	useEffect,
	useMemo,
	useRef,
} from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useCursorList } from "@/hooks/useCursorList";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { usePageTitle } from "@/hooks/usePageTitle";
import { validateMinecraftTextureFile } from "@/lib/minecraftTextureValidation";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type {
	IdCursor,
	MinecraftTextureType,
	YggdrasilProfile,
} from "@/types/api";
import { useMinecraftProfilesPageState } from "./useMinecraftProfilesPageState";

const DEFAULT_PROFILE_PAGE_SIZE = 5;
const PROFILE_PAGE_SIZE_OPTIONS = [5, 10] as const;
const PROFILE_SEARCH_DEBOUNCE_MS = 300;

export function useMinecraftProfilesPageController() {
	const { t } = useTranslation();
	const [state, dispatch] = useMinecraftProfilesPageState();
	const {
		deletingProfile,
		file,
		loading,
		model,
		profilePageSize,
		profileName,
		profileSkinUrls,
		query,
		renameDialogOpen,
		renameName,
		renaming,
		selectedUuid,
		textures,
		texturesLoading,
		textureType,
		uploadTextureType,
		visibility,
	} = state;

	usePageTitle(t("profiles.title"));

	// The search draft commits to the profile list query after a debounce.
	const debouncedQuery = useDebouncedValue(
		query.trim(),
		PROFILE_SEARCH_DEBOUNCE_MS,
	);

	const loadProfilePage = useCallback(
		async ({ cursor }: { cursor: IdCursor | null }) => {
			const params = {
				after_id: cursor?.id,
				limit: profilePageSize,
			};
			const page = await yggdrasilService.listProfiles(
				debouncedQuery ? { ...params, query: debouncedQuery } : params,
			);
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[debouncedQuery, profilePageSize],
	);
	const list = useCursorList<YggdrasilProfile, IdCursor>({
		deps: [debouncedQuery, profilePageSize],
		loadPage: loadProfilePage,
		onError: (error) => toast.error(formatUnknownError(error)),
	});
	const profiles = list.items;
	const profileTotal = list.total;
	const profilesLoading = list.loading;

	// Keep the selection anchored: when the loaded page no longer contains the
	// selected profile (deletion, page switch), fall back to the first row.
	useEffect(() => {
		if (
			selectedUuid &&
			profiles.some((profile) => profile.id === selectedUuid)
		) {
			return;
		}
		const fallback = profiles[0]?.id || "";
		if (fallback !== selectedUuid) {
			dispatch({ type: "selectedUuid", value: fallback });
		}
	}, [dispatch, profiles, selectedUuid]);

	const loadTextures = useCallback(
		async (uuid: string) => {
			if (!uuid) {
				dispatch({ type: "textures", value: [] });
				return;
			}
			dispatch({ type: "texturesLoading", value: true });
			try {
				dispatch({
					type: "textures",
					value: await yggdrasilService.listProfileTextures(uuid),
				});
			} catch (nextError) {
				toast.error(formatUnknownError(nextError));
				dispatch({ type: "textures", value: [] });
			} finally {
				dispatch({ type: "texturesLoading", value: false });
			}
		},
		[dispatch],
	);

	const loadProfileSkinUrls = useCallback(
		async (nextProfiles = profiles) => {
			if (nextProfiles.length === 0) {
				dispatch({ type: "profileSkinUrls", value: {} });
				return;
			}
			try {
				dispatch({
					type: "profileSkinUrls",
					value: await yggdrasilService.listProfileSkinTextureUrls(
						nextProfiles.map((profile) => profile.id),
					),
				});
			} catch (nextError) {
				console.warn(
					"Failed to load Minecraft profile skin avatars",
					nextError,
				);
				dispatch({ type: "profileSkinUrls", value: {} });
			}
		},
		[dispatch, profiles],
	);

	useEffect(() => {
		void loadTextures(selectedUuid);
	}, [selectedUuid, loadTextures]);

	useEffect(() => {
		void loadProfileSkinUrls();
	}, [loadProfileSkinUrls]);

	const selectedProfile = useMemo(
		() => profiles.find((profile) => profile.id === selectedUuid) ?? null,
		[profiles, selectedUuid],
	);
	const searchBusy =
		query.trim() !== debouncedQuery.trim() ||
		(profilesLoading && Boolean(debouncedQuery.trim()));
	const skinTexture =
		textures.find((texture) => texture.texture_type === "skin") ?? null;
	const capeTexture =
		textures.find((texture) => texture.texture_type === "cape") ?? null;
	const activeTexture = textureType === "skin" ? skinTexture : capeTexture;

	const yggdrasilConfig = useFrontendConfigStore((store) => store.yggdrasil);
	const renameUuidRef = useRef("");

	async function createProfile(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		dispatch({ type: "loading", value: true });
		try {
			const created = await yggdrasilService.createProfile({
				name: profileName,
			});
			dispatch({ type: "profileName", value: "" });
			// Reload the first page; the created profile is picked there.
			const page = await list.reloadFirstPage();
			if (page?.items.some((profile) => profile.id === created.id)) {
				dispatch({ type: "selectedUuid", value: created.id });
			}
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	function openRenameDialog(profile: { id: string; name: string }) {
		renameUuidRef.current = profile.id;
		dispatch({ type: "renameName", value: profile.name });
		dispatch({ type: "renameDialogOpen", value: true });
	}

	async function renameProfile(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const renameUuid = renameUuidRef.current;
		if (!renameUuid || !renameName.trim()) return;
		dispatch({ type: "renaming", value: true });
		try {
			const renamed = await yggdrasilService.renameProfile(renameUuid, {
				name: renameName.trim(),
			});
			dispatch({ type: "renameDialogOpen", value: false });
			renameUuidRef.current = "";
			dispatch({ type: "renameName", value: "" });
			const page = await list.reloadFirstPage();
			if (page?.items.some((profile) => profile.id === renamed.id)) {
				dispatch({ type: "selectedUuid", value: renamed.id });
			}
			toast.success(t("profiles.renameToast"));
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "renaming", value: false });
		}
	}

	async function deleteProfile() {
		if (!selectedUuid || !selectedProfile) return;
		dispatch({ type: "deletingProfile", value: true });
		try {
			await yggdrasilService.deleteProfile(selectedUuid);
			dispatch({ type: "deleteProfileDialogOpen", value: false });
			dispatch({ type: "selectedUuid", value: "" });
			await list.reloadFirstPage();
			toast.success(t("profiles.deleteProfileToast"));
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "deletingProfile", value: false });
		}
	}

	async function uploadTexture(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!file || !selectedUuid) return;
		if (!(await validateTextureFile(file))) return;
		dispatch({ type: "loading", value: true });
		try {
			const uploaded = await yggdrasilService.uploadWardrobeTexture({
				textureType: uploadTextureType,
				file,
				model,
				visibility,
			});
			await yggdrasilService.bindProfileTexture({
				uuid: selectedUuid,
				textureType: uploaded.texture_type,
				textureId: uploaded.id,
			});
			dispatch({ type: "textureDialogOpen", value: false });
			dispatch({ type: "file", value: null });
			toast.success(t("profiles.uploadAndBindToast"));
			await loadTextures(selectedUuid);
			if (uploaded.texture_type === "skin") {
				await loadProfileSkinUrls();
			}
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	async function validateTextureFile(nextFile: File) {
		const validation = await validateMinecraftTextureFile(
			nextFile,
			uploadTextureType,
			yggdrasilConfig,
		);
		if (validation.ok) return true;
		toast.error(t(validation.key, validation.values ?? {}));
		return false;
	}

	async function selectTextureFile(nextFile: File | null) {
		if (nextFile && !(await validateTextureFile(nextFile))) {
			dispatch({ type: "file", value: null });
			return;
		}
		dispatch({ type: "file", value: nextFile });
	}

	function dropTextureFile(event: DragEvent<HTMLLabelElement>) {
		event.preventDefault();
		dispatch({ type: "dragActive", value: false });
		void selectTextureFile(event.dataTransfer.files.item(0));
	}

	function dragTextureFile(event: DragEvent<HTMLLabelElement>) {
		event.preventDefault();
		dispatch({ type: "dragActive", value: true });
	}

	function leaveTextureDropZone() {
		dispatch({ type: "dragActive", value: false });
	}

	async function deleteTexture() {
		if (!selectedUuid) return;
		dispatch({ type: "loading", value: true });
		try {
			await yggdrasilService.unbindProfileTexture({
				uuid: selectedUuid,
				textureType,
			});
			dispatch({ type: "deleteDialogOpen", value: false });
			toast.success(t("profiles.deleteSuccess"));
			await loadTextures(selectedUuid);
			if (textureType === "skin") {
				await loadProfileSkinUrls();
			}
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	function openTextureDialog(nextTextureType: MinecraftTextureType) {
		dispatch({ type: "uploadTextureType", value: nextTextureType });
		dispatch({ type: "textureType", value: nextTextureType });
		dispatch({
			type: "model",
			value:
				nextTextureType === "skin"
					? (skinTexture?.texture_model ?? model)
					: model,
		});
		dispatch({ type: "file", value: null });
		dispatch({ type: "dragActive", value: false });
		dispatch({ type: "textureDialogOpen", value: true });
	}

	function openDeleteTextureDialog(nextTextureType: MinecraftTextureType) {
		dispatch({ type: "textureType", value: nextTextureType });
		dispatch({ type: "deleteDialogOpen", value: true });
	}

	function changeProfilePageSize(value: string | null) {
		const parsed = Number(value);
		const nextPageSize = PROFILE_PAGE_SIZE_OPTIONS.includes(
			parsed as (typeof PROFILE_PAGE_SIZE_OPTIONS)[number],
		)
			? parsed
			: DEFAULT_PROFILE_PAGE_SIZE;
		// pageSize is a list dep, so the profile list resets and reloads.
		dispatch({ type: "profilePageSize", value: nextPageSize });
	}

	return {
		activeTexture,
		capeTexture,
		deletingProfile,
		dispatch,
		loading,
		model,
		profileName,
		profileCurrentPage: list.currentPage,
		profileNextDisabled: list.nextPageDisabled,
		profilePageSize,
		profilePrevDisabled: list.prevPageDisabled,
		profileSkinUrls,
		profileTotal,
		profileTotalPages: list.totalPages,
		profiles,
		query,
		renameDialogOpen,
		renameName,
		renaming,
		searchBusy,
		selectedProfile,
		selectedUuid,
		skinTexture,
		state,
		texturesLoading,
		onChangePageSize: changeProfilePageSize,
		onCreateProfile: createProfile,
		onDeleteProfile: () => void deleteProfile(),
		onDeleteTexture: () => void deleteTexture(),
		onDragTextureFile: dragTextureFile,
		onDropTextureFile: dropTextureFile,
		onLeaveTextureDropZone: leaveTextureDropZone,
		onOpenDeleteTextureDialog: openDeleteTextureDialog,
		onOpenRenameDialog: openRenameDialog,
		onOpenTextureDialog: openTextureDialog,
		onNextProfilePage: list.goNext,
		onPreviousProfilePage: list.goPrevious,
		onRenameProfile: renameProfile,
		onSelectTextureFile: selectTextureFile,
		onUploadTexture: uploadTexture,
	};
}

export type MinecraftProfilesPageController = ReturnType<
	typeof useMinecraftProfilesPageController
>;
