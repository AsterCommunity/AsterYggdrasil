import {
	type DragEvent,
	type FormEvent,
	type ReactNode,
	type UIEvent,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useTextureWardrobePageState } from "@/components/account/wardrobe-page/useTextureWardrobePageState";
import { DateTimeText } from "@/components/common/DateTimeText";
import { Badge } from "@/components/ui/badge";
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
import { Skeleton } from "@/components/ui/skeleton";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";
import { MinecraftTextureImagePreview } from "@/components/yggdrasil/MinecraftTextureImagePreview";
import { TextureTagFilterPopover } from "@/components/yggdrasil/TextureTagFilterPopover";
import { TextureUploadForm } from "@/components/yggdrasil/TextureUploadForm";
import { usePageTitle } from "@/hooks/usePageTitle";
import { useTextureTagPager } from "@/hooks/useTextureTagPager";
import { validateMinecraftTextureFile } from "@/lib/minecraftTextureValidation";
import { cn } from "@/lib/utils";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type {
	MinecraftTextureTagInfo,
	MinecraftTextureType,
	MinecraftWardrobeTextureMetadata,
	TextureTagSearchMethod,
} from "@/types/api";

const WARDROBE_PAGE_SIZE_OPTIONS = [10, 20] as const;
const DEFAULT_WARDROBE_PAGE_SIZE = 10;
const WARDROBE_SEARCH_DEBOUNCE_MS = 300;
const TAG_FILTER_APPLY_DEBOUNCE_MS = 240;

export default function TextureWardrobePage() {
	const { t } = useTranslation();
	const [state, dispatch] = useTextureWardrobePageState();
	const [textureOffset, setTextureOffset] = useState(0);
	const [texturePageSize, setTexturePageSize] = useState<number>(
		DEFAULT_WARDROBE_PAGE_SIZE,
	);
	const [activeTab, setActiveTab] = useState<MinecraftTextureType>("skin");
	const [uploadDialogOpen, setUploadDialogOpen] = useState(false);
	const [dragActive, setDragActive] = useState(false);
	const [selectedTagIds, setSelectedTagIds] = useState<number[]>([]);
	const [draftTagIds, setDraftTagIds] = useState<number[]>([]);
	const [tagFilterOpen, setTagFilterOpen] = useState(false);
	const [tagSearchMethod, setTagSearchMethod] =
		useState<TextureTagSearchMethod>("all");
	const tagFilterTriggerRef = useRef<HTMLDivElement | null>(null);
	const tagFilterPopoverRef = useRef<HTMLDivElement | null>(null);
	const [editTagIds, setEditTagIds] = useState<number[]>([]);
	const [editTagQuery, setEditTagQuery] = useState("");
	const yggdrasilConfig = useFrontendConfigStore((store) => store.yggdrasil);
	const [debouncedQuery, setDebouncedQuery] = useState("");
	const {
		activeTexture,
		deleteDialogOpen,
		deleteTexture,
		dialogOpen,
		editDialogOpen,
		editTexture,
		file,
		loading,
		model,
		profileQuery,
		profiles,
		query,
		selectedProfileId,
		submitting,
		textureTotal,
		textures,
		textureType,
		uploadName,
		visibility,
	} = state;

	usePageTitle(t("wardrobe.title"));

	const libraryTextureType: MinecraftTextureType =
		activeTab === "cape" ? "cape" : "skin";

	const loadProfiles = useCallback(async () => {
		try {
			const nextProfiles = await yggdrasilService.listProfiles();
			dispatch({ type: "profilesSuccess", profiles: nextProfiles.items });
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		}
	}, [dispatch]);

	const tagPager = useTextureTagPager({
		loadPage: yggdrasilService.listTextureLibraryTagsPage,
		onError: (error) => toast.error(formatUnknownError(error)),
		retainedTagIds: [...draftTagIds, ...editTagIds],
	});
	const {
		addTags,
		ensureLoaded: ensureTagsLoaded,
		hasMore: hasMoreTags,
		loadMore: loadMoreTags,
		loading: tagLoading,
		search: searchTags,
		tags,
	} = tagPager;

	const loadTextures = useCallback(
		async (
			nextOffset = textureOffset,
			nextType = libraryTextureType,
			nextQuery = debouncedQuery,
			nextTagIds = selectedTagIds,
			nextTagSearchMethod = tagSearchMethod,
		) => {
			dispatch({ type: "loading", value: true });
			const keyword = nextQuery.trim();
			const tagIds = Array.from(new Set(nextTagIds));
			try {
				const nextTextures = await yggdrasilService.listWardrobeTextures({
					limit: texturePageSize,
					offset: nextOffset,
					texture_type: nextType,
					keyword: keyword || undefined,
					tag_ids: tagIds.length > 0 ? tagIds : undefined,
					tag_search_method:
						tagIds.length > 0 ? nextTagSearchMethod : undefined,
				});
				dispatch({
					type: "texturesSuccess",
					textureTotal: nextTextures.total,
					textures: nextTextures.items,
				});
			} catch (nextError) {
				toast.error(formatUnknownError(nextError));
				dispatch({ type: "loading", value: false });
			} finally {
				dispatch({ type: "loading", value: false });
			}
		},
		[
			debouncedQuery,
			dispatch,
			libraryTextureType,
			selectedTagIds,
			tagSearchMethod,
			textureOffset,
			texturePageSize,
		],
	);

	useEffect(() => {
		const timer = window.setTimeout(() => {
			setTextureOffset(0);
			setDebouncedQuery(query.trim());
		}, WARDROBE_SEARCH_DEBOUNCE_MS);
		return () => window.clearTimeout(timer);
	}, [query]);

	useEffect(() => {
		if (tagFilterOpen || editDialogOpen) {
			ensureTagsLoaded();
		}
	}, [editDialogOpen, ensureTagsLoaded, tagFilterOpen]);

	useEffect(() => {
		void loadTextures();
	}, [loadTextures]);

	useEffect(() => {
		const timer = window.setTimeout(() => {
			setTextureOffset(0);
			setSelectedTagIds((current) =>
				sameNumberArray(current, draftTagIds) ? current : draftTagIds,
			);
		}, TAG_FILTER_APPLY_DEBOUNCE_MS);
		return () => window.clearTimeout(timer);
	}, [draftTagIds]);

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
			setTextureOffset(0);
			dispatch({ type: "prependTexture", value: uploaded });
			dispatch({ type: "activeTexture", value: uploaded });
			dispatch({ type: "file", value: null });
			dispatch({ type: "uploadName", value: "" });
			setActiveTab(uploaded.texture_type);
			setUploadDialogOpen(false);
			setDragActive(false);
			toast.success(t("wardrobe.uploadSuccess"));
			await loadTextures(0, uploaded.texture_type, debouncedQuery);
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
		toast.error(t(validation.key, validation.values));
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
		dispatch({ type: "editTexture", value: texture });
		addTags(texture.tags);
		setEditTagIds(texture.tags.map((tag) => tag.id));
		setEditTagQuery("");
		dispatch({ type: "editDialogOpen", value: true });
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
		dispatch({ type: "submitting", value: true });
		try {
			const updated = await yggdrasilService.updateWardrobeTexture(
				editTexture.id,
				{
					display_name: nextName || null,
					visibility:
						nextVisibility === "public" || nextVisibility === "private"
							? nextVisibility
							: editTexture.visibility,
				},
			);
			const tagged = await yggdrasilService.replaceWardrobeTextureTags(
				editTexture.id,
				{
					tag_ids: Array.from(new Set(editTagIds)),
				},
			);
			dispatch({
				type: "replaceTexture",
				value: { ...updated, tags: tagged.tags },
			});
			dispatch({ type: "editDialogOpen", value: false });
			toast.success(t("wardrobe.editSuccess"));
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	async function deleteWardrobeTexture() {
		if (!deleteTexture) return;
		dispatch({ type: "submitting", value: true });
		try {
			await yggdrasilService.deleteWardrobeTexture(deleteTexture.id);
			dispatch({ type: "removeTexture", id: deleteTexture.id });
			if (activeTexture?.id === deleteTexture.id) {
				dispatch({ type: "activeTexture", value: null });
			}
			toast.success(t("wardrobe.deleteSuccess"));
			dispatch({ type: "deleteDialogOpen", value: false });
			await loadTextures();
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	function selectTab(tab: MinecraftTextureType) {
		setActiveTab(tab);
		setTextureOffset(0);
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
		setTagSearchMethod(nextMethod);
		setTextureOffset(0);
		dispatch({ type: "activeTexture", value: null });
	}

	return (
		<div className="mx-auto grid w-full max-w-[96rem] gap-4 px-4 py-5 sm:px-6 lg:px-7">
			<header className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
				<div className="min-w-0">
					<h1 className="text-3xl font-semibold tracking-normal">
						{t("wardrobe.title")}
					</h1>
					<p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">
						{t("wardrobe.description")}
					</p>
				</div>
				<p className="text-sm font-medium text-muted-foreground sm:pt-2">
					{t("wardrobe.heroAside")}
				</p>
			</header>

			<div className="grid items-start gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(24rem,0.48fr)]">
				<section className="min-w-0 self-start overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs">
					<div className="flex flex-col gap-3 border-b border-border/70 bg-muted/35 p-3 lg:flex-row lg:items-center lg:justify-between">
						<div className="flex min-w-0 flex-wrap items-center gap-1">
							<WardrobeTabButton
								active={activeTab === "skin"}
								onClick={() => selectTab("skin")}
							>
								{t("wardrobe.type.skin")}
							</WardrobeTabButton>
							<WardrobeTabButton
								active={activeTab === "cape"}
								onClick={() => selectTab("cape")}
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
										searchBusy
											? "wardrobe-search-spinner"
											: "wardrobe-search-icon"
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
									onChange={(event) =>
										dispatch({
											type: "query",
											value: event.currentTarget.value,
										})
									}
								/>
							</div>
							<TextureTagFilterPopover
								open={tagFilterOpen}
								popoverRef={tagFilterPopoverRef}
								selectedIds={draftTagIds}
								selectedTags={selectedTags}
								searchMethod={tagSearchMethod}
								hasMore={hasMoreTags}
								loading={tagLoading}
								tags={tags}
								testId="wardrobe-tag-filter-popover"
								triggerRef={tagFilterTriggerRef}
								onClear={clearTagFilter}
								onLoadMore={loadMoreTags}
								onOpenChange={setTagFilterOpen}
								onSearchQueryChange={searchTags}
								onSearchMethodChange={changeTagSearchMethod}
								onSelectedIdsChange={(nextIds) => {
									setDraftTagIds(nextIds);
									dispatch({ type: "activeTexture", value: null });
								}}
							/>
							<Button
								type="button"
								variant="outline"
								size="sm"
								onClick={() => void loadTextures()}
								disabled={loading || submitting}
							>
								{loading ? t("common.loading") : t("common.refresh")}
							</Button>
							<Button
								type="button"
								size="sm"
								onClick={() => openUploadDialog()}
								disabled={submitting}
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
						) : visibleTextures.length === 0 ? (
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
										onClick={() => openUploadDialog()}
									>
										{t("wardrobe.uploadTab")}
									</Button>
								}
							/>
						) : (
							<div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
								{visibleTextures.map((texture) => (
									<TextureCard
										key={texture.id}
										active={previewTexture?.id === texture.id}
										texture={texture}
										date={<DateTimeText value={texture.created_at} />}
										onSelect={() =>
											dispatch({ type: "activeTexture", value: texture })
										}
									/>
								))}
							</div>
						)}
					</div>
					<WardrobePagination
						currentPage={Math.floor(textureOffset / texturePageSize) + 1}
						nextDisabled={textureOffset + texturePageSize >= textureTotal}
						prevDisabled={textureOffset === 0}
						pageSize={texturePageSize}
						total={textureTotal}
						totalPages={Math.max(1, Math.ceil(textureTotal / texturePageSize))}
						onNext={() =>
							setTextureOffset((current) => current + texturePageSize)
						}
						onPageSizeChange={(nextPageSize) => {
							setTexturePageSize(nextPageSize);
							setTextureOffset(0);
						}}
						onPrevious={() =>
							setTextureOffset((current) =>
								Math.max(0, current - texturePageSize),
							)
						}
					/>
				</section>

				<PreviewPanel
					texture={previewTexture}
					total={textureTotal}
					onBind={() => {
						if (previewTexture) openBindDialog(previewTexture);
					}}
					onDelete={() => {
						if (previewTexture) openDeleteDialog(previewTexture);
					}}
					onEdit={() => {
						if (previewTexture) openEditDialog(previewTexture);
					}}
				/>
			</div>

			<Dialog open={uploadDialogOpen} onOpenChange={setUploadDialogOpen}>
				<DialogContent
					keepMounted
					className="sm:max-w-lg"
					onAnimationEnd={clearUploadDialogStateAfterClose}
				>
					<TextureUploadForm
						description={t("wardrobe.uploadDescription")}
						dragActive={dragActive}
						file={file}
						fileInputId="wardrobe-texture-file"
						model={model}
						name={uploadName}
						nameLabel={t("wardrobe.textureName")}
						namePlaceholder={
							previewTexture
								? t("wardrobe.textureNamePlaceholder", {
										name: previewTexture.name,
									})
								: t("wardrobe.textureNamePlaceholderFallback")
						}
						submitLabel={t("common.upload")}
						submittingLabel={t("wardrobe.uploading")}
						submitting={submitting}
						textureType={textureType}
						title={t("wardrobe.uploadTitle")}
						visibility={visibility}
						onCancel={() => setUploadDialogOpen(false)}
						onDragEnter={dragTextureFile}
						onDragLeave={leaveTextureDropZone}
						onDrop={dropTextureFile}
						onFileChange={selectTextureFile}
						onModelChange={(nextModel) =>
							dispatch({ type: "model", value: nextModel })
						}
						onNameChange={(nextName) =>
							dispatch({ type: "uploadName", value: nextName })
						}
						onSubmit={uploadTexture}
						onTextureTypeChange={(nextType) => {
							dispatch({ type: "textureType", value: nextType });
							dispatch({ type: "file", value: null });
							dispatch({ type: "uploadName", value: "" });
							setDragActive(false);
						}}
						onVisibilityChange={(nextVisibility) =>
							dispatch({ type: "visibility", value: nextVisibility })
						}
					/>
				</DialogContent>
			</Dialog>

			<Dialog
				open={editDialogOpen}
				onOpenChange={(open) =>
					dispatch({ type: "editDialogOpen", value: open })
				}
			>
				<DialogContent
					keepMounted
					onAnimationEnd={clearEditDialogStateAfterClose}
				>
					<form
						key={editTexture?.id ?? "empty-edit-texture"}
						className="grid gap-4"
						onSubmit={updateWardrobeTexture}
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
							<div className="grid gap-1.5">
								<TextureTagSelector
									disabled={submitting}
									hasMore={hasMoreTags}
									loading={tagLoading}
									query={editTagQuery}
									selectedIds={editTagIds}
									tags={tags}
									onLoadMore={loadMoreTags}
									onQueryChange={setEditTagQuery}
									onSearchQueryChange={searchTags}
									onToggle={toggleEditTag}
								/>
							</div>
						</div>
						<DialogFooter>
							<DialogClose
								render={
									<Button
										type="button"
										variant="outline"
										disabled={submitting}
									/>
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

			<Dialog
				open={dialogOpen}
				onOpenChange={(open) => dispatch({ type: "dialogOpen", value: open })}
			>
				<DialogContent
					keepMounted
					onAnimationEnd={clearBindDialogStateAfterClose}
				>
					<DialogHeader>
						<DialogTitle>{t("wardrobe.bindDialogTitle")}</DialogTitle>
						<DialogDescription>
							{activeTexture
								? t("wardrobe.bindDialogDescription", {
										type: t(`wardrobe.type.${activeTexture.texture_type}`),
									})
								: t("wardrobe.bindDialogFallback")}
						</DialogDescription>
					</DialogHeader>

					<div className="grid gap-3">
						<Input
							value={profileQuery}
							placeholder={t("wardrobe.profileSearchPlaceholder")}
							onChange={(event) =>
								dispatch({
									type: "profileQuery",
									value: event.currentTarget.value,
								})
							}
						/>

						<div className="max-h-72 overflow-y-auto rounded-lg border border-border/70">
							{filteredProfiles.length === 0 ? (
								<div className="px-4 py-8 text-center text-sm text-muted-foreground">
									{profiles.length === 0
										? t("wardrobe.noProfiles")
										: t("wardrobe.noProfileSearchResults")}
								</div>
							) : (
								<div className="divide-y divide-border/70">
									{filteredProfiles.map((profile) => (
										<button
											key={profile.id}
											type="button"
											className={cn(
												"grid w-full gap-1 px-3 py-3 text-left transition-colors hover:bg-accent/35",
												selectedProfileId === profile.id && "bg-accent/50",
											)}
											onClick={() =>
												dispatch({
													type: "selectedProfileId",
													value: profile.id,
												})
											}
										>
											<span className="flex min-w-0 items-center gap-2">
												<span className="truncate font-medium">
													{profile.name}
												</span>
												{selectedProfileId === profile.id ? (
													<Badge variant="outline" className="rounded-md">
														{t("profiles.selected")}
													</Badge>
												) : null}
											</span>
										</button>
									))}
								</div>
							)}
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
						<Button
							type="button"
							disabled={!activeTexture || !selectedProfileId || submitting}
							onClick={() => void bindTexture()}
						>
							{submitting ? t("wardrobe.saving") : t("wardrobe.bindAction")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>

			<Dialog
				open={deleteDialogOpen}
				onOpenChange={(open) =>
					dispatch({ type: "deleteDialogOpen", value: open })
				}
			>
				<DialogContent
					keepMounted
					onAnimationEnd={clearDeleteDialogStateAfterClose}
				>
					<DialogHeader>
						<DialogTitle>{t("wardrobe.deleteDialogTitle")}</DialogTitle>
						<DialogDescription>
							{deleteTexture
								? t("wardrobe.deleteDialogDescription", {
										type: t(`wardrobe.type.${deleteTexture.texture_type}`),
									})
								: t("wardrobe.deleteDialogFallback")}
						</DialogDescription>
					</DialogHeader>

					{deleteTexture ? <TextureSummary texture={deleteTexture} /> : null}

					<DialogFooter>
						<DialogClose
							render={
								<Button type="button" variant="outline" disabled={submitting} />
							}
						>
							{t("common.cancel")}
						</DialogClose>
						<Button
							type="button"
							variant="destructive"
							disabled={!deleteTexture || submitting}
							onClick={() => void deleteWardrobeTexture()}
						>
							{submitting ? t("wardrobe.saving") : t("wardrobe.deleteAction")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}

function WardrobeTabButton({
	active,
	children,
	onClick,
}: {
	active: boolean;
	children: React.ReactNode;
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

function TextureCard({
	active,
	date,
	onSelect,
	texture,
}: {
	active: boolean;
	date: ReactNode;
	onSelect: () => void;
	texture: MinecraftWardrobeTextureMetadata;
}) {
	const { t } = useTranslation();
	const label = textureLabel(texture);

	return (
		<button
			type="button"
			className={cn(
				"group grid overflow-hidden rounded-md border bg-background text-left shadow-xs transition hover:-translate-y-0.5 hover:border-primary/45 hover:shadow-md focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35",
				active
					? "border-primary/70 ring-2 ring-primary/20"
					: "border-border/70",
			)}
			onClick={onSelect}
		>
			<TexturePreview texture={texture} compact />
			<div className="grid gap-1.5 border-t border-border/70 bg-muted/30 p-2.5">
				<div className="flex min-w-0 items-center justify-between gap-2">
					<div className="truncate text-xs font-semibold">{label}</div>
					<div className="flex shrink-0 items-center gap-1">
						<span className="rounded-md bg-background/80 px-1.5 py-0.5 text-[0.6875rem] text-muted-foreground">
							{t(`wardrobe.type.${texture.texture_type}`)}
						</span>
						<Badge
							variant="outline"
							className={cn(
								"h-5 rounded-md px-1.5 text-[0.6875rem]",
								textureVisibilityBadgeClass(texture.visibility),
							)}
						>
							{t(`wardrobe.visibility.${texture.visibility}`)}
						</Badge>
					</div>
				</div>
				<div className="flex flex-wrap gap-x-2 gap-y-1 text-[0.6875rem] text-muted-foreground">
					<span>
						{texture.width}x{texture.height}
					</span>
					<span>{formatBytes(texture.file_size)}</span>
					<span>{date}</span>
				</div>
				{texture.tags.length > 0 ? <TextureTags tags={texture.tags} /> : null}
			</div>
		</button>
	);
}

function PreviewPanel({
	onBind,
	onDelete,
	onEdit,
	texture,
	total,
}: {
	onBind: () => void;
	onDelete: () => void;
	onEdit: () => void;
	texture: MinecraftWardrobeTextureMetadata | null;
	total: number;
}) {
	const { t } = useTranslation();
	const skinUrl = texture?.texture_type === "skin" ? texture.url : null;
	const capeUrl = texture?.texture_type === "cape" ? texture.url : null;

	return (
		<aside className="grid min-w-0 gap-3 xl:sticky xl:top-20 xl:self-start">
			<MinecraftPreviewPanel
				label={t("wardrobe.previewTitle")}
				playerName={texture ? t(`wardrobe.type.${texture.texture_type}`) : null}
				skinUrl={skinUrl}
				capeUrl={capeUrl}
				model={texture?.texture_model ?? "default"}
				emptyTitle={t("wardrobe.previewEmptyTitle")}
				emptyDescription={t("wardrobe.previewEmptyDescription")}
				failedTitle={t("profiles.previewFailedTitle")}
				failedDescription={t("profiles.previewFailedDescription")}
				noSkinLabel={t("wardrobe.totalTextures", {
					count: total.toString(),
				})}
				idleLabel={t("profiles.motionIdle")}
				walkLabel={t("profiles.motionWalk")}
				frameClassName="h-[34rem]"
				skeletonClassName="h-[38rem]"
			/>
			<div className="grid gap-3 rounded-lg border border-border/70 bg-card/95 p-4 shadow-xs">
				{texture ? <TextureSummary texture={texture} /> : null}
				<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
					<div className="flex flex-wrap gap-2">
						<Button type="button" disabled={!texture} onClick={onBind}>
							{t("wardrobe.bindToProfile")}
						</Button>
						<Button
							type="button"
							variant="outline"
							disabled={!texture}
							onClick={onEdit}
						>
							<Icon name="PencilSimple" className="mr-2 size-4" />
							{t("wardrobe.editAction")}
						</Button>
					</div>
					<Button
						type="button"
						variant="destructive"
						disabled={!texture}
						onClick={onDelete}
					>
						{t("wardrobe.deleteAction")}
					</Button>
				</div>
			</div>
		</aside>
	);
}

function TexturePreview({
	compact,
	texture,
}: {
	compact?: boolean;
	texture: MinecraftWardrobeTextureMetadata;
}) {
	const { t } = useTranslation();
	const alt = t("wardrobe.texturePreviewAlt", {
		type: t(`wardrobe.type.${texture.texture_type}`),
	});

	return (
		<MinecraftTextureImagePreview
			alt={alt}
			aspect={compact ? "wide" : "portrait"}
			previewUrl={texture.preview_url}
			textureUrl={texture.url}
		/>
	);
}

function TextureSummary({
	texture,
}: {
	texture: MinecraftWardrobeTextureMetadata;
}) {
	const { t } = useTranslation();

	return (
		<div className="grid gap-2 rounded-lg border border-border/70 bg-muted/20 p-3 text-sm">
			<div className="min-w-0 truncate font-semibold">{texture.name}</div>
			<div className="flex flex-wrap items-center gap-2">
				<Badge variant="secondary" className="rounded-md">
					{t(`wardrobe.type.${texture.texture_type}`)}
				</Badge>
				{texture.texture_type === "skin" ? (
					<Badge variant="outline" className="rounded-md">
						{texture.texture_model}
					</Badge>
				) : null}
				<Badge variant="outline" className="rounded-md">
					{texture.width}x{texture.height}
				</Badge>
				<Badge variant="outline" className="rounded-md">
					{formatBytes(texture.file_size)}
				</Badge>
				<Badge
					variant="outline"
					className={cn(
						"rounded-md",
						textureVisibilityBadgeClass(texture.visibility),
					)}
				>
					{t(`wardrobe.visibility.${texture.visibility}`)}
				</Badge>
			</div>
			{texture.tags.length > 0 ? <TextureTags tags={texture.tags} /> : null}
		</div>
	);
}

function TextureTags({ tags }: { tags: MinecraftTextureTagInfo[] }) {
	return (
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
	);
}

function TextureTagSelector({
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

	useEffect(() => {
		const timer = window.setTimeout(() => {
			onSearchQueryChange(query.trim());
		}, 180);
		return () => window.clearTimeout(timer);
	}, [onSearchQueryChange, query]);

	function maybeLoadMore(event: UIEvent<HTMLDivElement>) {
		if (loading || !hasMore) return;
		const target = event.currentTarget;
		if (target.scrollHeight - target.scrollTop - target.clientHeight < 56) {
			onLoadMore();
		}
	}

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
					{tags.length > 0 ? (
						<div
							className="max-h-52 overflow-y-auto rounded-lg border border-border/70 bg-muted/20 p-2"
							onScroll={maybeLoadMore}
						>
							<div className="grid gap-1">
								{tags.map((tag) => (
									<label
										key={tag.id}
										className="flex min-w-0 cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-background/70 has-disabled:cursor-not-allowed has-disabled:opacity-60"
									>
										<input
											type="checkbox"
											checked={selectedIds.includes(tag.id)}
											disabled={disabled}
											className="size-4 rounded border-border"
											onChange={() => onToggle(tag.id)}
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
									<div className="px-2 py-1 text-xs text-muted-foreground">
										{t("common.loading")}
									</div>
								) : null}
							</div>
						</div>
					) : (
						<div className="rounded-lg border border-dashed border-border/70 px-3 py-2 text-sm text-muted-foreground">
							{loading
								? t("common.loading")
								: hasQuery
									? t("wardrobe.noTagSearchResults")
									: t("wardrobe.noAvailableTags")}
						</div>
					)}
				</>
			) : (
				<div className="rounded-lg border border-dashed border-border/70 px-3 py-2 text-sm text-muted-foreground">
					{t("wardrobe.noAvailableTags")}
				</div>
			)}
		</div>
	);
}

function textureVisibilityBadgeClass(
	visibility: MinecraftWardrobeTextureMetadata["visibility"],
) {
	return visibility === "public"
		? "border-emerald-500/35 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/35 dark:bg-emerald-400/10 dark:text-emerald-300"
		: "border-border/80 bg-muted/70 text-muted-foreground";
}

function sameNumberArray(left: number[], right: number[]) {
	return (
		left.length === right.length &&
		left.every((value, index) => value === right[index])
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
	action?: React.ReactNode;
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

function textureLabel(texture: MinecraftWardrobeTextureMetadata) {
	return texture.name;
}

function formatBytes(value: number) {
	if (value < 1024) return `${value} B`;
	const kib = value / 1024;
	if (kib < 1024) return `${kib.toFixed(1)} KiB`;
	return `${(kib / 1024).toFixed(1)} MiB`;
}
