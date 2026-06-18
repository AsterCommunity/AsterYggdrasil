import {
	type FormEvent,
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { toast } from "sonner";
import { AuthUserMenu } from "@/components/common/AuthUserMenu";
import { AppFooter } from "@/components/layout/AppFooter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { buttonVariants } from "@/components/ui/buttonVariants";
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
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";
import { MinecraftSkinAvatar } from "@/components/yggdrasil/MinecraftSkinAvatar";
import { MinecraftTextureImagePreview } from "@/components/yggdrasil/MinecraftTextureImagePreview";
import { TextureTagFilterPopover } from "@/components/yggdrasil/TextureTagFilterPopover";
import { usePageTitle } from "@/hooks/usePageTitle";
import { useTextureTagPager } from "@/hooks/useTextureTagPager";
import { cn } from "@/lib/utils";
import { publicPaths, publicTexturePath } from "@/routes/routePaths";
import { ApiError, formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type {
	MinecraftTextureType,
	PublicTextureLibraryTextureMetadata,
	TextureTagSearchMethod,
} from "@/types/api";

const LIBRARY_PAGE_SIZE_OPTIONS = [12, 24] as const;
const DEFAULT_LIBRARY_PAGE_SIZE = 12;
const TAG_FILTER_APPLY_DEBOUNCE_MS = 240;

export default function PublicTextureLibraryPage() {
	const { t } = useTranslation();
	const branding = useFrontendConfigStore((state) => state.branding);
	const user = useAuthStore((state) => state.user);
	const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
	const hydrate = useAuthStore((state) => state.hydrate);
	const logout = useAuthStore((state) => state.logout);
	const [textures, setTextures] = useState<
		PublicTextureLibraryTextureMetadata[]
	>([]);
	const [textureType, setTextureType] = useState<MinecraftTextureType | "all">(
		"all",
	);
	const [selectedTagIds, setSelectedTagIds] = useState<number[]>([]);
	const [draftTagIds, setDraftTagIds] = useState<number[]>([]);
	const [tagFilterOpen, setTagFilterOpen] = useState(false);
	const [tagSearchMethod, setTagSearchMethod] =
		useState<TextureTagSearchMethod>("all");
	const tagFilterTriggerRef = useRef<HTMLDivElement | null>(null);
	const tagFilterPopoverRef = useRef<HTMLDivElement | null>(null);
	const [query, setQuery] = useState("");
	const [appliedQuery, setAppliedQuery] = useState("");
	const [loading, setLoading] = useState(true);
	const [offset, setOffset] = useState(0);
	const [pageSize, setPageSize] = useState<number>(DEFAULT_LIBRARY_PAGE_SIZE);
	const [total, setTotal] = useState(0);
	const serverName = branding.title || t("home.titleFallback");

	usePageTitle(t("library.title"));

	useEffect(() => {
		void hydrate();
	}, [hydrate]);

	const tagPager = useTextureTagPager({
		loadPage: yggdrasilService.listPublicTextureLibraryTags,
		onError: (error) => toast.error(formatUnknownError(error)),
		retainedTagIds: draftTagIds,
	});
	const {
		ensureLoaded: ensureTagsLoaded,
		hasMore: hasMoreTags,
		loadMore: loadMoreTags,
		loading: tagLoading,
		search: searchTags,
		tags,
	} = tagPager;

	const loadTextures = useCallback(async () => {
		setLoading(true);
		try {
			const page = await yggdrasilService.listPublicTextureLibraryTextures({
				limit: pageSize,
				offset,
				keyword: appliedQuery.trim() || undefined,
				texture_type: textureType === "all" ? undefined : textureType,
				tag_ids: selectedTagIds.length > 0 ? selectedTagIds : undefined,
				tag_search_method:
					selectedTagIds.length > 0 ? tagSearchMethod : undefined,
			});
			setTextures(page.items);
			setTotal(page.total);
		} catch (error) {
			toast.error(formatUnknownError(error));
		} finally {
			setLoading(false);
		}
	}, [
		appliedQuery,
		offset,
		pageSize,
		selectedTagIds,
		tagSearchMethod,
		textureType,
	]);

	useEffect(() => {
		void loadTextures();
	}, [loadTextures]);

	useEffect(() => {
		if (tagFilterOpen) {
			ensureTagsLoaded();
		}
	}, [ensureTagsLoaded, tagFilterOpen]);

	useEffect(() => {
		const timer = window.setTimeout(() => {
			setOffset(0);
			setSelectedTagIds((current) =>
				sameNumberArray(current, draftTagIds) ? current : draftTagIds,
			);
		}, TAG_FILTER_APPLY_DEBOUNCE_MS);
		return () => window.clearTimeout(timer);
	}, [draftTagIds]);

	const selectedTags = useMemo(
		() => tags.filter((tag) => draftTagIds.includes(tag.id)),
		[tags, draftTagIds],
	);

	function submitSearch(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		setOffset(0);
		setAppliedQuery(query);
	}

	function clearTagFilter() {
		setDraftTagIds([]);
	}

	function changeTagSearchMethod(nextMethod: TextureTagSearchMethod) {
		setTagSearchMethod(nextMethod);
		setOffset(0);
	}

	const currentPage = Math.floor(offset / pageSize) + 1;
	const totalPages = Math.max(1, Math.ceil(total / pageSize));

	return (
		<PublicEntryShell
			branding={branding}
			title={serverName}
			tagline={t("brand.tagline")}
			variant="home"
			hideLanguageOnMobile
			headerActions={
				isAuthenticated && user ? (
					<AuthUserMenu
						user={user}
						scope="public"
						className="border-black/10 bg-white/64 text-[#102118] shadow-lg shadow-black/12 backdrop-blur hover:bg-white/80 aria-expanded:bg-white/80 dark:border-white/14 dark:bg-white/8 dark:text-white dark:shadow-black/20 dark:hover:bg-white/14 dark:aria-expanded:bg-white/14"
						onLogout={() => void logout()}
					/>
				) : (
					<Link
						to={publicPaths.login}
						className={cn(
							buttonVariants({ variant: "default", size: "sm" }),
							"h-10 rounded-lg border-emerald-300/24 bg-emerald-500 px-3 text-white shadow-lg shadow-emerald-950/35 hover:bg-emerald-400 sm:px-4",
						)}
					>
						<Icon name="SignIn" className="size-4" />
						<span className="hidden sm:inline">{t("home.loginRegister")}</span>
					</Link>
				)
			}
			footer={<AppFooter />}
		>
			<main className="relative z-10 min-w-0 flex-1">
				<div className="mx-auto grid w-full max-w-[92rem] gap-5 px-4 pt-6 pb-10 sm:px-8 lg:px-12">
					<header className="py-3">
						<div className="min-w-0">
							<Badge className="rounded-full border-emerald-700/20 bg-emerald-600/12 px-3 py-1 text-emerald-800 shadow-lg shadow-black/10 dark:border-emerald-300/24 dark:bg-emerald-400/14 dark:text-emerald-100">
								<Icon name="Images" className="size-3.5" />
								{t("library.eyebrow")}
							</Badge>
							<h1 className="mt-5 max-w-4xl text-balance font-black text-4xl leading-none tracking-normal text-[#102118] sm:text-6xl dark:text-white">
								{t("library.title")}
							</h1>
							<p className="mt-4 max-w-3xl text-sm leading-6 text-slate-700 sm:text-base dark:text-slate-300">
								{t("library.description")}
							</p>
						</div>
					</header>

					<section className="overflow-hidden rounded-xl border border-black/10 bg-white/76 shadow-2xl shadow-emerald-950/10 backdrop-blur-xl dark:border-white/10 dark:bg-white/[0.07] dark:shadow-black/25">
						<div className="flex flex-col gap-3 border-black/10 border-b bg-white/54 p-3 dark:border-white/10 dark:bg-white/[0.04] lg:flex-row lg:items-center lg:justify-between">
							<div className="flex min-w-0 flex-wrap items-center gap-1">
								<LibraryTypeButton
									active={textureType === "all"}
									onClick={() => {
										setTextureType("all");
										setOffset(0);
									}}
								>
									{t("library.type.all")}
								</LibraryTypeButton>
								<LibraryTypeButton
									active={textureType === "skin"}
									onClick={() => {
										setTextureType("skin");
										setOffset(0);
									}}
								>
									{t("wardrobe.type.skin")}
								</LibraryTypeButton>
								<LibraryTypeButton
									active={textureType === "cape"}
									onClick={() => {
										setTextureType("cape");
										setOffset(0);
									}}
								>
									{t("wardrobe.type.cape")}
								</LibraryTypeButton>
							</div>
							<form
								className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-center"
								onSubmit={submitSearch}
							>
								<div className="relative sm:w-80">
									<Icon
										name="MagnifyingGlass"
										aria-hidden="true"
										className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
									/>
									<Input
										value={query}
										placeholder={t("library.searchPlaceholder")}
										className="bg-white/82 pl-8 dark:bg-white/8"
										onChange={(event) => setQuery(event.currentTarget.value)}
									/>
								</div>
								<TextureTagFilterPopover
									open={tagFilterOpen}
									popoverRef={tagFilterPopoverRef}
									searchMethod={tagSearchMethod}
									selectedIds={draftTagIds}
									selectedTags={selectedTags}
									hasMore={hasMoreTags}
									loading={tagLoading}
									tags={tags}
									testId="library-tag-filter-popover"
									triggerRef={tagFilterTriggerRef}
									onClear={clearTagFilter}
									onLoadMore={loadMoreTags}
									onOpenChange={setTagFilterOpen}
									onSearchQueryChange={searchTags}
									onSearchMethodChange={changeTagSearchMethod}
									onSelectedIdsChange={setDraftTagIds}
								/>
								<Button type="submit" variant="outline" size="sm">
									{t("library.searchAction")}
								</Button>
								<Button
									type="button"
									size="sm"
									disabled={loading}
									onClick={() => void loadTextures()}
								>
									{loading ? t("common.loading") : t("common.refresh")}
								</Button>
							</form>
						</div>

						<div className="min-h-[28rem] p-3">
							{loading ? (
								<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
									{Array.from(
										{ length: 8 },
										(_, index) => `texture-library-skeleton-${index}`,
									).map((key) => (
										<Skeleton key={key} className="h-72 rounded-lg" />
									))}
								</div>
							) : textures.length === 0 ? (
								<div className="grid min-h-[24rem] place-items-center rounded-lg border border-dashed border-black/15 bg-white/45 px-4 text-center dark:border-white/15 dark:bg-white/[0.04]">
									<div className="max-w-md">
										<h2 className="text-lg font-semibold tracking-normal">
											{t("library.emptyTitle")}
										</h2>
										<p className="mt-2 text-sm leading-6 text-muted-foreground">
											{t("library.emptyDescription")}
										</p>
									</div>
								</div>
							) : (
								<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
									{textures.map((texture) => (
										<PublicTextureCard key={texture.id} texture={texture} />
									))}
								</div>
							)}
						</div>

						{total > 0 ? (
							<div className="flex flex-col gap-3 border-black/10 border-t bg-white/45 px-4 py-3 dark:border-white/10 dark:bg-white/[0.035] sm:flex-row sm:items-center sm:justify-between">
								<div className="text-sm text-muted-foreground">
									{t("admin.pagination.entriesPage", {
										current: currentPage,
										pages: totalPages,
										total,
									})}
								</div>
								<div className="flex flex-wrap items-center gap-2">
									<label className="flex items-center gap-2 text-sm text-muted-foreground">
										<span>{t("admin.pagination.pageSize")}</span>
										<select
											value={pageSize}
											aria-label={t("admin.pagination.pageSize")}
											className="h-8 rounded-md border border-input bg-background px-2 text-sm"
											onChange={(event) => {
												setPageSize(Number(event.currentTarget.value));
												setOffset(0);
											}}
										>
											{LIBRARY_PAGE_SIZE_OPTIONS.map((option) => (
												<option key={option} value={option}>
													{t("admin.pagination.pageSizeOption", {
														count: option,
													})}
												</option>
											))}
										</select>
									</label>
									<Button
										type="button"
										variant="outline"
										size="sm"
										disabled={offset === 0}
										onClick={() =>
											setOffset((current) => Math.max(0, current - pageSize))
										}
									>
										{t("admin.pagination.previous")}
									</Button>
									<Button
										type="button"
										variant="outline"
										size="sm"
										disabled={offset + pageSize >= total}
										onClick={() => setOffset((current) => current + pageSize)}
									>
										{t("admin.pagination.next")}
									</Button>
								</div>
							</div>
						) : null}
					</section>
				</div>
			</main>
		</PublicEntryShell>
	);
}

function LibraryTypeButton({
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

function sameNumberArray(left: number[], right: number[]) {
	return (
		left.length === right.length &&
		left.every((value, index) => value === right[index])
	);
}

function PublicTextureCard({
	texture,
}: {
	texture: PublicTextureLibraryTextureMetadata;
}) {
	const { t } = useTranslation();

	return (
		<Link
			to={publicTexturePath(texture.id)}
			className="group grid overflow-hidden rounded-md border border-border/70 bg-background text-left shadow-xs transition hover:-translate-y-0.5 hover:border-primary/45 hover:shadow-md focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35"
		>
			<MinecraftTextureImagePreview
				alt={t("library.texturePreviewAlt", {
					type: t(`wardrobe.type.${texture.texture_type}`),
				})}
				draggable={false}
				previewUrl={texture.preview_url}
				textureUrl={texture.url}
			/>
			<div className="flex min-w-0 gap-3 border-t border-border/70 bg-muted/25 p-3">
				<PublicTextureCardAvatar texture={texture} />
				<div className="grid min-w-0 flex-1 gap-3">
					<div className="min-w-0">
						<h2 className="truncate text-sm font-semibold tracking-normal group-hover:text-primary">
							{texture.name}
						</h2>
						<div className="mt-2 flex flex-wrap items-center gap-1.5">
							<Badge variant="secondary" className="rounded-md">
								{t(`wardrobe.type.${texture.texture_type}`)}
							</Badge>
							{texture.texture_type === "skin" ? (
								<Badge variant="outline" className="rounded-md">
									{texture.texture_model}
								</Badge>
							) : null}
						</div>
					</div>
					<div className="flex flex-wrap gap-x-2 gap-y-1 text-xs text-muted-foreground">
						<span>
							{texture.uploader?.name ?? t("library.unknownUploader")}
						</span>
						<span>
							{texture.width}x{texture.height}
						</span>
						<span>{formatBytes(texture.file_size)}</span>
					</div>
					{texture.tags.length > 0 ? (
						<div className="flex flex-wrap gap-1">
							{texture.tags.map((tag) => (
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
					) : null}
				</div>
			</div>
		</Link>
	);
}

function PublicTextureCardAvatar({
	texture,
}: {
	texture: PublicTextureLibraryTextureMetadata;
}) {
	const { t } = useTranslation();

	if (texture.texture_type === "skin") {
		return (
			<MinecraftSkinAvatar
				name={texture.name}
				skinUrl={texture.url}
				className="size-12 rounded-lg"
				testId={`public-texture-card-avatar-${texture.id}`}
				imageTestId={`public-texture-card-avatar-image-${texture.id}`}
			/>
		);
	}

	return (
		<span
			aria-hidden="true"
			className="grid size-12 shrink-0 place-items-center overflow-hidden rounded-lg bg-background/70 p-2 shadow-xs dark:bg-white/7"
			data-testid={`public-texture-card-avatar-${texture.id}`}
			title={texture.name}
		>
			<img
				src={texture.url}
				alt={t("library.texturePreviewAlt", {
					type: t(`wardrobe.type.${texture.texture_type}`),
				})}
				crossOrigin="anonymous"
				draggable={false}
				className="max-h-full max-w-full object-contain [image-rendering:pixelated]"
				data-testid={`public-texture-card-avatar-image-${texture.id}`}
			/>
		</span>
	);
}

export function PublicTextureDetail({
	texture,
}: {
	texture: PublicTextureLibraryTextureMetadata;
}) {
	const { t } = useTranslation();
	const skinUrl = texture.texture_type === "skin" ? texture.url : null;
	const capeUrl = texture.texture_type === "cape" ? texture.url : null;

	return (
		<div className="grid gap-4 sm:grid-cols-[minmax(0,0.85fr)_minmax(0,1fr)]">
			<MinecraftPreviewPanel
				label={t("wardrobe.previewTitle")}
				playerName={texture.name}
				skinUrl={skinUrl}
				capeUrl={capeUrl}
				compactHeader
				model={texture.texture_model}
				emptyTitle={t("wardrobe.previewEmptyTitle")}
				emptyDescription={t("wardrobe.previewEmptyDescription")}
				failedTitle={t("profiles.previewFailedTitle")}
				failedDescription={t("profiles.previewFailedDescription")}
				noSkinLabel={t("profiles.noSkinTexture")}
				idleLabel={t("profiles.motionIdle")}
				walkLabel={t("profiles.motionWalk")}
				frameClassName="h-72 sm:h-80"
				skeletonClassName="h-[23rem]"
				containerClassName="min-w-0"
			/>
			<div className="grid content-start gap-3">
				<DetailRow label={t("library.textureName")} value={texture.name} />
				<DetailRow
					label={t("library.textureType")}
					value={formatTextureKind(texture, t)}
				/>
				<DetailRow
					label={t("library.uploader")}
					value={texture.uploader?.name ?? t("library.unknownUploader")}
				/>
				<div className="grid gap-1.5">
					<div className="text-xs font-medium text-muted-foreground">
						{t("wardrobe.visibility.label")}
					</div>
					<div>
						<Badge
							variant="outline"
							className="rounded-md border-emerald-500/35 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/35 dark:bg-emerald-400/10 dark:text-emerald-300"
						>
							{t("wardrobe.visibility.public")}
						</Badge>
					</div>
				</div>
				<div className="grid grid-cols-2 gap-2">
					<DetailRow
						label={t("wardrobe.dimensions")}
						value={`${texture.width}x${texture.height}`}
					/>
					<DetailRow
						label={t("wardrobe.size")}
						value={formatBytes(texture.file_size)}
					/>
				</div>
				<div className="grid gap-1.5">
					<div className="text-xs font-medium text-muted-foreground">
						{t("library.tags")}
					</div>
					{texture.tags.length > 0 ? (
						<div className="flex flex-wrap gap-1">
							{texture.tags.map((tag) => (
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
					) : (
						<div className="text-sm text-muted-foreground">
							{t("library.noTags")}
						</div>
					)}
				</div>
			</div>
		</div>
	);
}

function DetailRow({
	label,
	mono,
	value,
}: {
	label: string;
	mono?: boolean;
	value: string;
}) {
	return (
		<div className="grid min-w-0 gap-1.5">
			<div className="text-xs font-medium text-muted-foreground">{label}</div>
			<div
				className={cn(
					"min-w-0 break-words text-sm font-medium",
					mono && "font-mono text-xs leading-5",
				)}
			>
				{value}
			</div>
		</div>
	);
}

export function PublicTextureCopyDialog({
	onCopied,
	onOpenChange,
	open,
	texture,
}: {
	onCopied?: () => void;
	onOpenChange: (open: boolean) => void;
	open: boolean;
	texture: PublicTextureLibraryTextureMetadata | null;
}) {
	const { t } = useTranslation();
	const [copying, setCopying] = useState(false);
	const [copyName, setCopyName] = useState("");
	const [copyError, setCopyError] = useState<string | null>(null);

	useEffect(() => {
		if (open && texture) {
			setCopyName(texture.name);
			setCopyError(null);
		}
	}, [open, texture]);

	function clearCopyDialogStateAfterClose() {
		if (open) return;
		setCopyName("");
		setCopyError(null);
	}

	async function copyTexture(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!texture) return;
		setCopying(true);
		setCopyError(null);
		try {
			const copied = await yggdrasilService.copyPublicTextureToWardrobe(
				texture.id,
				{
					display_name: copyName.trim() || null,
				},
			);
			toast.success(t("library.copySuccess", { name: copied.name }));
			onOpenChange(false);
			onCopied?.();
		} catch (error) {
			if (
				error instanceof ApiError &&
				error.code === "wardrobe.texture_name_taken"
			) {
				setCopyError(t("library.copyNameTaken"));
			} else {
				toast.error(formatUnknownError(error));
			}
		} finally {
			setCopying(false);
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				keepMounted
				className="grid-rows-[auto_minmax(0,1fr)_auto] sm:max-w-md"
				onAnimationEnd={clearCopyDialogStateAfterClose}
			>
				<DialogHeader>
					<DialogTitle>{t("library.copyDialogTitle")}</DialogTitle>
					<DialogDescription>
						{texture
							? t("library.copyDialogDescription", {
									name: texture.name,
								})
							: t("library.detailFallback")}
					</DialogDescription>
				</DialogHeader>

				{texture ? (
					<form
						id="public-texture-copy-form"
						className="min-h-0 space-y-3 overflow-y-auto pr-1"
						onSubmit={(event) => void copyTexture(event)}
					>
						<div className="grid gap-2">
							<Label htmlFor="public-texture-copy-name">
								{t("library.copyNameLabel")}
							</Label>
							<Input
								id="public-texture-copy-name"
								value={copyName}
								maxLength={96}
								disabled={copying}
								aria-invalid={copyError ? true : undefined}
								aria-describedby={
									copyError
										? "public-texture-copy-name-error"
										: "public-texture-copy-name-help"
								}
								onChange={(event) => {
									setCopyName(event.currentTarget.value);
									setCopyError(null);
								}}
							/>
							<p
								id="public-texture-copy-name-help"
								className="text-xs leading-5 text-muted-foreground"
							>
								{t("library.copyNameHelp")}
							</p>
							{copyError ? (
								<p
									id="public-texture-copy-name-error"
									className="text-sm font-medium text-destructive"
								>
									{copyError}
								</p>
							) : null}
						</div>
					</form>
				) : null}

				<DialogFooter>
					<Button
						type="button"
						variant="outline"
						disabled={copying}
						onClick={() => onOpenChange(false)}
					>
						{t("common.cancel")}
					</Button>
					<Button
						type="submit"
						form="public-texture-copy-form"
						disabled={!texture || copying}
					>
						<Icon
							name={copying ? "Spinner" : "Copy"}
							className={cn("size-4", copying && "animate-spin")}
						/>
						{copying ? t("library.copying") : t("library.copyConfirmAction")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

export function formatTextureKind(
	texture: Pick<
		PublicTextureLibraryTextureMetadata,
		"texture_model" | "texture_type"
	>,
	t: ReturnType<typeof useTranslation>["t"],
) {
	if (texture.texture_type !== "skin") {
		return t(`wardrobe.type.${texture.texture_type}`);
	}
	return `${t("wardrobe.type.skin")} / ${
		texture.texture_model === "slim"
			? t("profiles.slimModel")
			: t("profiles.defaultModel")
	}`;
}

export function formatBytes(bytes: number) {
	if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
	const units = ["B", "KiB", "MiB", "GiB"] as const;
	let value = bytes;
	let index = 0;
	while (value >= 1024 && index < units.length - 1) {
		value /= 1024;
		index += 1;
	}
	return `${value >= 10 || index === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[index]}`;
}
