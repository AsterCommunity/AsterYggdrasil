import { type RefObject, type UIEvent, useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type {
	MinecraftTextureTagInfo,
	TextureTagSearchMethod,
} from "@/types/api";

const TAG_FILTER_SEARCH_DEBOUNCE_MS = 180;
const TAG_FILTER_POPOVER_GAP = 8;
const TAG_FILTER_POPOVER_MARGIN = 16;
const TAG_FILTER_POPOVER_WIDTH = 352;

export function TextureTagFilterPopover({
	open,
	popoverRef,
	searchMethod,
	selectedIds,
	selectedTags,
	hasMore,
	loading,
	tags,
	testId = "texture-tag-filter-popover",
	triggerRef,
	onClear,
	onLoadMore,
	onOpenChange,
	onSearchQueryChange,
	onSearchMethodChange,
	onSelectedIdsChange,
}: {
	open: boolean;
	popoverRef: RefObject<HTMLDivElement | null>;
	searchMethod: TextureTagSearchMethod;
	selectedIds: number[];
	selectedTags: MinecraftTextureTagInfo[];
	hasMore: boolean;
	loading: boolean;
	tags: MinecraftTextureTagInfo[];
	testId?: string;
	triggerRef: RefObject<HTMLDivElement | null>;
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
	const [popoverRect, setPopoverRect] = useState<{
		left: number;
		top: number;
		width: number;
		maxHeight: number;
	} | null>(null);
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

	useEffect(() => {
		if (!open) return;

		function closeOnOutsidePointerDown(event: PointerEvent) {
			const target = event.target;
			if (
				target instanceof Node &&
				!triggerRef.current?.contains(target) &&
				!popoverRef.current?.contains(target)
			) {
				onOpenChange(false);
			}
		}

		function closeOnEscape(event: KeyboardEvent) {
			if (event.key === "Escape") {
				onOpenChange(false);
			}
		}

		document.addEventListener("pointerdown", closeOnOutsidePointerDown);
		document.addEventListener("keydown", closeOnEscape);
		return () => {
			document.removeEventListener("pointerdown", closeOnOutsidePointerDown);
			document.removeEventListener("keydown", closeOnEscape);
		};
	}, [onOpenChange, open, popoverRef, triggerRef]);

	useEffect(() => {
		if (!open) return;

		function updatePosition() {
			const trigger = triggerRef.current;
			if (!trigger) return;
			const rect = trigger.getBoundingClientRect();
			const width = Math.min(
				TAG_FILTER_POPOVER_WIDTH,
				window.innerWidth - TAG_FILTER_POPOVER_MARGIN * 2,
			);
			const left = Math.max(
				TAG_FILTER_POPOVER_MARGIN,
				Math.min(
					rect.right - width,
					window.innerWidth - width - TAG_FILTER_POPOVER_MARGIN,
				),
			);
			const top = Math.min(
				rect.bottom + TAG_FILTER_POPOVER_GAP,
				window.innerHeight - TAG_FILTER_POPOVER_MARGIN,
			);
			setPopoverRect({
				left,
				top,
				width,
				maxHeight: Math.max(
					240,
					window.innerHeight - top - TAG_FILTER_POPOVER_MARGIN,
				),
			});
		}

		updatePosition();
		window.addEventListener("resize", updatePosition);
		window.addEventListener("scroll", updatePosition, true);
		return () => {
			window.removeEventListener("resize", updatePosition);
			window.removeEventListener("scroll", updatePosition, true);
		};
	}, [open, triggerRef]);

	function toggleTag(tagId: number) {
		onSelectedIdsChange(
			selectedIds.includes(tagId)
				? selectedIds.filter((id) => id !== tagId)
				: [...selectedIds, tagId],
		);
	}

	function maybeLoadMore(event: UIEvent<HTMLDivElement>) {
		if (loading || !hasMore) return;
		const target = event.currentTarget;
		if (target.scrollHeight - target.scrollTop - target.clientHeight < 56) {
			onLoadMore();
		}
	}

	const popover = open
		? createPortal(
				<div
					ref={popoverRef}
					data-testid={testId}
					className="fixed z-50 grid gap-2 overflow-hidden rounded-lg border border-border/70 bg-popover p-3 text-popover-foreground shadow-xl"
					style={
						popoverRect
							? {
									left: popoverRect.left,
									top: popoverRect.top,
									width: popoverRect.width,
									maxHeight: popoverRect.maxHeight,
								}
							: {
									left: TAG_FILTER_POPOVER_MARGIN,
									top: TAG_FILTER_POPOVER_MARGIN,
									width: TAG_FILTER_POPOVER_WIDTH,
									maxHeight: `calc(100dvh - ${TAG_FILTER_POPOVER_MARGIN * 2}px)`,
								}
					}
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
						<TextureTagChips tags={selectedTags} />
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
					{tags.length === 0 && loading ? (
						<div className="rounded-lg border border-border/70 px-3 py-6 text-center text-sm text-muted-foreground">
							{t("common.loading")}
						</div>
					) : tags.length === 0 ? (
						<div className="rounded-lg border border-dashed border-border/70 px-3 py-6 text-center text-sm text-muted-foreground">
							{debouncedQuery
								? t("wardrobe.noTagSearchResults")
								: t("wardrobe.noAvailableTags")}
						</div>
					) : (
						<div
							className="min-h-0 max-h-[18rem] overflow-y-auto rounded-lg border border-border/70 bg-muted/20 p-2"
							onScroll={maybeLoadMore}
						>
							<div className="grid gap-1">
								{tags.map((tag) => (
									<label
										key={tag.id}
										className="flex min-w-0 cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-background/70"
									>
										<input
											type="checkbox"
											checked={selectedIds.includes(tag.id)}
											className="size-4 rounded border-border"
											onChange={() => toggleTag(tag.id)}
										/>
										<span
											aria-hidden="true"
											className="size-2.5 rounded-full"
											style={{ backgroundColor: tag.color }}
										/>
										<span className="truncate">{tag.name}</span>
									</label>
								))}
								{loading ? (
									<div className="px-2 py-1.5 text-xs text-muted-foreground">
										{t("common.loading")}
									</div>
								) : null}
							</div>
						</div>
					)}
					<div className="text-xs leading-5 text-muted-foreground">
						{t(`wardrobe.tagFilterHint.${searchMethod}`)}
					</div>
				</div>,
				document.body,
			)
		: null;

	return (
		<div ref={triggerRef} className="relative min-w-0">
			<Button
				type="button"
				variant={selectedTags.length > 0 || open ? "secondary" : "outline"}
				size="sm"
				className="w-full justify-between gap-2 sm:w-auto"
				aria-expanded={open}
				onClick={() => onOpenChange(!open)}
			>
				<span className="flex min-w-0 items-center gap-2">
					<Icon name="ListChecks" className="size-4 shrink-0" />
					<span className="truncate">{triggerLabel}</span>
				</span>
				<Icon name={open ? "CaretUp" : "CaretDown"} className="size-4" />
			</Button>
			{popover}
		</div>
	);
}

function TextureTagChips({ tags }: { tags: MinecraftTextureTagInfo[] }) {
	return (
		<div className="max-h-[10.5rem] overflow-y-auto pr-1">
			<div className="flex flex-wrap gap-1">
				{tags.map((tag) => (
					<span
						key={tag.id}
						className="rounded-md border px-1.5 py-0.5 text-[0.6875rem] font-medium"
						style={{
							borderColor: `${tag.color}55`,
							color: tag.color,
						}}
					>
						{tag.name}
					</span>
				))}
			</div>
		</div>
	);
}
