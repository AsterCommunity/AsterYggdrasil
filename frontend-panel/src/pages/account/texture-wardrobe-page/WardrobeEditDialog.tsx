import { type FormEvent, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { TextureTagPickerList } from "@/components/yggdrasil/TextureTagList";
import type {
	MinecraftTextureTagInfo,
	MinecraftWardrobeTextureMetadata,
} from "@/types/api";

export function WardrobeEditDialog({
	clearStateAfterClose,
	editDialogOpen,
	editTagIds,
	editTagQuery,
	editTexture,
	hasMoreTags,
	onLoadMoreTags,
	onOpenChange,
	onQueryChange,
	onSearchTags,
	onSubmit,
	onToggleTag,
	submitting,
	tagLoading,
	tags,
}: {
	clearStateAfterClose: () => void;
	editDialogOpen: boolean;
	editTagIds: number[];
	editTagQuery: string;
	editTexture: MinecraftWardrobeTextureMetadata | null;
	hasMoreTags: boolean;
	onLoadMoreTags: () => void;
	onOpenChange: (open: boolean) => void;
	onQueryChange: (query: string) => void;
	onSearchTags: (query: string) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onToggleTag: (tagId: number) => void;
	submitting: boolean;
	tagLoading: boolean;
	tags: MinecraftTextureTagInfo[];
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={editDialogOpen} onOpenChange={onOpenChange}>
			<DialogContent keepMounted onAnimationEnd={clearStateAfterClose}>
				<form
					key={editTexture?.id ?? "empty-edit-texture"}
					className="grid gap-4"
					onSubmit={onSubmit}
				>
					<DialogHeader>
						<DialogTitle>{t("wardrobe.editDialogTitle")}</DialogTitle>
						<DialogDescription>
							{editTexture
								? t("wardrobe.editDialogDescription", {
										name: editTexture.name,
									})
								: t("wardrobe.editDialogFallback")}
						</DialogDescription>
					</DialogHeader>
					<div className="grid gap-3">
						<div className="grid gap-1.5">
							<label
								htmlFor="wardrobe-edit-texture-name"
								className="text-sm font-medium"
							>
								{t("wardrobe.textureName")}
							</label>
							<Input
								id="wardrobe-edit-texture-name"
								name="name"
								defaultValue={editTexture?.display_name ?? ""}
								maxLength={96}
								placeholder={
									editTexture
										? t("wardrobe.textureNamePlaceholder", {
												name: editTexture.name,
											})
										: t("wardrobe.textureNamePlaceholderFallback")
								}
							/>
						</div>
						<div className="grid gap-1.5">
							<div className="text-sm font-medium">
								{t("wardrobe.visibility.label")}
							</div>
							<div className="grid grid-cols-2 gap-1 rounded-lg border border-border/70 bg-muted/30 p-1">
								{(["private", "public"] as const).map((option) => (
									<label
										key={option}
										className="relative grid h-8 cursor-pointer place-items-center overflow-hidden rounded-md px-3 text-sm font-medium transition-colors has-checked:bg-primary has-checked:text-primary-foreground has-checked:shadow-xs has-focus-visible:ring-3 has-focus-visible:ring-ring/35"
									>
										<input
											key={`${editTexture?.id ?? "none"}-${option}`}
											type="radio"
											name="visibility"
											value={option}
											defaultChecked={editTexture?.visibility === option}
											className="sr-only"
										/>
										{t(`wardrobe.visibility.${option}`)}
									</label>
								))}
							</div>
						</div>
						{editTexture?.texture_type === "skin" ? (
							<div className="grid gap-1.5">
								<div className="text-sm font-medium">{t("profiles.model")}</div>
								<div className="grid grid-cols-2 gap-1 rounded-lg border border-border/70 bg-muted/30 p-1">
									{(["default", "slim"] as const).map((option) => (
										<label
											key={option}
											className="relative grid h-8 cursor-pointer place-items-center overflow-hidden rounded-md px-3 text-sm font-medium transition-colors has-checked:bg-primary has-checked:text-primary-foreground has-checked:shadow-xs has-focus-visible:ring-3 has-focus-visible:ring-ring/35"
										>
											<input
												key={`${editTexture.id}-${option}`}
												type="radio"
												name="texture_model"
												value={option}
												defaultChecked={editTexture.texture_model === option}
												className="sr-only"
											/>
											{t(`profiles.${option}Model`)}
										</label>
									))}
								</div>
							</div>
						) : null}
						<div className="grid gap-1.5">
							<WardrobeEditTagSelector
								disabled={submitting}
								hasMore={hasMoreTags}
								loading={tagLoading}
								query={editTagQuery}
								selectedIds={editTagIds}
								tags={tags}
								onLoadMore={onLoadMoreTags}
								onQueryChange={onQueryChange}
								onSearchQueryChange={onSearchTags}
								onToggle={onToggleTag}
							/>
						</div>
					</div>
					<DialogFooter>
						<DialogClose
							render={
								<Button type="button" variant="outline" disabled={submitting} />
							}
						>
							{t("common.cancel")}
						</DialogClose>
						<Button type="submit" disabled={!editTexture || submitting}>
							{submitting ? t("wardrobe.saving") : t("common.save")}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}

function WardrobeEditTagSelector({
	disabled,
	hasMore,
	loading,
	onLoadMore,
	onQueryChange,
	onSearchQueryChange,
	onToggle,
	query,
	selectedIds,
	tags,
}: {
	disabled: boolean;
	hasMore: boolean;
	loading: boolean;
	onLoadMore: () => void;
	onQueryChange: (query: string) => void;
	onSearchQueryChange: (query: string) => void;
	onToggle: (tagId: number) => void;
	query: string;
	selectedIds: number[];
	tags: MinecraftTextureTagInfo[];
}) {
	const { t } = useTranslation();
	const hasQuery = query.trim().length > 0;

	// Debounces the callback itself: the parent-provided handler identity can
	// change across renders, so calling it synchronously here would re-enter
	// the parent update loop on every render.
	useEffect(() => {
		const timer = window.setTimeout(() => {
			onSearchQueryChange(query.trim());
		}, 180);
		return () => window.clearTimeout(timer);
	}, [onSearchQueryChange, query]);

	return (
		<div className="grid gap-1.5">
			<div className="flex items-center justify-between gap-2">
				<label
					htmlFor="wardrobe-edit-texture-tag-search"
					className="text-sm font-medium"
				>
					{t("wardrobe.tags")}
				</label>
				{selectedIds.length > 0 ? (
					<span className="text-xs text-muted-foreground">
						{t("wardrobe.selectedTags", { count: selectedIds.length })}
					</span>
				) : null}
			</div>
			{tags.length > 0 || loading || hasQuery ? (
				<>
					<div className="relative">
						<Icon
							name="MagnifyingGlass"
							className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
						/>
						<Input
							id="wardrobe-edit-texture-tag-search"
							type="search"
							value={query}
							placeholder={t("wardrobe.tagSearchPlaceholder")}
							className="pl-8"
							disabled={disabled}
							onChange={(event) => onQueryChange(event.currentTarget.value)}
						/>
					</div>
					<TextureTagPickerList
						className="max-h-52"
						disabled={disabled}
						emptyLabel={
							hasQuery
								? t("wardrobe.noTagSearchResults")
								: t("wardrobe.noAvailableTags")
						}
						hasMore={hasMore}
						loading={loading}
						loadingLabel={t("common.loading")}
						selectedIds={selectedIds}
						tags={tags}
						onLoadMore={onLoadMore}
						onToggle={onToggle}
					/>
				</>
			) : (
				<div className="rounded-lg border border-dashed border-border/70 px-3 py-2 text-sm text-muted-foreground">
					{t("wardrobe.noAvailableTags")}
				</div>
			)}
		</div>
	);
}
