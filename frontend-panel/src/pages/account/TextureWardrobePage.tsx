import {
	type DragEvent,
	type FormEvent,
	lazy,
	Suspense,
	useCallback,
	useEffect,
	useMemo,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useTextureWardrobePageState } from "@/components/account/wardrobe-page/useTextureWardrobePageState";
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
import { TextureUploadForm } from "@/components/yggdrasil/TextureUploadForm";
import { usePageTitle } from "@/hooks/usePageTitle";
import { validateMinecraftTextureFile } from "@/lib/minecraftTextureValidation";
import { cn } from "@/lib/utils";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type {
	MinecraftTextureType,
	MinecraftWardrobeTextureMetadata,
} from "@/types/api";

const WARDROBE_PAGE_SIZE_OPTIONS = [10, 20] as const;
const DEFAULT_WARDROBE_PAGE_SIZE = 10;
const WARDROBE_SEARCH_DEBOUNCE_MS = 300;

const MinecraftPreview = lazy(() =>
	import("@/components/yggdrasil/MinecraftPreview").then((module) => ({
		default: module.MinecraftPreview,
	})),
);

export default function TextureWardrobePage() {
	const { t, i18n } = useTranslation();
	const [state, dispatch] = useTextureWardrobePageState();
	const [textureOffset, setTextureOffset] = useState(0);
	const [texturePageSize, setTexturePageSize] = useState<number>(
		DEFAULT_WARDROBE_PAGE_SIZE,
	);
	const [activeTab, setActiveTab] = useState<MinecraftTextureType>("skin");
	const [uploadDialogOpen, setUploadDialogOpen] = useState(false);
	const [dragActive, setDragActive] = useState(false);
	const yggdrasilConfig = useFrontendConfigStore((store) => store.yggdrasil);
	const [debouncedQuery, setDebouncedQuery] = useState("");
	const {
		activeTexture,
		deleteDialogOpen,
		deleteTexture,
		dialogOpen,
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
		visibility,
	} = state;

	usePageTitle(t("wardrobe.title"));

	const libraryTextureType: MinecraftTextureType =
		activeTab === "cape" ? "cape" : "skin";

	const loadData = useCallback(
		async (
			nextOffset = textureOffset,
			nextType = libraryTextureType,
			nextQuery = debouncedQuery,
		) => {
			dispatch({ type: "loading", value: true });
			const keyword = nextQuery.trim();
			try {
				const [nextProfiles, nextTextures] = await Promise.all([
					yggdrasilService.listProfiles(),
					yggdrasilService.listWardrobeTextures({
						limit: texturePageSize,
						offset: nextOffset,
						texture_type: nextType,
						keyword: keyword || undefined,
					}),
				]);
				dispatch({
					type: "loadSuccess",
					profiles: nextProfiles.items,
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
		void loadData();
	}, [loadData]);

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

	const formatter = useMemo(
		() => new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium" }),
		[i18n.language],
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
				visibility,
			});
			setTextureOffset(0);
			dispatch({ type: "prependTexture", value: uploaded });
			dispatch({ type: "activeTexture", value: uploaded });
			dispatch({ type: "file", value: null });
			setActiveTab(uploaded.texture_type);
			setUploadDialogOpen(false);
			setDragActive(false);
			toast.success(t("wardrobe.uploadSuccess"));
			await loadData(0, uploaded.texture_type, debouncedQuery);
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
			await loadData();
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
							<Button
								type="button"
								variant="outline"
								size="sm"
								onClick={() => void loadData()}
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
										date={formatter.format(new Date(texture.created_at))}
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
				/>
			</div>

			<Dialog open={uploadDialogOpen} onOpenChange={setUploadDialogOpen}>
				<DialogContent keepMounted className="sm:max-w-lg">
					<TextureUploadForm
						description={t("wardrobe.uploadDescription")}
						dragActive={dragActive}
						file={file}
						fileInputId="wardrobe-texture-file"
						model={model}
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
						onSubmit={uploadTexture}
						onTextureTypeChange={(nextType) => {
							dispatch({ type: "textureType", value: nextType });
							dispatch({ type: "file", value: null });
							setDragActive(false);
						}}
						onVisibilityChange={(nextVisibility) =>
							dispatch({ type: "visibility", value: nextVisibility })
						}
					/>
				</DialogContent>
			</Dialog>

			<Dialog
				open={dialogOpen}
				onOpenChange={(open) => dispatch({ type: "dialogOpen", value: open })}
			>
				<DialogContent keepMounted>
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
				<DialogContent keepMounted>
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
	date: string;
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
					<span className="rounded-md bg-background/80 px-1.5 py-0.5 text-[0.6875rem] text-muted-foreground">
						{t(`wardrobe.type.${texture.texture_type}`)}
					</span>
				</div>
				<div className="flex flex-wrap gap-x-2 gap-y-1 text-[0.6875rem] text-muted-foreground">
					<span>
						{texture.width}x{texture.height}
					</span>
					<span>{formatBytes(texture.file_size)}</span>
					<span>{date}</span>
				</div>
			</div>
		</button>
	);
}

function PreviewPanel({
	onBind,
	onDelete,
	texture,
	total,
}: {
	onBind: () => void;
	onDelete: () => void;
	texture: MinecraftWardrobeTextureMetadata | null;
	total: number;
}) {
	const { t } = useTranslation();
	const skinUrl = texture?.texture_type === "skin" ? texture.url : null;
	const capeUrl = texture?.texture_type === "cape" ? texture.url : null;

	return (
		<aside className="grid min-w-0 gap-3 xl:sticky xl:top-20 xl:self-start">
			<Suspense fallback={<Skeleton className="h-[38rem] rounded-lg" />}>
				<MinecraftPreview
					label={t("wardrobe.previewTitle")}
					playerName={
						texture ? t(`wardrobe.type.${texture.texture_type}`) : null
					}
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
				/>
			</Suspense>
			<div className="grid gap-3 rounded-lg border border-border/70 bg-card/95 p-4 shadow-xs">
				{texture ? <TextureSummary texture={texture} /> : null}
				<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
					<div className="flex flex-wrap gap-2">
						<Button type="button" disabled={!texture} onClick={onBind}>
							{t("wardrobe.bindToProfile")}
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
		<div
			className={cn(
				"grid place-items-center bg-[linear-gradient(45deg,hsl(var(--muted))_25%,transparent_25%),linear-gradient(-45deg,hsl(var(--muted))_25%,transparent_25%),linear-gradient(45deg,transparent_75%,hsl(var(--muted))_75%),linear-gradient(-45deg,transparent_75%,hsl(var(--muted))_75%)] bg-[length:18px_18px] bg-[position:0_0,0_9px,9px_-9px,-9px_0] p-4",
				compact ? "aspect-[4/3]" : "aspect-[4/5]",
			)}
		>
			<img
				src={texture.url}
				alt={alt}
				className="max-h-full max-w-full object-contain [image-rendering:pixelated]"
			/>
		</div>
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
			</div>
		</div>
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
	return texture.hash.slice(0, 16);
}

function formatBytes(value: number) {
	if (value < 1024) return `${value} B`;
	const kib = value / 1024;
	if (kib < 1024) return `${kib.toFixed(1)} KiB`;
	return `${(kib / 1024).toFixed(1)} MiB`;
}
