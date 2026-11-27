import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { TextureLibrarySectionNav } from "@/components/admin/TextureLibrarySectionNav";
import { AdminFilterToolbar } from "@/components/common/AdminFilterToolbar";
import { AdminTableList } from "@/components/common/AdminTableList";
import { RefreshButton } from "@/components/common/RefreshButton";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { usePageTitle } from "@/hooks/usePageTitle";
import { parsePageSizeOption } from "@/lib/pagination";
import { adminTextureLibraryPath } from "@/routes/routePaths";
import type {
	MinecraftTextureLibraryStatus,
	MinecraftTextureType,
	MinecraftTextureVisibility,
} from "@/types/api";
import { AdminTextureDetailContent } from "./AdminTextureDetailContent";
import {
	ALL_VALUE,
	type FilterValue,
	libraryStatusOptions,
	type PublishedFilter,
	TEXTURE_PAGE_SIZE_OPTIONS,
	type TexturePageMode,
} from "./libraryTextureView";
import { TextureDeleteDialog } from "./TextureDeleteDialog";
import { TextureReviewDialog } from "./TextureReviewDialog";
import { TextureTableHeader, TextureTableRow } from "./TextureTexturesTable";
import { useTextureLibraryTexturesController } from "./useTextureLibraryTexturesController";

export default function AdminTextureLibraryTexturesPage({
	mode,
}: {
	mode: TexturePageMode;
}) {
	const { t } = useTranslation();
	const controller = useTextureLibraryTexturesController(mode);
	const navigate = useNavigate();
	const {
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
		filters,
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
	} = controller;
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
						<RefreshButton
							loading={list.loading}
							onClick={() => void list.reload()}
						/>
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

export function TextureSelect({
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
