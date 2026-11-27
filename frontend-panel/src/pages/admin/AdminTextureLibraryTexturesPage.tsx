import {
	type FormEvent,
	type KeyboardEvent,
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
	Link,
	useNavigate,
	useParams,
	useSearchParams,
} from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { TextureLibrarySectionNav } from "@/components/admin/TextureLibrarySectionNav";
import { AdminFilterToolbar } from "@/components/common/AdminFilterToolbar";
import {
	ADMIN_TABLE_MONO_TEXT_CLASS,
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminTableCell,
	AdminTableHead,
	AdminTableHeader,
	AdminTableRow,
} from "@/components/common/AdminTable";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { DateTimeText } from "@/components/common/DateTimeText";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";
import { TextureLibraryTextureAvatar } from "@/components/yggdrasil/TextureLibraryTextureAvatar";
import { TextureTagChips } from "@/components/yggdrasil/TextureTagList";
import { handleApiError } from "@/hooks/useApiError";
import { useCursorList } from "@/hooks/useCursorList";
import {
	managedEnumField,
	managedPageSizeField,
	managedRemovedField,
	managedStringField,
	useManagedListQueryState,
} from "@/hooks/useManagedListQueryState";
import { usePageTitle } from "@/hooks/usePageTitle";
import { usePendingId } from "@/hooks/usePendingId";
import { formatBytes } from "@/lib/numberUnit";
import { parsePageSizeOption } from "@/lib/pagination";
import { cn } from "@/lib/utils";
import { adminPaths, adminTextureLibraryPath } from "@/routes/routePaths";
import { adminTextureLibraryService } from "@/services/adminService";
import type {
	MinecraftTextureLibraryStatus,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	PublicTextureLibraryTextureMetadata,
	ReviewTextureLibraryTextureRequest,
} from "@/types/api";

const TEXTURE_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
const DEFAULT_TEXTURE_PAGE_SIZE = 20;
const ALL_VALUE = "__all__";

type TexturePageMode = "all" | "detail" | "reviews";
type TexturePageSize = (typeof TEXTURE_PAGE_SIZE_OPTIONS)[number];
type LibraryTexture = PublicTextureLibraryTextureMetadata;
type ReviewAction = "approve" | "reject" | "unpublish";

type ReviewDialogState = {
	action: ReviewAction;
	texture: LibraryTexture;
};

type DeleteDialogState = {
	texture: LibraryTexture;
};

type FilterValue<T extends string> = T | typeof ALL_VALUE;
type PublishedFilter = "published" | "not_published" | typeof ALL_VALUE;
type TextureCursor = { id: number; value: string };

function publishedToQuery(value: PublishedFilter): boolean | undefined {
	if (value === "published") return true;
	if (value === "not_published") return false;
	return undefined;
}

function parseTextureId(value: string | undefined) {
	const parsed = Number(value);
	return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
}

export default function AdminTextureLibraryTexturesPage({
	mode,
}: {
	mode: TexturePageMode;
}) {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const params = useParams();
	const [searchParams, setSearchParams] = useSearchParams();
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

	usePageTitle(
		detailMode
			? (detailTexture?.name ?? t("admin.textureLibraryDetailPage.title"))
			: mode === "reviews"
				? t("admin.textureLibraryReviewPage.title")
				: t("admin.textureLibraryTexturesPage.title"),
	);

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

	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				total={list.total}
				currentPage={currentPage}
				totalPages={totalPages}
				pageSize={String(pageSize)}
				pageSizeOptions={TEXTURE_PAGE_SIZE_OPTIONS.map((size) => ({
					label: t("admin.pagination.pageSizeOption", { count: size }),
					value: String(size),
				}))}
				prevDisabled={list.prevPageDisabled}
				nextDisabled={list.nextPageDisabled}
				onPrevious={list.goPrevious}
				onNext={list.goNext}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(value, TEXTURE_PAGE_SIZE_OPTIONS);
					if (next == null) return;
					setFilters({ pageSize: next });
				}}
			/>
		),
		[
			currentPage,
			list.goNext,
			list.goPrevious,
			list.nextPageDisabled,
			list.prevPageDisabled,
			list.total,
			pageSize,
			setFilters,
			t,
			totalPages,
		],
	);

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

		const textureId = reviewDialog.texture.id;
		const payload: ReviewTextureLibraryTextureRequest = {
			review_note: reviewNote.trim() || null,
		};
		await runTextureAction(textureId, async () => {
			try {
				const updated =
					reviewDialog.action === "approve"
						? await adminTextureLibraryService.approveTexture(
								textureId,
								payload,
							)
						: reviewDialog.action === "reject"
							? await adminTextureLibraryService.rejectTexture(
									textureId,
									payload,
								)
							: await adminTextureLibraryService.unpublishTexture(
									textureId,
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
		const textureId = deleteDialog.texture.id;
		await runTextureAction(textureId, async () => {
			try {
				await adminTextureLibraryService.deleteTexture(textureId);
				list.setItems((current) =>
					current.filter((texture) => texture.id !== textureId),
				);
				setDetailTexture((current) =>
					current?.id === textureId ? null : current,
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

	if (detailMode) {
		return (
			<AdminPageShell>
				<AdminTextureDetailContent
					loading={detailLoading}
					submitting={submittingTextureId === detailTexture?.id}
					texture={detailTexture}
					validTextureId={textureId}
					onApprove={(texture) => openReviewDialog("approve", texture)}
					onDelete={(texture) => setDeleteDialog({ texture })}
					onReject={(texture) => openReviewDialog("reject", texture)}
					onRefresh={() => void loadDetailTexture()}
					onUnpublish={(texture) => openReviewDialog("unpublish", texture)}
				/>
				<TextureReviewDialog
					open={reviewDialog !== null}
					reviewDialog={reviewDialog}
					reviewNote={reviewNote}
					submitting={submittingTextureId !== null}
					onClose={closeReviewDialog}
					onNoteChange={setReviewNote}
					onSubmit={submitReviewAction}
				/>
				<TextureDeleteDialog
					open={deleteDialog !== null}
					deleteDialog={deleteDialog}
					submitting={submittingTextureId !== null}
					onClose={closeDeleteDialog}
					onConfirm={confirmDeleteTexture}
				/>
			</AdminPageShell>
		);
	}

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={
					mode === "reviews"
						? t("admin.textureLibraryReviewPage.title")
						: t("admin.textureLibraryTexturesPage.title")
				}
				description={
					mode === "reviews"
						? t("admin.textureLibraryReviewPage.description")
						: t("admin.textureLibraryTexturesPage.description")
				}
				actions={
					<>
						<TextureLibrarySectionNav
							active={mode === "reviews" ? "reviews" : "textures"}
						/>
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={list.loading}
							onClick={() => void reload()}
						>
							<Icon
								name={list.loading ? "Spinner" : "RefreshCw"}
								className={cn("size-4", list.loading && "animate-spin")}
							/>
							{t("common.refresh")}
						</Button>
					</>
				}
			/>

			<AdminTableList
				columns={7}
				emptyIcon={<Icon name="Images" className="size-5" />}
				emptyTitle={t("admin.textureLibraryTexturesPage.emptyTitle")}
				emptyDescription={t(
					"admin.textureLibraryTexturesPage.emptyDescription",
				)}
				filtered={filtered}
				filteredEmptyTitle={t(
					"admin.textureLibraryTexturesPage.filteredEmptyTitle",
				)}
				filteredEmptyDescription={t(
					"admin.textureLibraryTexturesPage.filteredEmptyDescription",
				)}
				headerRow={<TextureTableHeader />}
				items={list.items}
				loading={list.loading}
				pagination={pagination}
				renderRow={(texture) => (
					<TextureTableRow
						key={texture.id}
						loading={submittingTextureId === texture.id}
						texture={texture}
						onOpen={() => navigate(adminTextureLibraryPath(texture.id))}
						onApprove={() => openReviewDialog("approve", texture)}
						onDelete={() => setDeleteDialog({ texture })}
						onReject={() => openReviewDialog("reject", texture)}
						onUnpublish={() => openReviewDialog("unpublish", texture)}
					/>
				)}
				rows={8}
				toolbar={
					<AdminFilterToolbar
						activeFilterCount={activeFilterCount}
						inline
						onResetFilters={resetFilters}
					>
						<div className="relative min-w-[220px] flex-1 md:max-w-sm">
							<Icon
								name="MagnifyingGlass"
								className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
							/>
							<Input
								value={keyword}
								onChange={(event) =>
									setFilters({
										keyword: event.currentTarget.value,
									})
								}
								placeholder={t(
									"admin.textureLibraryTexturesPage.searchPlaceholder",
								)}
								className="pl-9"
							/>
						</div>
						<TextureSelect
							ariaLabel={t("admin.textureLibraryTexturesPage.typeLabel")}
							value={textureType}
							options={[
								{
									label: t("admin.textureLibraryTexturesPage.allTypes"),
									value: ALL_VALUE,
								},
								{ label: t("wardrobe.type.skin"), value: "skin" },
								{ label: t("wardrobe.type.cape"), value: "cape" },
							]}
							onChange={(value) =>
								setFilters({
									textureType: value as FilterValue<MinecraftTextureType>,
								})
							}
						/>
						<TextureSelect
							ariaLabel={t("admin.textureLibraryTexturesPage.visibilityLabel")}
							value={visibility}
							options={[
								{
									label: t("admin.textureLibraryTexturesPage.allVisibility"),
									value: ALL_VALUE,
								},
								{ label: t("wardrobe.visibility.public"), value: "public" },
								{ label: t("wardrobe.visibility.private"), value: "private" },
							]}
							onChange={(value) =>
								setFilters({
									visibility: value as FilterValue<MinecraftTextureVisibility>,
								})
							}
						/>
						<TextureSelect
							ariaLabel={t("admin.textureLibraryTexturesPage.statusLabel")}
							value={libraryStatus}
							options={[
								{
									label: t("admin.textureLibraryTexturesPage.allStatuses"),
									value: ALL_VALUE,
								},
								...libraryStatusOptions(t),
							]}
							onChange={(value) =>
								setFilters({
									libraryStatus:
										value as FilterValue<MinecraftTextureLibraryStatus>,
								})
							}
						/>
						<TextureSelect
							ariaLabel={t("admin.textureLibraryTexturesPage.publishedLabel")}
							value={published}
							options={[
								{
									label: t("admin.textureLibraryTexturesPage.allPublished"),
									value: ALL_VALUE,
								},
								{
									label: t("admin.textureLibraryTexturesPage.publishedOnly"),
									value: "published",
								},
								{
									label: t("admin.textureLibraryTexturesPage.notPublished"),
									value: "not_published",
								},
							]}
							onChange={(value) =>
								setFilters({
									published: value as PublishedFilter,
								})
							}
						/>
					</AdminFilterToolbar>
				}
			/>

			<TextureReviewDialog
				open={reviewDialog !== null}
				reviewDialog={reviewDialog}
				reviewNote={reviewNote}
				submitting={submittingTextureId !== null}
				onClose={closeReviewDialog}
				onNoteChange={setReviewNote}
				onSubmit={submitReviewAction}
			/>
			<TextureDeleteDialog
				open={deleteDialog !== null}
				deleteDialog={deleteDialog}
				submitting={submittingTextureId !== null}
				onClose={closeDeleteDialog}
				onConfirm={confirmDeleteTexture}
			/>
		</AdminPageShell>
	);
}

function TextureTableHeader() {
	const { t } = useTranslation();
	return (
		<AdminTableHeader>
			<AdminTableRow>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.texture")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.uploader")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.visibility")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.status")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.submittedAt")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.reviewedAt")}
				</AdminTableHead>
				<AdminTableHead className="text-right">
					{t("admin.textureLibraryTexturesPage.actions")}
				</AdminTableHead>
			</AdminTableRow>
		</AdminTableHeader>
	);
}

function TextureTableRow({
	loading,
	onApprove,
	onDelete,
	onOpen,
	onReject,
	onUnpublish,
	texture,
}: {
	loading: boolean;
	onApprove: () => void;
	onDelete: () => void;
	onOpen: () => void;
	onReject: () => void;
	onUnpublish: () => void;
	texture: LibraryTexture;
}) {
	const { t } = useTranslation();
	const canApprove =
		texture.visibility === "public" &&
		texture.library_status === "pending_review";
	const canUnpublish = texture.library_status === "published";
	const handleRowKeyDown = (event: KeyboardEvent<HTMLTableRowElement>) => {
		if (event.defaultPrevented) return;
		if (event.key === "Enter" || event.key === " ") {
			event.preventDefault();
			onOpen();
		}
	};

	return (
		<AdminTableRow
			tabIndex={0}
			className="cursor-pointer outline-none focus-visible:ring-3 focus-visible:ring-ring/30"
			onClick={onOpen}
			onKeyDown={handleRowKeyDown}
		>
			<AdminTableCell className="min-w-56">
				<div className="flex min-w-0 items-center gap-3">
					<TextureLibraryTextureAvatar
						texture={texture}
						testId={`admin-texture-preview-${texture.id}`}
						imageTestId={`admin-texture-preview-image-${texture.id}`}
						className="size-10"
					/>
					<div className="min-w-0">
						<div className="truncate font-medium">
							{textureDisplayName(texture)}
						</div>
						<div className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
							#{texture.id} · {t(`wardrobe.type.${texture.texture_type}`)} ·{" "}
							{texture.width}x{texture.height}
						</div>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="flex min-w-0 items-center gap-2.5">
					{texture.uploader ? (
						<UserAvatarImage
							avatar={texture.uploader.avatar}
							name={texture.uploader.name}
							alt=""
							size="sm"
							className="rounded-lg"
						/>
					) : null}
					<div className="min-w-0">
						<div className="truncate">
							{texture.uploader?.name ??
								t("admin.textureLibraryTexturesPage.unknownUploader")}
						</div>
						<div className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
							{texture.uploader
								? `@${texture.uploader.username} · #${texture.uploader.id}`
								: "-"}
						</div>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<Badge variant="outline" className="rounded-md">
					{t(`wardrobe.visibility.${texture.visibility}`)}
				</Badge>
			</AdminTableCell>
			<AdminTableCell>
				<div className="grid gap-1">
					<LibraryStatusBadge status={texture.library_status} />
					{texture.library_review_note ? (
						<div className="max-w-56 truncate text-xs text-muted-foreground">
							{texture.library_review_note}
						</div>
					) : null}
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<DateTimeText value={texture.library_submitted_at} />
			</AdminTableCell>
			<AdminTableCell>
				<DateTimeText value={texture.library_reviewed_at} />
			</AdminTableCell>
			<AdminTableCell className="text-right">
				<div className="flex justify-end gap-1.5">
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={!canApprove || loading}
						onClick={(event) => {
							event.stopPropagation();
							onApprove();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						{t("admin.textureLibraryTexturesPage.approveAction")}
					</Button>
					<Button
						type="button"
						size="sm"
						variant="destructive"
						disabled={!canApprove || loading}
						onClick={(event) => {
							event.stopPropagation();
							onReject();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						{t("admin.textureLibraryTexturesPage.rejectAction")}
					</Button>
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={!canUnpublish || loading}
						onClick={(event) => {
							event.stopPropagation();
							onUnpublish();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						{t("admin.textureLibraryTexturesPage.unpublishAction")}
					</Button>
					<Button
						type="button"
						size="sm"
						variant="destructive"
						disabled={loading}
						onClick={(event) => {
							event.stopPropagation();
							onDelete();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						<Icon name="Trash" className="size-4" />
						{t("common.delete")}
					</Button>
				</div>
			</AdminTableCell>
		</AdminTableRow>
	);
}

function TextureDeleteDialog({
	deleteDialog,
	onClose,
	onConfirm,
	open,
	submitting,
}: {
	deleteDialog: DeleteDialogState | null;
	onClose: (open: boolean) => void;
	onConfirm: () => void;
	open: boolean;
	submitting: boolean;
}) {
	const { t } = useTranslation();
	return (
		<ConfirmDialog
			open={open}
			loading={submitting}
			title={t("admin.textureLibraryTexturesPage.deleteTitle")}
			description={t("admin.textureLibraryTexturesPage.deleteDescription", {
				name: deleteDialog ? textureDisplayName(deleteDialog.texture) : "",
			})}
			cancelLabel={t("common.cancel")}
			confirmLabel={t("common.delete")}
			variant="destructive"
			onConfirm={onConfirm}
			onOpenChange={onClose}
		/>
	);
}

function TextureReviewDialog({
	onClose,
	onNoteChange,
	onSubmit,
	open,
	reviewDialog,
	reviewNote,
	submitting,
}: {
	onClose: () => void;
	onNoteChange: (value: string) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	open: boolean;
	reviewDialog: ReviewDialogState | null;
	reviewNote: string;
	submitting: boolean;
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={open} onOpenChange={onClose}>
			<DialogContent keepMounted className="sm:max-w-lg">
				<form className="grid gap-4" onSubmit={onSubmit}>
					<DialogHeader>
						<DialogTitle>
							{reviewDialog
								? t(
										`admin.textureLibraryTexturesPage.${reviewDialog.action}Title`,
									)
								: t("admin.textureLibraryTexturesPage.reviewTitle")}
						</DialogTitle>
						<DialogDescription>
							{reviewDialog
								? t(
										`admin.textureLibraryTexturesPage.${reviewDialog.action}Description`,
										{
											name: textureDisplayName(reviewDialog.texture),
										},
									)
								: t("admin.textureLibraryTexturesPage.reviewDescription")}
						</DialogDescription>
					</DialogHeader>
					<div className="grid gap-2">
						<label
							htmlFor="texture-library-review-note"
							className="text-sm font-medium"
						>
							{t("admin.textureLibraryTexturesPage.reviewNote")}
						</label>
						<Textarea
							id="texture-library-review-note"
							value={reviewNote}
							maxLength={500}
							placeholder={t(
								"admin.textureLibraryTexturesPage.reviewNotePlaceholder",
							)}
							onChange={(event) => onNoteChange(event.currentTarget.value)}
						/>
						<p className="text-xs text-muted-foreground">
							{reviewDialog?.action === "reject"
								? t("admin.textureLibraryTexturesPage.rejectNoteHint")
								: t("admin.textureLibraryTexturesPage.reviewNoteHint")}
						</p>
					</div>
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={submitting}
							onClick={onClose}
						>
							{t("common.cancel")}
						</Button>
						<Button
							type="submit"
							variant={
								reviewDialog?.action === "reject" ? "destructive" : "default"
							}
							disabled={submitting}
						>
							{submitting ? (
								<Icon name="Spinner" className="size-4 animate-spin" />
							) : null}
							{reviewDialog
								? t(
										`admin.textureLibraryTexturesPage.${reviewDialog.action}Action`,
									)
								: t("common.save")}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}

function AdminTextureDetailContent({
	loading,
	onApprove,
	onDelete,
	onRefresh,
	onReject,
	onUnpublish,
	submitting,
	texture,
	validTextureId,
}: {
	loading: boolean;
	onApprove: (texture: LibraryTexture) => void;
	onDelete: (texture: LibraryTexture) => void;
	onRefresh: () => void;
	onReject: (texture: LibraryTexture) => void;
	onUnpublish: (texture: LibraryTexture) => void;
	submitting: boolean;
	texture: LibraryTexture | null;
	validTextureId: number | null;
}) {
	const { t } = useTranslation();

	if (!validTextureId) {
		return (
			<AdminSurface>
				<div className="space-y-2">
					<h1 className="text-lg font-semibold">
						{t("admin.textureLibraryDetailPage.title")}
					</h1>
					<p className="text-sm text-muted-foreground">
						{t("admin.textureLibraryDetailPage.invalidId")}
					</p>
				</div>
			</AdminSurface>
		);
	}

	const canApprove =
		texture?.visibility === "public" &&
		texture.library_status === "pending_review";
	const canUnpublish = texture?.library_status === "published";

	return (
		<>
			<AdminPageHeader
				title={
					texture
						? textureDisplayName(texture)
						: t("admin.textureLibraryDetailPage.title")
				}
				description={t("admin.textureLibraryDetailPage.description")}
				actions={
					<>
						<Button
							type="button"
							variant="outline"
							size="sm"
							render={<Link to={adminPaths.textureLibrary} />}
						>
							<Icon name="ArrowLeft" className="size-4" />
							{t("admin.textureLibraryDetailPage.backToLibrary")}
						</Button>
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={loading}
							onClick={onRefresh}
						>
							<Icon
								name={loading ? "Spinner" : "RefreshCw"}
								className={cn("size-4", loading && "animate-spin")}
							/>
							{t("common.refresh")}
						</Button>
					</>
				}
			/>

			{loading ? (
				<div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.72fr)]">
					<Skeleton className="h-[34rem] rounded-lg" />
					<Skeleton className="h-[34rem] rounded-lg" />
				</div>
			) : !texture ? (
				<AdminSurface>
					<div className="space-y-2">
						<h2 className="text-lg font-semibold">
							{t("admin.textureLibraryDetailPage.notFoundTitle")}
						</h2>
						<p className="text-sm text-muted-foreground">
							{t("admin.textureLibraryDetailPage.notFoundDescription")}
						</p>
					</div>
				</AdminSurface>
			) : (
				<div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.72fr)]">
					<AdminSurface padded={false} className="min-w-0 overflow-hidden">
						<TextureDetailHeader texture={texture} />
						<div className="grid gap-5 p-4 sm:p-5">
							<TextureRecordSection texture={texture} />
							<TextureReviewSection texture={texture} />
						</div>
					</AdminSurface>

					<aside className="grid min-w-0 max-w-full gap-3 xl:sticky xl:top-20 xl:self-start">
						<MinecraftPreviewPanel
							label={t("admin.textureLibraryDetailPage.preview")}
							playerName={textureDisplayName(texture)}
							skinUrl={texture.texture_type === "skin" ? texture.url : null}
							capeUrl={texture.texture_type === "cape" ? texture.url : null}
							model={texture.texture_model}
							emptyTitle={t("profiles.previewEmptyTitle")}
							emptyDescription={t("profiles.previewEmptyDescription")}
							failedTitle={t("profiles.previewFailedTitle")}
							failedDescription={t("profiles.previewFailedDescription")}
							noSkinLabel={t("profiles.noSkinTexture")}
							idleLabel={t("profiles.motionIdle")}
							walkLabel={t("profiles.motionWalk")}
							frameClassName="h-[34rem]"
							skeletonClassName="h-[38rem]"
						/>
						<TextureReviewActions
							canApprove={canApprove}
							canUnpublish={canUnpublish}
							submitting={submitting}
							texture={texture}
							onApprove={onApprove}
							onDelete={onDelete}
							onReject={onReject}
							onUnpublish={onUnpublish}
						/>
					</aside>
				</div>
			)}
		</>
	);
}

function TextureDetailHeader({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	return (
		<div className="border-b border-border/70 bg-muted/16 px-4 py-4 dark:border-white/10 dark:bg-white/4 sm:px-5">
			<div className="flex min-w-0 items-start gap-3">
				<TextureLibraryTextureAvatar
					texture={texture}
					className="size-12 rounded-lg"
					testId={`admin-texture-detail-avatar-${texture.id}`}
					imageTestId={`admin-texture-detail-avatar-image-${texture.id}`}
				/>
				<div className="min-w-0">
					<div className="flex min-w-0 flex-wrap items-center gap-2">
						<h2 className="break-words text-lg font-semibold text-foreground">
							{textureDisplayName(texture)}
						</h2>
						<LibraryStatusBadge status={texture.library_status} />
					</div>
					<p className="mt-1 break-all font-mono text-xs text-muted-foreground">
						{texture.hash}
					</p>
					<div className="mt-2 flex flex-wrap gap-1.5">
						<Badge variant="outline" className="rounded-md">
							{t(`wardrobe.type.${texture.texture_type}`)}
						</Badge>
						{texture.texture_type === "skin" ? (
							<Badge variant="outline" className="rounded-md">
								{texture.texture_model}
							</Badge>
						) : null}
						<Badge variant="outline" className="rounded-md">
							{t(`wardrobe.visibility.${texture.visibility}`)}
						</Badge>
					</div>
				</div>
			</div>
		</div>
	);
}

function TextureRecordSection({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	return (
		<section className="grid gap-3">
			<div>
				<h3 className="text-base font-semibold text-foreground">
					{t("admin.textureLibraryDetailPage.recordTitle")}
				</h3>
				<p className="mt-1 text-sm leading-6 text-muted-foreground">
					{t("admin.textureLibraryDetailPage.recordDescription")}
				</p>
			</div>
			<div className="grid min-w-0 gap-3 md:grid-cols-2 xl:grid-cols-3">
				<TextureUploaderTile texture={texture} />
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.textureId")}
					value={`#${texture.id}`}
					mono
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.textureName")}
					value={texture.name}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.hash")}
					value={texture.hash}
					mono
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.dimensions")}
					value={`${texture.width}x${texture.height}`}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.fileSize")}
					value={formatBytes(texture.file_size)}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.mimeType")}
					value={texture.mime_type}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.createdAt")}
					value={<DateTimeText value={texture.created_at} />}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.updatedAt")}
					value={<DateTimeText value={texture.updated_at} />}
				/>
			</div>
			<div className="grid gap-1.5">
				<div className="text-xs font-medium text-muted-foreground">
					{t("admin.textureLibraryDetailPage.tags")}
				</div>
				{texture.tags.length > 0 ? (
					<TextureTagChips tags={texture.tags} />
				) : (
					<div className="text-sm text-muted-foreground">
						{t("admin.textureLibraryDetailPage.noTags")}
					</div>
				)}
			</div>
		</section>
	);
}

function TextureUploaderTile({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	const uploader = texture.uploader;

	return (
		<div className="min-w-0 rounded-lg border border-border/70 bg-background/60 p-3">
			<p className="text-xs uppercase tracking-wide text-muted-foreground">
				{t("admin.textureLibraryDetailPage.uploader")}
			</p>
			{uploader ? (
				<div className="mt-2 flex min-w-0 items-center gap-3 rounded-md">
					<UserAvatarImage
						avatar={uploader.avatar}
						name={uploader.name}
						alt=""
						size="sm"
						className="rounded-lg"
					/>
					<div className="min-w-0">
						<div className="truncate text-sm font-medium text-foreground">
							{uploader.name}
						</div>
						<div className="mt-1 truncate text-xs text-muted-foreground">
							@{uploader.username} · #{uploader.id}
						</div>
					</div>
				</div>
			) : (
				<p className="mt-2 text-sm text-muted-foreground">
					{t("admin.textureLibraryTexturesPage.unknownUploader")}
				</p>
			)}
		</div>
	);
}

function TextureReviewSection({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	return (
		<section className="grid gap-3">
			<div>
				<h3 className="text-base font-semibold text-foreground">
					{t("admin.textureLibraryDetailPage.reviewTitle")}
				</h3>
				<p className="mt-1 text-sm leading-6 text-muted-foreground">
					{t("admin.textureLibraryDetailPage.reviewDescription")}
				</p>
			</div>
			<div className="grid min-w-0 gap-3 md:grid-cols-3">
				<TextureInfoTile
					label={t("admin.textureLibraryTexturesPage.status")}
					value={<LibraryStatusBadge status={texture.library_status} />}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryTexturesPage.submittedAt")}
					value={<DateTimeText value={texture.library_submitted_at} />}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryTexturesPage.reviewedAt")}
					value={<DateTimeText value={texture.library_reviewed_at} />}
				/>
			</div>
			<div className="rounded-lg border border-border/70 bg-background/60 p-3">
				<div className="text-xs font-medium text-muted-foreground">
					{t("admin.textureLibraryTexturesPage.reviewNote")}
				</div>
				<div className="mt-2 text-sm leading-6">
					{texture.library_review_note?.trim() || "-"}
				</div>
			</div>
		</section>
	);
}

function TextureReviewActions({
	canApprove,
	canUnpublish,
	onApprove,
	onDelete,
	onReject,
	onUnpublish,
	submitting,
	texture,
}: {
	canApprove: boolean;
	canUnpublish: boolean;
	onApprove: (texture: LibraryTexture) => void;
	onDelete: (texture: LibraryTexture) => void;
	onReject: (texture: LibraryTexture) => void;
	onUnpublish: (texture: LibraryTexture) => void;
	submitting: boolean;
	texture: LibraryTexture;
}) {
	const { t } = useTranslation();
	return (
		<div className="grid gap-3 rounded-lg border border-border/70 bg-card/95 p-4 shadow-xs dark:border-white/10">
			<div className="flex min-w-0 flex-wrap items-center gap-2">
				<Badge variant="outline" className="rounded-md">
					{t(`wardrobe.visibility.${texture.visibility}`)}
				</Badge>
				<LibraryStatusBadge status={texture.library_status} />
			</div>
			<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-end">
				<Button
					type="button"
					variant="outline"
					disabled={!canApprove || submitting}
					onClick={() => onApprove(texture)}
				>
					{t("admin.textureLibraryTexturesPage.approveAction")}
				</Button>
				<Button
					type="button"
					variant="destructive"
					disabled={!canApprove || submitting}
					onClick={() => onReject(texture)}
				>
					{t("admin.textureLibraryTexturesPage.rejectAction")}
				</Button>
				<Button
					type="button"
					variant="outline"
					disabled={!canUnpublish || submitting}
					onClick={() => onUnpublish(texture)}
				>
					{t("admin.textureLibraryTexturesPage.unpublishAction")}
				</Button>
				<Button
					type="button"
					variant="destructive"
					disabled={submitting}
					onClick={() => onDelete(texture)}
				>
					<Icon name="Trash" className="size-4" />
					{t("common.delete")}
				</Button>
			</div>
		</div>
	);
}

function TextureInfoTile({
	label,
	mono,
	value,
}: {
	label: string;
	mono?: boolean;
	value: ReactNode;
}) {
	return (
		<div className="min-w-0 rounded-lg border border-border/70 bg-background/60 p-3">
			<p className="text-xs uppercase tracking-wide text-muted-foreground">
				{label}
			</p>
			<div
				className={cn(
					"mt-2 min-w-0 break-words text-sm font-medium text-foreground",
					mono && ADMIN_TABLE_MONO_TEXT_CLASS,
				)}
			>
				{value}
			</div>
		</div>
	);
}

function LibraryStatusBadge({
	status,
}: {
	status: MinecraftTextureLibraryStatus;
}) {
	const { t } = useTranslation();
	return (
		<Badge
			variant={status === "published" ? "default" : "outline"}
			className={cn(
				"w-fit rounded-md",
				status === "pending_review" &&
					"border-amber-500/35 bg-amber-500/10 text-amber-700 dark:text-amber-300",
				status === "rejected" &&
					"border-destructive/35 bg-destructive/10 text-destructive",
			)}
		>
			{t(`admin.textureLibraryTexturesPage.libraryStatus.${status}`)}
		</Badge>
	);
}

function TextureSelect({
	ariaLabel,
	onChange,
	options,
	value,
}: {
	ariaLabel: string;
	onChange: (value: string) => void;
	options: ReadonlyArray<{ label: string; value: string }>;
	value: string;
}) {
	return (
		<Select
			items={options}
			value={value}
			onValueChange={(nextValue) => {
				if (nextValue) onChange(nextValue);
			}}
		>
			<SelectTrigger width="compact" aria-label={ariaLabel}>
				<SelectValue />
			</SelectTrigger>
			<SelectContent align="start">
				{options.map((option) => (
					<SelectItem key={option.value} value={option.value}>
						{option.label}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}

function textureDisplayName(texture: LibraryTexture) {
	return texture.display_name?.trim() || texture.name;
}

function libraryStatusOptions(t: ReturnType<typeof useTranslation>["t"]) {
	return (
		[
			"private",
			"pending_review",
			"published",
			"rejected",
		] as const satisfies readonly MinecraftTextureLibraryStatus[]
	).map((status) => ({
		label: t(`admin.textureLibraryTexturesPage.libraryStatus.${status}`),
		value: status,
	}));
}
