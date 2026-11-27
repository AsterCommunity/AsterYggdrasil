import {
	type DragEvent,
	type FormEvent,
	useCallback,
	useEffect,
	useMemo,
	useReducer,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useCursorList } from "@/hooks/useCursorList";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { useTextureTagPager } from "@/hooks/useTextureTagPager";
import { validateMinecraftTextureFile } from "@/lib/minecraftTextureValidation";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import { textureTagStore } from "@/stores/textureTagStore";
import type {
	DateTimeIdCursor,
	MinecraftTextureModel,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	MinecraftWardrobeTextureMetadata,
	TextureTagSearchMethod,
	UpdateWardrobeTextureRequest,
	YggdrasilProfile,
} from "@/types/api";

const WARDROBE_SEARCH_DEBOUNCE_MS = 300;
const TAG_FILTER_APPLY_DEBOUNCE_MS = 240;

type TextureWardrobePageState = {
	activeTexture: MinecraftWardrobeTextureMetadata | null;
	deleteDialogOpen: boolean;
	deleteTexture: MinecraftWardrobeTextureMetadata | null;
	dialogOpen: boolean;
	/**
	 * Metadata write already accepted by the server for the current edit
	 * session, if any. A tag-write retry skips the metadata request only when
	 * its request payload matches the accepted one.
	 */
	editMetadataApplied: {
		request: UpdateWardrobeTextureRequest;
		response: MinecraftWardrobeTextureMetadata;
		textureId: number;
	} | null;
	editDialogOpen: boolean;
	/** Incremented per edit-dialog open; late save completions carry it. */
	editSession: number;
	editTexture: MinecraftWardrobeTextureMetadata | null;
	file: File | null;
	model: MinecraftTextureModel;
	profileQuery: string;
	profiles: YggdrasilProfile[];
	query: string;
	selectedProfileId: string;
	submitting: boolean;
	textureType: MinecraftTextureType;
	uploadName: string;
	visibility: MinecraftTextureVisibility;
};

type TextureWardrobePageAction =
	| { type: "activeTexture"; value: MinecraftWardrobeTextureMetadata | null }
	| { type: "deleteDialogOpen"; value: boolean }
	| { type: "deleteTexture"; value: MinecraftWardrobeTextureMetadata | null }
	| { type: "dialogOpen"; value: boolean }
	| {
			type: "edit_metadata_applied";
			request: UpdateWardrobeTextureRequest;
			response: MinecraftWardrobeTextureMetadata;
			session: number;
			textureId: number;
	  }
	| { type: "close_edit_dialog"; session?: number }
	| { type: "editDialogOpen"; value: boolean }
	| { type: "edit_save_settled"; session: number }
	| { type: "editTexture"; value: MinecraftWardrobeTextureMetadata | null }
	| { type: "file"; value: File | null }
	| { type: "model"; value: MinecraftTextureModel }
	| { type: "open_edit_dialog"; texture: MinecraftWardrobeTextureMetadata }
	| { type: "profilesSuccess"; profiles: YggdrasilProfile[] }
	| { type: "profileQuery"; value: string }
	| { type: "query"; value: string }
	| { type: "selectedProfileId"; value: string | ((current: string) => string) }
	| { type: "submitting"; value: boolean }
	| { type: "textureType"; value: MinecraftTextureType }
	| { type: "uploadName"; value: string }
	| { type: "visibility"; value: MinecraftTextureVisibility }
	| { type: "replaceTexture"; value: MinecraftWardrobeTextureMetadata };

const initialDialogState: TextureWardrobePageState = {
	activeTexture: null,
	deleteDialogOpen: false,
	deleteTexture: null,
	dialogOpen: false,
	editMetadataApplied: null,
	editDialogOpen: false,
	editSession: 0,
	editTexture: null,
	file: null,
	model: "default",
	profileQuery: "",
	profiles: [],
	query: "",
	selectedProfileId: "",
	submitting: false,
	textureType: "skin",
	uploadName: "",
	visibility: "private",
};

function dialogReducer(
	state: TextureWardrobePageState,
	action: TextureWardrobePageAction,
): TextureWardrobePageState {
	switch (action.type) {
		case "activeTexture":
		case "deleteDialogOpen":
		case "deleteTexture":
		case "dialogOpen":
		case "editDialogOpen":
		case "editTexture":
		case "file":
		case "model":
		case "profileQuery":
		case "query":
		case "submitting":
		case "textureType":
		case "uploadName":
		case "visibility":
			return { ...state, [action.type]: action.value };
		case "open_edit_dialog":
			return {
				...state,
				editDialogOpen: true,
				// A fresh session starts with no accepted metadata write; the
				// partial-save carry-over from a previous session must not
				// apply to a different texture or a re-opened one.
				editMetadataApplied: null,
				editSession: state.editSession + 1,
				editTexture: action.texture,
			};
		case "close_edit_dialog": {
			// A session-tagged close only applies to the dialog it was issued
			// for: the admin may have dismissed dialog A (Esc/overlay/header)
			// and opened dialog B, whose drafts must survive A's completion.
			if (
				action.session !== undefined &&
				action.session !== state.editSession
			) {
				return { ...state, submitting: false };
			}
			return {
				...state,
				editDialogOpen: false,
				editMetadataApplied: null,
				submitting: false,
			};
		}
		case "edit_metadata_applied": {
			// A late completion may only record into its own session.
			if (action.session !== state.editSession) return state;
			return {
				...state,
				editMetadataApplied: {
					request: action.request,
					response: action.response,
					textureId: action.textureId,
				},
			};
		}
		case "edit_save_settled":
			// A late save completion must not clear the submitting flag of a
			// newer session's own in-flight save.
			return state.editSession === action.session
				? { ...state, submitting: false }
				: state;
		case "profilesSuccess":
			return {
				...state,
				profiles: action.profiles,
				selectedProfileId:
					state.selectedProfileId || action.profiles[0]?.id || "",
			};
		case "replaceTexture":
			return {
				...state,
				activeTexture:
					state.activeTexture?.id === action.value.id
						? action.value
						: state.activeTexture,
				deleteTexture:
					state.deleteTexture?.id === action.value.id
						? action.value
						: state.deleteTexture,
				editTexture:
					state.editTexture?.id === action.value.id
						? action.value
						: state.editTexture,
			};
		case "selectedProfileId":
			return {
				...state,
				selectedProfileId:
					typeof action.value === "function"
						? action.value(state.selectedProfileId)
						: action.value,
			};
	}
}

function sameNumberArray(left: number[], right: number[]) {
	return (
		left.length === right.length &&
		left.every((value, index) => value === right[index])
	);
}

export function useTextureWardrobePageController() {
	const { t } = useTranslation();
	const [state, dispatch] = useReducer(dialogReducer, initialDialogState);
	const [texturePageSize, setTexturePageSize] = useState<number>(10);
	const [activeTab, setActiveTab] = useState<MinecraftTextureType>("skin");
	const [uploadDialogOpen, setUploadDialogOpen] = useState(false);
	const [dragActive, setDragActive] = useState(false);
	const [selectedTagIds, setSelectedTagIds] = useState<number[]>([]);
	const [draftTagIds, setDraftTagIds] = useState<number[]>([]);
	const [tagFilterOpen, setTagFilterOpen] = useState(false);
	const [tagSearchMethod, setTagSearchMethod] =
		useState<TextureTagSearchMethod>("all");
	const [editTagIds, setEditTagIds] = useState<number[]>([]);
	const [editTagQuery, setEditTagQuery] = useState("");
	const yggdrasilConfig = useFrontendConfigStore((store) => store.yggdrasil);
	const {
		activeTexture,
		deleteDialogOpen,
		deleteTexture,
		dialogOpen,
		editDialogOpen,
		editMetadataApplied,
		editSession,
		editTexture,
		file,
		model,
		profileQuery,
		profiles,
		query,
		selectedProfileId,
		submitting,
		textureType,
		uploadName,
		visibility,
	} = state;

	const libraryTextureType: MinecraftTextureType =
		activeTab === "cape" ? "cape" : "skin";

	const loadProfiles = useCallback(async () => {
		try {
			const nextProfiles = await yggdrasilService.listProfiles();
			dispatch({ type: "profilesSuccess", profiles: nextProfiles.items });
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		}
	}, []);

	const tagPager = useTextureTagPager({
		loadPage: (params) =>
			textureTagStore.getTextureTagPage(params, () =>
				yggdrasilService.listTextureLibraryTagsPage(params),
			),
		onError: (error) => toast.error(formatUnknownError(error)),
		retainedTagIds: [...draftTagIds, ...editTagIds],
	});
	const {
		addTags,
		ensureLoaded: ensureTagsLoaded,
		hasMore: hasMoreTags,
		loadMore: loadMoreTags,
		loading: tagLoading,
		resetEnsureLoaded: resetEnsureTagsLoaded,
		search: searchTags,
		tags,
	} = tagPager;

	// The search draft and draft tag selection commit after short debounces
	// so bursts of input collapse into one wardrobe request.
	const debouncedQuery = useDebouncedValue(
		query.trim(),
		WARDROBE_SEARCH_DEBOUNCE_MS,
	);
	const debouncedDraftTagIds = useDebouncedValue(
		draftTagIds,
		TAG_FILTER_APPLY_DEBOUNCE_MS,
	);

	const loadTexturePage = useCallback(
		async ({ cursor }: { cursor: DateTimeIdCursor | null }) => {
			const tagIds = Array.from(new Set(selectedTagIds));
			const nextTextures = await yggdrasilService.listWardrobeTextures({
				limit: texturePageSize,
				after_updated_at: cursor?.value,
				after_id: cursor?.id,
				texture_type: libraryTextureType,
				keyword: debouncedQuery || undefined,
				tag_ids: tagIds.length > 0 ? tagIds : undefined,
				tag_search_method: tagIds.length > 0 ? tagSearchMethod : undefined,
			});
			return {
				items: nextTextures.items,
				nextCursor: nextTextures.next_cursor ?? null,
				total: nextTextures.total,
			};
		},
		[
			debouncedQuery,
			libraryTextureType,
			selectedTagIds,
			tagSearchMethod,
			texturePageSize,
		],
	);
	const textureList = useCursorList<
		MinecraftWardrobeTextureMetadata,
		DateTimeIdCursor
	>({
		deps: [
			texturePageSize,
			libraryTextureType,
			debouncedQuery,
			selectedTagIds,
			tagSearchMethod,
		],
		loadPage: loadTexturePage,
		onError: (error) => toast.error(formatUnknownError(error)),
	});
	const { setItems: setTextureItems, setTotal: setTextureTotalItems } =
		textureList;

	useEffect(() => {
		if (tagFilterOpen || editDialogOpen) {
			ensureTagsLoaded();
		}
	}, [editDialogOpen, ensureTagsLoaded, tagFilterOpen]);

	useEffect(() => {
		setSelectedTagIds((current) =>
			sameNumberArray(current, debouncedDraftTagIds)
				? current
				: debouncedDraftTagIds,
		);
	}, [debouncedDraftTagIds]);

	const { loading, total: textureTotal } = textureList;
	const textures = textureList.items;
	const visibleTextures = textures;
	const searchBusy =
		query.trim() !== debouncedQuery.trim() ||
		(loading && Boolean(debouncedQuery.trim()));

	const previewTexture = useMemo(() => {
		if (
			activeTexture &&
			textures.some((texture) => texture.id === activeTexture.id)
		) {
			return activeTexture;
		}
		return visibleTextures[0] ?? textures[0] ?? null;
	}, [activeTexture, textures, visibleTextures]);

	const filteredProfiles = useMemo(() => {
		const trimmed = profileQuery.trim().toLowerCase();
		if (!trimmed) return profiles;
		return profiles.filter(
			(profile) =>
				profile.name.toLowerCase().includes(trimmed) ||
				profile.id.toLowerCase().includes(trimmed),
		);
	}, [profiles, profileQuery]);

	const selectedTags = useMemo(
		() => tags.filter((tag) => draftTagIds.includes(tag.id)),
		[tags, draftTagIds],
	);

	async function uploadTexture(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!file) return;
		if (!(await validateTextureFile(file))) return;
		dispatch({ type: "submitting", value: true });
		try {
			const uploaded = await yggdrasilService.uploadWardrobeTexture({
				textureType,
				model,
				file,
				name: uploadName,
				visibility,
			});
			toast.success(t("wardrobe.uploadSuccess"));
			// Reload the first page for the new texture's tab; the tab switch
			// is also a list dep, so the cursor list resets itself.
			setActiveTab(uploaded.texture_type);
			dispatch({ type: "activeTexture", value: uploaded });
			dispatch({ type: "file", value: null });
			dispatch({ type: "uploadName", value: "" });
			setUploadDialogOpen(false);
			setDragActive(false);
			const page = await textureList.reloadFirstPage();
			if (uploaded.texture_type === libraryTextureType) {
				setTextureItems((current) => [uploaded, ...current]);
				setTextureTotalItems((current) => current + 1);
			}
			void page;
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	async function validateTextureFile(nextFile: File) {
		const validation = await validateMinecraftTextureFile(
			nextFile,
			textureType,
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

	function openUploadDialog(nextType = libraryTextureType) {
		dispatch({ type: "textureType", value: nextType });
		dispatch({ type: "file", value: null });
		dispatch({ type: "uploadName", value: "" });
		setDragActive(false);
		setUploadDialogOpen(true);
	}

	function dropTextureFile(event: DragEvent<HTMLLabelElement>) {
		event.preventDefault();
		setDragActive(false);
		void selectTextureFile(event.dataTransfer.files.item(0));
	}

	function dragTextureFile(event: DragEvent<HTMLLabelElement>) {
		event.preventDefault();
		setDragActive(true);
	}

	function leaveTextureDropZone() {
		setDragActive(false);
	}

	function openBindDialog(texture: MinecraftWardrobeTextureMetadata) {
		dispatch({ type: "activeTexture", value: texture });
		dispatch({
			type: "selectedProfileId",
			value: (current) => current || profiles[0]?.id || "",
		});
		dispatch({ type: "dialogOpen", value: true });
		if (profiles.length === 0) {
			void loadProfiles();
		}
	}

	async function bindTexture() {
		if (!activeTexture || !selectedProfileId) return;
		dispatch({ type: "submitting", value: true });
		try {
			await yggdrasilService.bindProfileTexture({
				uuid: selectedProfileId,
				textureType: activeTexture.texture_type,
				textureId: activeTexture.id,
			});
			const profile = profiles.find((item) => item.id === selectedProfileId);
			toast.success(
				t("wardrobe.bindSuccess", {
					name: profile?.name ?? selectedProfileId,
				}),
			);
			dispatch({ type: "dialogOpen", value: false });
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	function openDeleteDialog(texture: MinecraftWardrobeTextureMetadata) {
		dispatch({ type: "deleteTexture", value: texture });
		dispatch({ type: "deleteDialogOpen", value: true });
	}

	function openEditDialog(texture: MinecraftWardrobeTextureMetadata) {
		dispatch({ type: "open_edit_dialog", texture });
		addTags(texture.tags);
		resetEnsureTagsLoaded();
		setEditTagIds(texture.tags.map((tag) => tag.id));
		setEditTagQuery("");
	}

	function clearUploadDialogStateAfterClose() {
		if (uploadDialogOpen) return;
		dispatch({ type: "file", value: null });
		dispatch({ type: "uploadName", value: "" });
		setDragActive(false);
	}

	function clearEditDialogStateAfterClose() {
		if (editDialogOpen) return;
		dispatch({ type: "editTexture", value: null });
		setEditTagIds([]);
		setEditTagQuery("");
	}

	function clearBindDialogStateAfterClose() {
		if (dialogOpen) return;
		dispatch({ type: "profileQuery", value: "" });
	}

	function clearDeleteDialogStateAfterClose() {
		if (deleteDialogOpen) return;
		dispatch({ type: "deleteTexture", value: null });
	}

	async function updateWardrobeTexture(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!editTexture) return;
		const form = new FormData(event.currentTarget);
		const nextName = String(form.get("name") ?? "").trim();
		const nextVisibility = String(
			form.get("visibility") || editTexture.visibility,
		);
		const nextModel = String(
			form.get("texture_model") || editTexture.texture_model,
		);
		const metadataUpdate: UpdateWardrobeTextureRequest = {
			display_name: nextName || null,
			visibility:
				nextVisibility === "public" || nextVisibility === "private"
					? nextVisibility
					: editTexture.visibility,
		};
		if (editTexture.texture_type === "skin") {
			metadataUpdate.texture_model =
				nextModel === "slim" || nextModel === "default"
					? nextModel
					: editTexture.texture_model;
		}
		// The save carries the edit session it was issued in: late completions
		// (metadata applied, dialog close, submitting reset) only affect that
		// session, never a newer dialog opened after this one was dismissed.
		const session = editSession;
		const textureId = editTexture.id;
		const applied = editMetadataApplied;
		const metadataAlreadyAccepted =
			applied?.textureId === textureId &&
			applied.request.display_name === metadataUpdate.display_name &&
			applied.request.visibility === metadataUpdate.visibility &&
			applied.request.texture_model === metadataUpdate.texture_model;
		dispatch({ type: "submitting", value: true });
		try {
			// The save is two writes: metadata, then tags. Once the server
			// accepted the metadata write, apply its response immediately —
			// cards and preview must reflect what the server already has even
			// if the tag write below fails, and a retry only re-runs the
			// missing tag write when the request is unchanged (an edited
			// name/visibility/model resubmits the metadata).
			let updated = metadataAlreadyAccepted ? applied.response : null;
			if (!updated) {
				updated = await yggdrasilService.updateWardrobeTexture(
					textureId,
					metadataUpdate,
				);
				dispatch({
					type: "edit_metadata_applied",
					request: metadataUpdate,
					response: updated,
					session,
					textureId,
				});
				replaceTextureEverywhere({ ...updated, tags: editTexture.tags });
			}
			try {
				const tagged = await yggdrasilService.replaceWardrobeTextureTags(
					textureId,
					{
						tag_ids: Array.from(new Set(editTagIds)),
					},
				);
				replaceTextureEverywhere({ ...updated, tags: tagged.tags });
				dispatch({ type: "close_edit_dialog", session });
				toast.success(t("wardrobe.editSuccess"));
			} catch (tagError) {
				// Metadata is saved; keep the dialog open with the tag draft
				// intact so the retry only completes the tag write.
				toast.error(formatUnknownError(tagError));
			}
		} catch (metadataError) {
			toast.error(formatUnknownError(metadataError));
		} finally {
			dispatch({ type: "edit_save_settled", session });
		}
	}

	async function deleteWardrobeTexture() {
		if (!deleteTexture) return;
		dispatch({ type: "submitting", value: true });
		try {
			await yggdrasilService.deleteWardrobeTexture(deleteTexture.id);
			if (activeTexture?.id === deleteTexture.id) {
				dispatch({ type: "activeTexture", value: null });
			}
			toast.success(t("wardrobe.deleteSuccess"));
			dispatch({ type: "deleteDialogOpen", value: false });
			await textureList.reload();
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	function replaceTextureEverywhere(updated: MinecraftWardrobeTextureMetadata) {
		dispatch({ type: "replaceTexture", value: updated });
		setTextureItems((current) =>
			current.map((texture) => (texture.id === updated.id ? updated : texture)),
		);
	}

	async function submitTextureLibrary(
		texture: MinecraftWardrobeTextureMetadata,
	) {
		dispatch({ type: "submitting", value: true });
		try {
			const updated = await yggdrasilService.submitTextureLibraryReview(
				texture.id,
			);
			replaceTextureEverywhere(updated);
			toast.success(
				updated.library_status === "published"
					? t("wardrobe.librarySubmitSuccessPublished")
					: t("wardrobe.librarySubmitSuccessPending"),
			);
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	async function withdrawTextureLibrary(
		texture: MinecraftWardrobeTextureMetadata,
	) {
		dispatch({ type: "submitting", value: true });
		try {
			const updated = await yggdrasilService.withdrawTextureLibrarySubmission(
				texture.id,
			);
			replaceTextureEverywhere(updated);
			toast.success(t("wardrobe.libraryWithdrawSuccess"));
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	function selectTab(tab: MinecraftTextureType) {
		// The tab is a list dep, so the cursor list resets itself.
		setActiveTab(tab);
		dispatch({ type: "textureType", value: tab });
		dispatch({ type: "activeTexture", value: null });
	}

	function toggleEditTag(tagId: number) {
		setEditTagIds((current) =>
			current.includes(tagId)
				? current.filter((id) => id !== tagId)
				: [...current, tagId],
		);
	}

	function clearTagFilter() {
		setDraftTagIds([]);
		dispatch({ type: "activeTexture", value: null });
	}

	function changeTagSearchMethod(nextMethod: TextureTagSearchMethod) {
		// The tag search method is a list dep, so the cursor list resets itself.
		setTagSearchMethod(nextMethod);
		dispatch({ type: "activeTexture", value: null });
	}

	return {
		activeTab,
		activeTexture,
		bindTexture,
		changeTagSearchMethod,
		clearBindDialogStateAfterClose,
		clearDeleteDialogStateAfterClose,
		clearEditDialogStateAfterClose,
		clearTagFilter,
		clearUploadDialogStateAfterClose,
		deleteDialogOpen,
		dispatch,
		deleteTexture,
		deleteWardrobeTexture,
		dialogOpen,
		draftTagIds,
		dragActive,
		dragTextureFile,
		dropTextureFile,
		editDialogOpen,
		editTagIds,
		editTagQuery,
		editTexture,
		file,
		filteredProfiles,
		hasMoreTags,
		leaveTextureDropZone,
		loadMoreTags,
		loading,
		model,
		openBindDialog,
		openDeleteDialog,
		openEditDialog,
		openUploadDialog,
		previewTexture,
		profileQuery,
		profiles,
		query,
		searchBusy,
		searchTags,
		selectedProfileId,
		selectedTags,
		selectTab,
		selectTextureFile,
		setDragActive,
		setDraftTagIds,
		setEditTagQuery,
		setTagFilterOpen,
		setTexturePageSize,
		setUploadDialogOpen,
		tags,
		submitting,
		submitTextureLibrary,
		tagFilterOpen,
		tagLoading,
		tagSearchMethod,
		textureList,
		texturePageSize,
		textureTotal,
		textureType,
		toggleEditTag,
		updateWardrobeTexture,
		uploadDialogOpen,
		uploadName,
		uploadTexture,
		visibility,
		visibleTextures,
		withdrawTextureLibrary,
	};
}
