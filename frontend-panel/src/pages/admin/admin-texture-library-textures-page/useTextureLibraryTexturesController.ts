import {
	type FormEvent,
	useCallback,
	useEffect,
	useMemo,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { handleApiError } from "@/hooks/useApiError";
import { useCursorList } from "@/hooks/useCursorList";
import {
	managedEnumField,
	managedPageSizeField,
	managedRemovedField,
	managedStringField,
	useManagedListQueryState,
} from "@/hooks/useManagedListQueryState";
import { usePendingId } from "@/hooks/usePendingId";
import { adminPaths } from "@/routes/routePaths";
import { adminTextureLibraryService } from "@/services/adminService";
import type {
	MinecraftTextureLibraryStatus,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	ReviewTextureLibraryTextureRequest,
} from "@/types/api";
import {
	ALL_VALUE,
	DEFAULT_TEXTURE_PAGE_SIZE,
	type DeleteDialogState,
	type FilterValue,
	type LibraryTexture,
	type PublishedFilter,
	parseTextureId,
	publishedToQuery,
	type ReviewAction,
	type ReviewDialogState,
	TEXTURE_PAGE_SIZE_OPTIONS,
	type TextureCursor,
	type TexturePageMode,
	type TexturePageSize,
} from "./libraryTextureView";

export function useTextureLibraryTexturesController(mode: TexturePageMode) {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const [searchParams, setSearchParams] = useSearchParams();
	const params = useParams();
	const textureId = parseTextureId(params.textureId);
	const detailMode = mode === "detail";
	const [detailTexture, setDetailTexture] = useState<LibraryTexture | null>(
		null,
	);
	const [detailLoading, setDetailLoading] = useState(detailMode);
	const [reviewDialog, setReviewDialog] = useState<ReviewDialogState | null>(
		null,
	);
	const [deleteDialog, setDeleteDialog] = useState<DeleteDialogState | null>(
		null,
	);
	const [reviewNote, setReviewNote] = useState("");
	const { pendingId: submittingTextureId, runWithPending: runTextureAction } =
		usePendingId<number>();

	const modeDefaults = useMemo(
		(): {
			libraryStatus: FilterValue<MinecraftTextureLibraryStatus>;
			published: PublishedFilter;
			visibility: FilterValue<MinecraftTextureVisibility>;
		} => ({
			libraryStatus:
				mode === "reviews"
					? ("pending_review" as MinecraftTextureLibraryStatus)
					: ALL_VALUE,
			published: mode === "reviews" ? "not_published" : ALL_VALUE,
			visibility: mode === "reviews" ? "public" : ALL_VALUE,
		}),
		[mode],
	);
	const querySchema = useMemo(
		() => ({
			keyword: managedStringField("keyword"),
			libraryStatus: managedEnumField(
				"libraryStatus",
				["private", "pending_review", "published", "rejected"],
				modeDefaults.libraryStatus,
			),
			offset: managedRemovedField("offset"),
			pageSize: managedPageSizeField(
				TEXTURE_PAGE_SIZE_OPTIONS,
				DEFAULT_TEXTURE_PAGE_SIZE,
			),
			published: managedEnumField(
				"published",
				["published", "not_published"],
				modeDefaults.published,
			),
			textureType: managedEnumField("textureType", ["skin", "cape"], ALL_VALUE),
			visibility: managedEnumField(
				"visibility",
				["private", "public"],
				modeDefaults.visibility,
			),
		}),
		[modeDefaults],
	);
	const { query: filters, setQuery } = useManagedListQueryState<{
		keyword: string;
		libraryStatus: FilterValue<MinecraftTextureLibraryStatus>;
		offset: null;
		pageSize: TexturePageSize;
		published: PublishedFilter;
		textureType: FilterValue<MinecraftTextureType>;
		visibility: FilterValue<MinecraftTextureVisibility>;
	}>({
		schema: querySchema,
		searchParams,
		setSearchParams,
	});
	const {
		keyword,
		libraryStatus,
		pageSize,
		published,
		textureType,
		visibility,
	} = filters;

	const listQuery = useMemo(
		() => ({
			limit: pageSize,
			keyword: keyword || undefined,
			texture_type: textureType === ALL_VALUE ? undefined : textureType,
			visibility: visibility === ALL_VALUE ? undefined : visibility,
			library_status: libraryStatus === ALL_VALUE ? undefined : libraryStatus,
			published: publishedToQuery(published),
		}),
		[keyword, libraryStatus, pageSize, published, textureType, visibility],
	);

	const loadTexturePage = useCallback(
		async ({ cursor }: { cursor: TextureCursor | null }) => {
			const page = await adminTextureLibraryService.listTextures({
				...listQuery,
				after_updated_at: cursor?.value,
				after_id: cursor?.id,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[listQuery],
	);
	const list = useCursorList<LibraryTexture, TextureCursor>({
		deps: [
			pageSize,
			keyword,
			textureType,
			visibility,
			libraryStatus,
			published,
		],
		enabled: !detailMode,
		loadPage: loadTexturePage,
		onError: handleApiError,
	});
	const reload = list.reload;

	const loadDetailTexture = useCallback(async () => {
		if (!detailMode) return;
		if (!textureId) {
			setDetailTexture(null);
			setDetailLoading(false);
			return;
		}
		setDetailLoading(true);
		try {
			setDetailTexture(await adminTextureLibraryService.getTexture(textureId));
		} catch (error) {
			setDetailTexture(null);
			handleApiError(error);
		} finally {
			setDetailLoading(false);
		}
	}, [detailMode, textureId]);

	useEffect(() => {
		void loadDetailTexture();
	}, [loadDetailTexture]);

	const currentPage = list.currentPage;
	const totalPages = Math.max(1, Math.ceil(list.total / pageSize));
	const activeFilterCount =
		(keyword ? 1 : 0) +
		(textureType !== ALL_VALUE ? 1 : 0) +
		(visibility !== modeDefaults.visibility ? 1 : 0) +
		(libraryStatus !== modeDefaults.libraryStatus ? 1 : 0) +
		(published !== modeDefaults.published ? 1 : 0);
	const filtered = activeFilterCount > 0;

	const setFilters = useCallback(
		(nextValues: {
			keyword?: string;
			libraryStatus?: FilterValue<MinecraftTextureLibraryStatus>;
			pageSize?: TexturePageSize;
			published?: PublishedFilter;
			textureType?: FilterValue<MinecraftTextureType>;
			visibility?: FilterValue<MinecraftTextureVisibility>;
		}) => {
			// The cursor list resets itself: every filter value is a list dep.
			setQuery(nextValues);
		},
		[setQuery],
	);

	const resetFilters = useCallback(() => {
		setQuery({
			keyword: "",
			libraryStatus: modeDefaults.libraryStatus,
			pageSize: DEFAULT_TEXTURE_PAGE_SIZE,
			published: modeDefaults.published,
			textureType: ALL_VALUE,
			visibility: modeDefaults.visibility,
		});
	}, [modeDefaults, setQuery]);

	function openReviewDialog(action: ReviewAction, texture: LibraryTexture) {
		setReviewDialog({ action, texture });
		setReviewNote(texture.library_review_note ?? "");
	}

	function closeReviewDialog() {
		if (submittingTextureId !== null) return;
		setReviewDialog(null);
		setReviewNote("");
	}

	function closeDeleteDialog(open: boolean) {
		if (open || submittingTextureId !== null) return;
		setDeleteDialog(null);
	}

	async function submitReviewAction(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!reviewDialog) return;
		if (reviewDialog.action === "reject" && !reviewNote.trim()) {
			toast.error(t("admin.textureLibraryTexturesPage.reviewNoteRequired"));
			return;
		}

		const actionTextureId = reviewDialog.texture.id;
		const payload: ReviewTextureLibraryTextureRequest = {
			review_note: reviewNote.trim() || null,
		};
		await runTextureAction(actionTextureId, async () => {
			try {
				const updated =
					reviewDialog.action === "approve"
						? await adminTextureLibraryService.approveTexture(
								actionTextureId,
								payload,
							)
						: reviewDialog.action === "reject"
							? await adminTextureLibraryService.rejectTexture(
									actionTextureId,
									payload,
								)
							: await adminTextureLibraryService.unpublishTexture(
									actionTextureId,
									payload,
								);
				list.setItems((current) =>
					current.map((texture) =>
						texture.id === updated.id ? updated : texture,
					),
				);
				setDetailTexture((current) =>
					current?.id === updated.id ? updated : current,
				);
				toast.success(
					t(`admin.textureLibraryTexturesPage.${reviewDialog.action}Success`),
				);
				setReviewDialog(null);
				setReviewNote("");
				if (!detailMode) await reload();
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	async function confirmDeleteTexture() {
		if (!deleteDialog) return;
		const deletingTextureId = deleteDialog.texture.id;
		await runTextureAction(deletingTextureId, async () => {
			try {
				await adminTextureLibraryService.deleteTexture(deletingTextureId);
				list.setItems((current) =>
					current.filter((texture) => texture.id !== deletingTextureId),
				);
				setDetailTexture((current) =>
					current?.id === deletingTextureId ? null : current,
				);
				toast.success(t("admin.textureLibraryTexturesPage.deleteSuccess"));
				setDeleteDialog(null);
				if (detailMode) {
					navigate(adminPaths.textureLibrary, { replace: true });
				} else {
					await reload();
				}
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	return {
		activeFilterCount,
		closeDeleteDialog,
		closeReviewDialog,
		confirmDeleteTexture,
		currentPage,
		deleteDialog,
		detailLoading,
		detailMode,
		detailTexture,
		filtered,
		filters: {
			keyword,
			libraryStatus,
			pageSize,
			published,
			textureType,
			visibility,
		},
		list,
		loadDetailTexture,
		openReviewDialog,
		resetFilters,
		reviewDialog,
		reviewNote,
		setDeleteDialog,
		setFilters,
		setReviewNote,
		submitReviewAction,
		submittingTextureId,
		textureId,
		totalPages,
	};
}
