import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { DateTimeText } from "@/components/common/DateTimeText";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { TextureTagFilterPopover } from "@/components/yggdrasil/TextureTagFilterPopover";
import { cn } from "@/lib/utils";
import type {
	MinecraftTextureTagInfo,
	MinecraftTextureType,
	MinecraftWardrobeTextureMetadata,
	TextureTagSearchMethod,
} from "@/types/api";
import { WardrobeTextureCard } from "./WardrobeTextureCard";

const WARDROBE_PAGE_SIZE_OPTIONS = [10, 20] as const;

export function WardrobeListSection({
	activeTab,
	draftTagIds,
	hasMoreTags,
	loading,
	onClearTagFilter,
	onChangeTagSearchMethod,
	onLoadMoreTags,
	onOpenTagFilter,
	onPageSizeChange,
	onQueryChange,
	onReload,
	onSearchQueryChange,
	onSelectTab,
	onSelectedTagIdsChange,
	onTextureSelect,
	onUpload,
	openUploadDialog,
	pageSize,
	query,
	searchBusy,
	selectedTags,
	tagFilterOpen,
	tagLoading,
	tagSearchMethod,
	tags,
	textureList,
	previewTextureId,
	uploading,
}: {
	activeTab: MinecraftTextureType;
	draftTagIds: number[];
	hasMoreTags: boolean;
	loading: boolean;
	onClearTagFilter: () => void;
	onChangeTagSearchMethod: (method: TextureTagSearchMethod) => void;
	onLoadMoreTags: () => void;
	onOpenTagFilter: (open: boolean) => void;
	onPageSizeChange: (pageSize: number) => void;
	onQueryChange: (query: string) => void;
	onReload: () => void;
	onSearchQueryChange: (query: string) => void;
	onSelectTab: (tab: MinecraftTextureType) => void;
	onSelectedTagIdsChange: (tagIds: number[]) => void;
	onTextureSelect: (texture: MinecraftWardrobeTextureMetadata) => void;
	onUpload: () => void;
	openUploadDialog: () => void;
	pageSize: number;
	query: string;
	searchBusy: boolean;
	selectedTags: MinecraftTextureTagInfo[];
	tagFilterOpen: boolean;
	tagLoading: boolean;
	tagSearchMethod: TextureTagSearchMethod;
	tags: MinecraftTextureTagInfo[];
	textureList: {
		currentPage: number;
		goNext: () => void;
		goPrevious: () => void;
		items: MinecraftWardrobeTextureMetadata[];
		nextPageDisabled: boolean;
		prevPageDisabled: boolean;
		total: number;
	};
	previewTextureId: number | null;
	uploading: boolean;
}) {
	const { t } = useTranslation();
	const textures = textureList.items;
	const textureTotal = textureList.total;

	return (
		<section className="min-w-0 self-start overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs">
			<div className="flex flex-col gap-3 border-b border-border/70 bg-muted/35 p-3 lg:flex-row lg:items-center lg:justify-between">
				<div className="flex min-w-0 flex-wrap items-center gap-1">
					<WardrobeTabButton
						active={activeTab === "skin"}
						onClick={() => onSelectTab("skin")}
					>
						{t("wardrobe.type.skin")}
					</WardrobeTabButton>
					<WardrobeTabButton
						active={activeTab === "cape"}
						onClick={() => onSelectTab("cape")}
					>
						{t("wardrobe.type.cape")}
					</WardrobeTabButton>
				</div>
				<div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-center">
					<div className="relative sm:w-72">
						<Icon
							name={searchBusy ? "Spinner" : "MagnifyingGlass"}
							aria-hidden="true"
							data-testid={
								searchBusy ? "wardrobe-search-spinner" : "wardrobe-search-icon"
							}
							className={cn(
								"absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground",
								searchBusy && "animate-spin text-emerald-500",
							)}
						/>
						<Input
							value={query}
							placeholder={t("wardrobe.searchPlaceholder")}
							className="pl-8"
							onChange={(event) => onQueryChange(event.currentTarget.value)}
						/>
					</div>
					<TextureTagFilterPopover
						open={tagFilterOpen}
						selectedIds={draftTagIds}
						selectedTags={selectedTags}
						searchMethod={tagSearchMethod}
						hasMore={hasMoreTags}
						loading={tagLoading}
						tags={tags}
						testId="wardrobe-tag-filter-popover"
						onClear={onClearTagFilter}
						onLoadMore={onLoadMoreTags}
						onOpenChange={onOpenTagFilter}
						onSearchQueryChange={onSearchQueryChange}
						onSearchMethodChange={onChangeTagSearchMethod}
						onSelectedIdsChange={onSelectedTagIdsChange}
					/>
					<Button
						type="button"
						variant="outline"
						size="sm"
						onClick={onReload}
						disabled={loading || uploading}
					>
						{loading ? t("common.loading") : t("common.refresh")}
					</Button>
					<Button
						type="button"
						size="sm"
						onClick={onUpload}
						disabled={uploading}
					>
						{t("wardrobe.uploadTab")}
					</Button>
				</div>
			</div>

			<div className="max-h-[min(42rem,calc(100dvh-18rem))] min-h-[24rem] overflow-y-auto p-3">
				{loading ? (
					<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
						{Array.from(
							{ length: 6 },
							(_, index) => `wardrobe-skeleton-${index}`,
						).map((key) => (
							<Skeleton key={key} className="h-52 rounded-lg" />
						))}
					</div>
				) : textures.length === 0 ? (
					<WardrobeEmptyState
						title={t("wardrobe.emptyTitle")}
						description={
							textures.length === 0
								? t("wardrobe.emptyDescription")
								: t("wardrobe.noSearchResults")
						}
						action={
							<Button
								type="button"
								variant="outline"
								onClick={openUploadDialog}
							>
								{t("wardrobe.uploadTab")}
							</Button>
						}
					/>
				) : (
					<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
						{textures.map((texture) => (
							<WardrobeTextureCard
								key={texture.id}
								active={previewTextureId === texture.id}
								texture={texture}
								date={<DateTimeText value={texture.created_at} />}
								onSelect={() => onTextureSelect(texture)}
							/>
						))}
					</div>
				)}
			</div>
			<WardrobePagination
				currentPage={textureList.currentPage}
				nextDisabled={textureList.nextPageDisabled}
				prevDisabled={textureList.prevPageDisabled}
				pageSize={pageSize}
				total={textureTotal}
				totalPages={Math.max(1, Math.ceil(textureTotal / pageSize))}
				onNext={textureList.goNext}
				onPageSizeChange={onPageSizeChange}
				onPrevious={textureList.goPrevious}
			/>
		</section>
	);
}

function WardrobeTabButton({
	active,
	children,
	onClick,
}: {
	active: boolean;
	children: ReactNode;
	onClick: () => void;
}) {
	return (
		<Button
			type="button"
			variant={active ? "default" : "ghost"}
			className={cn(
				"rounded-md px-4",
				active ? "bg-primary text-primary-foreground" : "text-muted-foreground",
			)}
			onClick={onClick}
		>
			{children}
		</Button>
	);
}

function WardrobePagination({
	currentPage,
	nextDisabled,
	onNext,
	onPageSizeChange,
	onPrevious,
	pageSize,
	prevDisabled,
	total,
	totalPages,
}: {
	currentPage: number;
	nextDisabled: boolean;
	onNext: () => void;
	onPageSizeChange: (pageSize: number) => void;
	onPrevious: () => void;
	pageSize: number;
	prevDisabled: boolean;
	total: number;
	totalPages: number;
}) {
	const { t } = useTranslation();
	if (total <= 0) return null;

	return (
		<div className="flex flex-col gap-3 border-t border-border/70 bg-muted/20 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
			<div className="text-sm text-muted-foreground">
				{t("admin.pagination.entriesPage", {
					current: currentPage,
					pages: totalPages,
					total,
				})}
			</div>
			<div className="flex flex-wrap items-center gap-2">
				<select
					aria-label={t("admin.pagination.pageSize")}
					className="h-8 rounded-lg border border-input/80 bg-card/70 px-2.5 text-sm shadow-xs outline-none transition-[background-color,border-color,box-shadow] focus-visible:border-ring focus-visible:bg-background focus-visible:ring-3 focus-visible:ring-ring/30 dark:bg-input/25 dark:shadow-none"
					value={String(pageSize)}
					onChange={(event) =>
						onPageSizeChange(Number(event.currentTarget.value))
					}
				>
					{WARDROBE_PAGE_SIZE_OPTIONS.map((option) => (
						<option key={option} value={option}>
							{t("admin.pagination.pageSizeOption", { count: option })}
						</option>
					))}
				</select>
				<Button
					type="button"
					variant="outline"
					size="sm"
					disabled={prevDisabled}
					onClick={onPrevious}
				>
					{t("admin.pagination.previous")}
				</Button>
				<span className="rounded-md bg-primary px-3 py-1.5 text-sm font-semibold text-primary-foreground">
					{currentPage}
				</span>
				<Button
					type="button"
					variant="outline"
					size="sm"
					disabled={nextDisabled}
					onClick={onNext}
				>
					{t("admin.pagination.next")}
				</Button>
			</div>
		</div>
	);
}

function WardrobeEmptyState({
	action,
	description,
	title,
}: {
	action?: ReactNode;
	description: string;
	title: string;
}) {
	return (
		<div className="grid min-h-72 place-items-center px-4 py-12 text-center">
			<div className="max-w-sm">
				<div className="text-sm font-semibold">{title}</div>
				<p className="mt-2 text-sm leading-6 text-muted-foreground">
					{description}
				</p>
				{action ? (
					<div className="mt-4 flex justify-center">{action}</div>
				) : null}
			</div>
		</div>
	);
}
