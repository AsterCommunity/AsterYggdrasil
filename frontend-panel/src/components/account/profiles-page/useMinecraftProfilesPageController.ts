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
import { usePageTitle } from "@/hooks/usePageTitle";
import { validateMinecraftTextureFile } from "@/lib/minecraftTextureValidation";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { MinecraftTextureType } from "@/types/api";
import { useMinecraftProfilesPageState } from "./useMinecraftProfilesPageState";

const DEFAULT_PROFILE_PAGE_SIZE = 5;
const PROFILE_PAGE_SIZE_OPTIONS = [5, 10] as const;
const PROFILE_SEARCH_DEBOUNCE_MS = 300;

export function useMinecraftProfilesPageController() {
	const { t } = useTranslation();
	const [state, dispatch] = useMinecraftProfilesPageState();
	const {
		debouncedQuery,
		deletingProfile,
		file,
		loading,
		model,
		profileCursorStack,
		profileNextCursor,
		profilePageSize,
		profileName,
		profileSkinUrls,
		profileTotal,
		profiles,
		profilesLoading,
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

	const loadProfiles = useCallback(
		async (
			nextCursorStack = profileCursorStack,
			nextPageSize = profilePageSize,
			selectedUuidAfterLoad?: string,
		) => {
			const trimmedQuery = debouncedQuery.trim();
			const cursor = nextCursorStack.at(-1);
			const params = {
				after_id: cursor?.id,
				limit: nextPageSize,
			};
			dispatch({ type: "profilesLoading", value: true });
			try {
				const next = await yggdrasilService.listProfiles(
					trimmedQuery ? { ...params, query: trimmedQuery } : params,
				);
				if (
					next.items.length === 0 &&
					next.total > 0 &&
					nextCursorStack.length > 0
				) {
					dispatch({
						type: "profileCursorStack",
						value: nextCursorStack.slice(0, -1),
					});
					dispatch({ type: "profileNextCursor", value: null });
					return;
				}
				dispatch({
					type: "profilePage",
					value: {
						...next,
						cursorStack: nextCursorStack,
						selectedUuid: selectedUuidAfterLoad,
					},
				});
			} finally {
				dispatch({ type: "profilesLoading", value: false });
			}
		},
		[debouncedQuery, dispatch, profileCursorStack, profilePageSize],
	);

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
		void loadProfiles().catch((nextError) =>
			toast.error(formatUnknownError(nextError)),
		);
	}, [loadProfiles]);

	useEffect(() => {
		const timeout = window.setTimeout(() => {
			dispatch({ type: "profileCursorStack", value: [] });
			dispatch({ type: "profileNextCursor", value: null });
			dispatch({ type: "debouncedQuery", value: query.trim() });
		}, PROFILE_SEARCH_DEBOUNCE_MS);
		return () => window.clearTimeout(timeout);
	}, [dispatch, query]);

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
			await loadProfiles([], profilePageSize, created.id);
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
			await loadProfiles([], profilePageSize, renamed.id);
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
			await loadProfiles([], profilePageSize);
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
		dispatch({ type: "profilePageSize", value: nextPageSize });
		dispatch({ type: "profileCursorStack", value: [] });
		dispatch({ type: "profileNextCursor", value: null });
		void loadProfiles([], nextPageSize).catch((nextError) =>
			toast.error(formatUnknownError(nextError)),
		);
	}

	function nextProfilePage() {
		if (!profileNextCursor) return;
		dispatch({
			type: "profileCursorStack",
			value: [...profileCursorStack, profileNextCursor],
		});
	}

	function previousProfilePage() {
		dispatch({
			type: "profileCursorStack",
			value: profileCursorStack.slice(0, -1),
		});
	}

	return {
		activeTexture,
		capeTexture,
		deletingProfile,
		dispatch,
		loading,
		model,
		profileName,
		profileCurrentPage: profileCursorStack.length + 1,
		profileNextDisabled: profileNextCursor == null,
		profilePageSize,
		profilePrevDisabled: profileCursorStack.length === 0,
		profileSkinUrls,
		profileTotal,
		profileTotalPages: Math.max(
			profileCursorStack.length + (profileNextCursor ? 2 : 1),
			1,
		),
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
		onNextProfilePage: nextProfilePage,
		onPreviousProfilePage: previousProfilePage,
		onRenameProfile: renameProfile,
		onSelectTextureFile: selectTextureFile,
		onUploadTexture: uploadTexture,
	};
}
