import { Popover as PopoverPrimitive } from "@base-ui/react/popover";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import {
	TextureTagChips,
	TextureTagPickerList,
} from "@/components/yggdrasil/TextureTagList";
import { cn } from "@/lib/utils";
import type {
	MinecraftTextureTagInfo,
	TextureTagSearchMethod,
} from "@/types/api";

const TAG_FILTER_SEARCH_DEBOUNCE_MS = 180;

export function TextureTagFilterPopover({
	open,
	searchMethod,
	selectedIds,
	selectedTags,
	hasMore,
	loading,
	tags,
	testId = "texture-tag-filter-popover",
	onClear,
	onLoadMore,
	onOpenChange,
	onSearchQueryChange,
	onSearchMethodChange,
	onSelectedIdsChange,
}: {
	open: boolean;
	searchMethod: TextureTagSearchMethod;
	selectedIds: number[];
	selectedTags: MinecraftTextureTagInfo[];
	hasMore: boolean;
	loading: boolean;
	tags: MinecraftTextureTagInfo[];
	testId?: string;
	onClear: () => void;
	onLoadMore: () => void;
	onOpenChange: (open: boolean) => void;
	onSearchQueryChange: (query: string) => void;
	onSearchMethodChange: (method: TextureTagSearchMethod) => void;
	onSelectedIdsChange: (selectedIds: number[]) => void;
}) {
	const { t } = useTranslation();
	const [query, setQuery] = useState("");
	const [debouncedQuery, setDebouncedQuery] = useState("");
	const triggerLabel =
		selectedTags.length > 0
			? t("wardrobe.tagFilterSelected", { count: selectedTags.length })
			: t("wardrobe.tagFilterButton");

	useEffect(() => {
		const timer = window.setTimeout(() => {
			setDebouncedQuery(query.trim());
		}, TAG_FILTER_SEARCH_DEBOUNCE_MS);
		return () => window.clearTimeout(timer);
	}, [query]);

	useEffect(() => {
		if (!open) return;
		onSearchQueryChange(debouncedQuery);
	}, [debouncedQuery, onSearchQueryChange, open]);

	function toggleTag(tagId: number) {
		onSelectedIdsChange(
			selectedIds.includes(tagId)
				? selectedIds.filter((id) => id !== tagId)
				: [...selectedIds, tagId],
		);
	}

	return (
		<PopoverPrimitive.Root
			open={open}
			// trap-focus：Tab 焦点留在浮层内，但不锁页面滚动、不拦截外部点击
			modal="trap-focus"
			onOpenChange={(nextOpen) => onOpenChange(nextOpen)}
		>
			<PopoverPrimitive.Trigger
				aria-haspopup="dialog"
				render={
					<Button
						type="button"
						variant={selectedTags.length > 0 || open ? "secondary" : "outline"}
						size="sm"
						className="w-full justify-between gap-2 sm:w-auto"
					>
						<span className="flex min-w-0 items-center gap-2">
							<Icon name="ListChecks" className="size-4 shrink-0" />
							<span className="truncate">{triggerLabel}</span>
						</span>
						<Icon name={open ? "CaretUp" : "CaretDown"} className="size-4" />
					</Button>
				}
			/>
			<PopoverPrimitive.Portal>
				<PopoverPrimitive.Positioner
					side="bottom"
					align="end"
					sideOffset={8}
					collisionPadding={16}
					className="isolate z-50 outline-none"
				>
					<PopoverPrimitive.Popup
						data-testid={testId}
						data-theme-surface="overlay"
						className="z-50 grid max-h-(--available-height) w-[min(22rem,calc(100vw-2rem))] origin-(--transform-origin) gap-2 overflow-hidden rounded-lg border border-border/70 bg-popover p-3 text-popover-foreground shadow-xl duration-100 outline-none data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95 motion-reduce:duration-0"
					>
						<div className="flex items-center justify-between gap-2">
							<div className="text-sm font-medium">{t("wardrobe.tags")}</div>
							{selectedTags.length > 0 ? (
								<Button
									type="button"
									variant="ghost"
									size="sm"
									className="h-7 px-2 text-xs"
									onClick={onClear}
								>
									{t("wardrobe.tagFilterClear")}
								</Button>
							) : null}
						</div>
						{selectedTags.length > 0 ? (
							<div className="max-h-[10.5rem] overflow-y-auto pr-1">
								<TextureTagChips tags={selectedTags} />
							</div>
						) : (
							<div className="text-xs text-muted-foreground">
								{t("wardrobe.tagFilterAll")}
							</div>
						)}
						<div className="grid grid-cols-2 gap-1 rounded-lg border border-border/70 bg-muted/30 p-1">
							{(["all", "any"] as const).map((method) => (
								<button
									key={method}
									type="button"
									className={cn(
										"h-8 rounded-md px-2 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35",
										searchMethod === method
											? "bg-background text-foreground shadow-xs"
											: "text-muted-foreground hover:bg-background/60",
									)}
									onClick={() => onSearchMethodChange(method)}
								>
									{t(`wardrobe.tagSearchMethod.${method}`)}
								</button>
							))}
						</div>
						<div className="relative">
							<Icon
								name="MagnifyingGlass"
								className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
							/>
							<Input
								type="search"
								value={query}
								placeholder={t("wardrobe.tagSearchPlaceholder")}
								className="h-9 pl-8"
								onChange={(event) => setQuery(event.currentTarget.value)}
							/>
						</div>
						<TextureTagPickerList
							className="min-h-0 max-h-[18rem]"
							emptyLabel={
								debouncedQuery
									? t("wardrobe.noTagSearchResults")
									: t("wardrobe.noAvailableTags")
							}
							hasMore={hasMore}
							loading={loading}
							loadingLabel={t("common.loading")}
							selectedIds={selectedIds}
							tags={tags}
							onLoadMore={onLoadMore}
							onToggle={toggleTag}
						/>
						<div className="text-xs leading-5 text-muted-foreground">
							{t(`wardrobe.tagFilterHint.${searchMethod}`)}
						</div>
						<PopoverPrimitive.Close className="sr-only">
							{t("common.close")}
						</PopoverPrimitive.Close>
					</PopoverPrimitive.Popup>
				</PopoverPrimitive.Positioner>
			</PopoverPrimitive.Portal>
		</PopoverPrimitive.Root>
	);
}
