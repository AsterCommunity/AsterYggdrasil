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
import { useMinecraftProfilesPageState } from "@/components/account/profiles-page/useMinecraftProfilesPageState";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
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
import { Icon, type IconName } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { TextureUploadForm } from "@/components/yggdrasil/TextureUploadForm";
import { usePageTitle } from "@/hooks/usePageTitle";
import { validateMinecraftTextureFile } from "@/lib/minecraftTextureValidation";
import { cn } from "@/lib/utils";
import { formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type {
	MinecraftTextureMetadata,
	MinecraftTextureType,
} from "@/types/api";

const PROFILE_PAGE_SIZE_OPTIONS = [5, 10] as const;
const DEFAULT_PROFILE_PAGE_SIZE = 5;
const PROFILE_SEARCH_DEBOUNCE_MS = 300;

const MinecraftPreview = lazy(() =>
	import("@/components/yggdrasil/MinecraftPreview").then((module) => ({
		default: module.MinecraftPreview,
	})),
);

export default function MinecraftProfilesPage() {
	const { t } = useTranslation();
	const [state, dispatch] = useMinecraftProfilesPageState();
	const [profileOffset, setProfileOffset] = useState(0);
	const [profilePageSize, setProfilePageSize] = useState<number>(
		DEFAULT_PROFILE_PAGE_SIZE,
	);
	const [debouncedQuery, setDebouncedQuery] = useState("");
	const [profilesLoading, setProfilesLoading] = useState(false);
	const {
		file,
		loading,
		model,
		profileName,
		profileTotal,
		profiles,
		query,
		selectedUuid,
		textures,
		texturesLoading,
		textureType,
		visibility,
	} = state;

	usePageTitle(t("profiles.title"));

	const loadProfiles = useCallback(
		async (nextOffset = profileOffset, nextPageSize = profilePageSize) => {
			const trimmedQuery = debouncedQuery.trim();
			const params = {
				limit: nextPageSize,
				offset: nextOffset,
			};
			setProfilesLoading(true);
			try {
				const next = await yggdrasilService.listProfiles(
					trimmedQuery ? { ...params, query: trimmedQuery } : params,
				);
				dispatch({ type: "profilePage", value: next });
			} finally {
				setProfilesLoading(false);
			}
		},
		[debouncedQuery, dispatch, profileOffset, profilePageSize],
	);

	const loadTextures = useCallback(
		async (uuid: string) => {
			if (!uuid) {
				dispatch({ type: "textures", value: [] });
				return;
			}
			dispatch({ type: "texturesLoading", value: true });
			try {
				dispatch({
					type: "textures",
					value: await yggdrasilService.listProfileTextures(uuid),
				});
			} catch (nextError) {
				toast.error(formatUnknownError(nextError));
				dispatch({ type: "textures", value: [] });
			} finally {
				dispatch({ type: "texturesLoading", value: false });
			}
		},
		[dispatch],
	);

	useEffect(() => {
		void loadProfiles().catch((nextError) =>
			toast.error(formatUnknownError(nextError)),
		);
	}, [loadProfiles]);

	useEffect(() => {
		const timeout = window.setTimeout(() => {
			setProfileOffset(0);
			setDebouncedQuery(query.trim());
		}, PROFILE_SEARCH_DEBOUNCE_MS);
		return () => window.clearTimeout(timeout);
	}, [query]);

	useEffect(() => {
		void loadTextures(selectedUuid);
	}, [selectedUuid, loadTextures]);

	const selectedProfile = useMemo(
		() => profiles.find((profile) => profile.id === selectedUuid) ?? null,
		[profiles, selectedUuid],
	);
	const searchBusy =
		query.trim() !== debouncedQuery.trim() ||
		(profilesLoading && Boolean(debouncedQuery.trim()));
	const skinTexture =
		textures.find((texture) => texture.texture_type === "skin") ?? null;
	const capeTexture =
		textures.find((texture) => texture.texture_type === "cape") ?? null;
	const activeTexture = textureType === "skin" ? skinTexture : capeTexture;

	const [textureDialogOpen, setTextureDialogOpen] = useState(false);
	const [textureManageDialogOpen, setTextureManageDialogOpen] = useState(false);
	const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
	const [deleteProfileDialogOpen, setDeleteProfileDialogOpen] = useState(false);
	const [renameDialogOpen, setRenameDialogOpen] = useState(false);
	const [uploadTextureType, setUploadTextureType] =
		useState<MinecraftTextureType>("skin");
	const yggdrasilConfig = useFrontendConfigStore((store) => store.yggdrasil);
	const [renameUuid, setRenameUuid] = useState("");
	const [renameName, setRenameName] = useState("");
	const [deletingProfile, setDeletingProfile] = useState(false);
	const [renaming, setRenaming] = useState(false);
	const [dragActive, setDragActive] = useState(false);

	async function createProfile(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		dispatch({ type: "loading", value: true });
		try {
			const created = await yggdrasilService.createProfile({
				name: profileName,
			});
			dispatch({ type: "profileName", value: "" });
			setProfileOffset(0);
			await loadProfiles(0, profilePageSize);
			dispatch({ type: "selectedUuid", value: created.id });
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	function openRenameDialog(profile: { id: string; name: string }) {
		setRenameUuid(profile.id);
		setRenameName(profile.name);
		setRenameDialogOpen(true);
	}

	async function renameProfile(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!renameUuid || !renameName.trim()) return;
		setRenaming(true);
		try {
			const renamed = await yggdrasilService.renameProfile(renameUuid, {
				name: renameName.trim(),
			});
			setRenameDialogOpen(false);
			setRenameUuid("");
			setRenameName("");
			setProfileOffset(0);
			await loadProfiles(0, profilePageSize);
			dispatch({ type: "selectedUuid", value: renamed.id });
			toast.success(t("profiles.renameToast"));
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			setRenaming(false);
		}
	}

	async function deleteProfile() {
		if (!selectedUuid || !selectedProfile) return;
		setDeletingProfile(true);
		try {
			await yggdrasilService.deleteProfile(selectedUuid);
			setDeleteProfileDialogOpen(false);
			setProfileOffset(0);
			dispatch({ type: "selectedUuid", value: "" });
			await loadProfiles(0, profilePageSize);
			toast.success(t("profiles.deleteProfileToast"));
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			setDeletingProfile(false);
		}
	}

	async function uploadTexture(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!file || !selectedUuid) return;
		if (!(await validateTextureFile(file))) return;
		dispatch({ type: "loading", value: true });
		try {
			const uploaded = await yggdrasilService.uploadWardrobeTexture({
				textureType: uploadTextureType,
				file,
				model,
				visibility,
			});
			await yggdrasilService.bindProfileTexture({
				uuid: selectedUuid,
				textureType: uploaded.texture_type,
				textureId: uploaded.id,
			});
			setTextureDialogOpen(false);
			dispatch({ type: "file", value: null });
			toast.success(t("profiles.uploadAndBindToast"));
			await loadTextures(selectedUuid);
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	async function validateTextureFile(nextFile: File) {
		const validation = await validateMinecraftTextureFile(
			nextFile,
			uploadTextureType,
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

	async function deleteTexture() {
		if (!selectedUuid) return;
		dispatch({ type: "loading", value: true });
		try {
			await yggdrasilService.unbindProfileTexture({
				uuid: selectedUuid,
				textureType,
			});
			setDeleteDialogOpen(false);
			toast.success(t("profiles.deleteSuccess"));
			await loadTextures(selectedUuid);
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	function openTextureDialog(nextTextureType: MinecraftTextureType) {
		setUploadTextureType(nextTextureType);
		dispatch({ type: "textureType", value: nextTextureType });
		dispatch({
			type: "model",
			value:
				nextTextureType === "skin"
					? (skinTexture?.texture_model ?? model)
					: model,
		});
		dispatch({ type: "file", value: null });
		setDragActive(false);
		setTextureDialogOpen(true);
	}

	function openDeleteTextureDialog(nextTextureType: MinecraftTextureType) {
		dispatch({ type: "textureType", value: nextTextureType });
		setDeleteDialogOpen(true);
	}

	function changeProfilePageSize(value: string | null) {
		const parsed = Number(value);
		const nextPageSize = PROFILE_PAGE_SIZE_OPTIONS.includes(
			parsed as (typeof PROFILE_PAGE_SIZE_OPTIONS)[number],
		)
			? parsed
			: DEFAULT_PROFILE_PAGE_SIZE;
		setProfilePageSize(nextPageSize);
		setProfileOffset(0);
		void loadProfiles(0, nextPageSize).catch((nextError) =>
			toast.error(formatUnknownError(nextError)),
		);
	}

	return (
		<div className="mx-auto w-full max-w-[96rem] px-4 py-5 sm:px-6 lg:px-7">
			<div className="mb-5 border-b border-border/70 pb-5 dark:border-white/10">
				<h1 className="text-2xl font-semibold tracking-normal text-foreground sm:text-3xl">
					{t("profiles.title")}
				</h1>
				<p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
					{t("profiles.description")}
				</p>
			</div>

			<div className="grid items-start gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
				<section className="min-w-0 self-start rounded-lg border border-border/70 bg-card/86 shadow-sm backdrop-blur dark:border-white/10 dark:bg-card/64 dark:shadow-none">
					<div className="flex flex-col gap-4 border-b border-border/70 p-4 sm:flex-row sm:items-center sm:justify-between lg:flex-col lg:items-stretch xl:flex-row xl:items-center">
						<div>
							<div className="flex items-center gap-2 text-sm font-semibold">
								<Icon name="User" className="size-4" />
								{t("profiles.listTitle")}
							</div>
						</div>
						<div className="relative sm:w-72 lg:w-full xl:w-72">
							<Icon
								name={searchBusy ? "Spinner" : "MagnifyingGlass"}
								aria-hidden="true"
								data-testid={
									searchBusy ? "profile-search-spinner" : "profile-search-icon"
								}
								className={cn(
									"absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground",
									searchBusy && "animate-spin text-emerald-500",
								)}
							/>
							<Input
								value={query}
								placeholder={t("profiles.searchPlaceholder")}
								className="pl-8"
								onChange={(event) =>
									dispatch({
										type: "query",
										value: event.currentTarget.value,
									})
								}
							/>
						</div>
					</div>

					<div className="grid gap-3 p-4">
						<form
							className="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end"
							onSubmit={createProfile}
						>
							<div className="grid gap-2">
								<Label htmlFor="profile-name">
									{t("profiles.profileName")}
								</Label>
								<Input
									id="profile-name"
									value={profileName}
									placeholder={t("profiles.createPlaceholder")}
									required
									onChange={(event) =>
										dispatch({
											type: "profileName",
											value: event.currentTarget.value,
										})
									}
								/>
							</div>
							<Button
								type="submit"
								disabled={loading || !profileName.trim()}
								className="sm:min-w-28"
							>
								<Icon name={loading ? "Spinner" : "Plus"} className="size-4" />
								{t("common.create")}
							</Button>
						</form>

						{profiles.length === 0 && !query.trim() ? (
							<div className="rounded-lg border border-dashed border-border bg-muted/20 px-4 py-10 text-center">
								<div className="font-medium">{t("profiles.noProfiles")}</div>
								<p className="mt-2 text-sm text-muted-foreground">
									{t("profiles.noProfilesDescription")}
								</p>
							</div>
						) : profiles.length === 0 ? (
							<div className="rounded-lg border border-dashed border-border bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground">
								{t("profiles.noSearchResults")}
							</div>
						) : (
							<div className="overflow-hidden rounded-lg border border-border/70">
								<div className="grid grid-cols-[minmax(0,1fr)_7.5rem] border-b border-border/70 bg-muted/35 px-3 py-2 text-xs font-medium text-muted-foreground">
									<span>{t("profiles.profileName")}</span>
									<span>{t("common.actions")}</span>
								</div>
								<div className="divide-y divide-border/70">
									{profiles.map((profile) => (
										<div
											key={profile.id}
											className={cn(
												"grid grid-cols-[minmax(0,1fr)_7.5rem] items-center gap-3 px-3 py-3 transition-colors hover:bg-accent/35",
												profile.id === selectedUuid && "bg-accent/45",
											)}
										>
											<button
												type="button"
												onClick={() =>
													dispatch({ type: "selectedUuid", value: profile.id })
												}
												className="min-w-0 rounded-md text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/30"
											>
												<div className="flex min-w-0 items-center gap-2">
													<span className="truncate font-medium">
														{profile.name}
													</span>
													{profile.id === selectedUuid ? (
														<Badge variant="outline" className="rounded-md">
															{t("profiles.selected")}
														</Badge>
													) : null}
												</div>
											</button>
											<TooltipProvider delay={0}>
												<div className="flex justify-start gap-1">
													<ProfileRowActionButton
														ariaLabel={t("profiles.manageTexturesForProfile", {
															name: profile.name,
														})}
														icon="FileImage"
														label={t("profiles.manageTexturesAction")}
														testId={`profile-textures-action-${profile.id}`}
														onClick={() => {
															dispatch({
																type: "selectedUuid",
																value: profile.id,
															});
															setTextureManageDialogOpen(true);
														}}
													/>
													<ProfileRowActionButton
														ariaLabel={t("profiles.renameAction", {
															name: profile.name,
														})}
														icon="PencilSimple"
														label={t("profiles.renameShortAction")}
														testId={`profile-rename-action-${profile.id}`}
														onClick={() => openRenameDialog(profile)}
													/>
													<ProfileRowActionButton
														ariaLabel={t("profiles.deleteProfileActionFor", {
															name: profile.name,
														})}
														destructive
														disabled={deletingProfile}
														icon="Trash"
														label={t("profiles.deleteProfileAction")}
														testId={`profile-delete-action-${profile.id}`}
														onClick={() => {
															dispatch({
																type: "selectedUuid",
																value: profile.id,
															});
															setDeleteProfileDialogOpen(true);
														}}
													/>
												</div>
											</TooltipProvider>
										</div>
									))}
								</div>
							</div>
						)}
						<AdminOffsetPagination
							currentPage={Math.floor(profileOffset / profilePageSize) + 1}
							nextDisabled={profileOffset + profilePageSize >= profileTotal}
							onNext={() =>
								setProfileOffset((current) => current + profilePageSize)
							}
							onPageSizeChange={changeProfilePageSize}
							onPrevious={() =>
								setProfileOffset((current) =>
									Math.max(0, current - profilePageSize),
								)
							}
							pageSize={String(profilePageSize)}
							pageSizeOptions={PROFILE_PAGE_SIZE_OPTIONS.map((pageSize) => ({
								label: t("admin.pagination.pageSizeOption", {
									count: pageSize,
								}),
								value: String(pageSize),
							}))}
							prevDisabled={profileOffset === 0}
							total={profileTotal}
							totalPages={Math.max(
								1,
								Math.ceil(profileTotal / profilePageSize),
							)}
						/>
					</div>
				</section>

				<aside className="min-w-0">
					<Suspense fallback={<PreviewSkeleton />}>
						<MinecraftPreview
							label={t("profiles.previewPanelTitle")}
							playerName={selectedProfile?.name}
							skinUrl={skinTexture?.url ?? null}
							capeUrl={capeTexture?.url ?? null}
							model={skinTexture?.texture_model ?? model}
							emptyTitle={t("profiles.previewEmptyTitle")}
							emptyDescription={t("profiles.previewEmptyDescription")}
							failedTitle={t("profiles.previewFailedTitle")}
							failedDescription={t("profiles.previewFailedDescription")}
							noSkinLabel={t("profiles.noSkinTexture")}
							idleLabel={t("profiles.motionIdle")}
							walkLabel={t("profiles.motionWalk")}
							className="w-full"
							frameClassName="lg:h-[42rem]"
						/>
					</Suspense>
				</aside>

				<Dialog
					open={textureManageDialogOpen}
					onOpenChange={setTextureManageDialogOpen}
				>
					<DialogContent keepMounted className="sm:max-w-2xl">
						<DialogHeader>
							<DialogTitle>{t("profiles.textureTitle")}</DialogTitle>
							<DialogDescription>
								{selectedProfile
									? t("profiles.textureManageDialogDescription", {
											name: selectedProfile.name,
										})
									: t("profiles.workbenchEmptyHint")}
							</DialogDescription>
						</DialogHeader>
						<div className="overflow-hidden rounded-lg border border-border/70 bg-muted/12 dark:border-white/10 dark:bg-muted/8">
							<div className="divide-y divide-border/70 dark:divide-white/10">
								<TextureSlotCard
									title={t("home.textureTypeSkin")}
									typeLabel={t("wardrobe.type.skin")}
									texture={skinTexture}
									loading={texturesLoading}
									disabled={!selectedProfile}
									onUpload={() => openTextureDialog("skin")}
									onDelete={() => openDeleteTextureDialog("skin")}
								/>
								<TextureSlotCard
									title={t("home.textureTypeCape")}
									typeLabel={t("wardrobe.type.cape")}
									texture={capeTexture}
									loading={texturesLoading}
									disabled={!selectedProfile}
									onUpload={() => openTextureDialog("cape")}
									onDelete={() => openDeleteTextureDialog("cape")}
								/>
							</div>
						</div>
						<DialogFooter>
							<Button
								type="button"
								variant="outline"
								onClick={() => setTextureManageDialogOpen(false)}
							>
								{t("common.close")}
							</Button>
						</DialogFooter>
					</DialogContent>
				</Dialog>

				<Dialog open={textureDialogOpen} onOpenChange={setTextureDialogOpen}>
					<DialogContent keepMounted className="sm:max-w-lg">
						<TextureUploadForm
							description={
								selectedProfile
									? t("profiles.uploadDialogDescription", {
											name: selectedProfile.name,
											type: t(`wardrobe.type.${uploadTextureType}`),
										})
									: t("profiles.workbenchEmptyHint")
							}
							dragActive={dragActive}
							file={file}
							fileInputId="profile-texture-file"
							model={model}
							submitDisabled={loading || !selectedUuid || !file}
							submitLabel={t("profiles.uploadAndBindAction")}
							submitting={loading}
							submittingLabel={t("profiles.uploadAndBindAction")}
							textureType={uploadTextureType}
							textureTypeLocked
							title={t("profiles.uploadDialogTitle", {
								type: t(`wardrobe.type.${uploadTextureType}`),
							})}
							visibility={visibility}
							onCancel={() => setTextureDialogOpen(false)}
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

				<Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
					<DialogContent keepMounted className="sm:max-w-md">
						<DialogHeader>
							<DialogTitle>
								{t("profiles.deleteDialogTitle", {
									type: t(`wardrobe.type.${textureType}`),
								})}
							</DialogTitle>
							<DialogDescription>
								{selectedProfile
									? t("profiles.deleteDialogDescription", {
											name: selectedProfile.name,
										})
									: t("profiles.workbenchEmptyHint")}
							</DialogDescription>
						</DialogHeader>
						{activeTexture ? (
							<div className="grid gap-2 rounded-lg border border-border/70 bg-muted/20 p-3 text-sm">
								<div className="flex flex-wrap items-center gap-2">
									<Badge variant="secondary" className="rounded-md">
										{t(`wardrobe.type.${activeTexture.texture_type}`)}
									</Badge>
									<Badge variant="outline" className="rounded-md">
										{activeTexture.width}x{activeTexture.height}
									</Badge>
								</div>
								<div className="truncate font-mono text-xs text-muted-foreground">
									{activeTexture.hash}
								</div>
							</div>
						) : null}
						<DialogFooter>
							<Button
								type="button"
								variant="outline"
								disabled={loading}
								onClick={() => setDeleteDialogOpen(false)}
							>
								{t("common.cancel")}
							</Button>
							<Button
								type="button"
								variant="destructive"
								disabled={loading || !selectedUuid || !activeTexture}
								onClick={() => void deleteTexture()}
							>
								<Icon name={loading ? "Spinner" : "Trash"} className="size-4" />
								{t("profiles.unbindTextureAction")}
							</Button>
						</DialogFooter>
					</DialogContent>
				</Dialog>
			</div>

			<Dialog open={renameDialogOpen} onOpenChange={setRenameDialogOpen}>
				<DialogContent className="sm:max-w-md">
					<form className="grid gap-4" onSubmit={renameProfile}>
						<DialogHeader>
							<DialogTitle>{t("profiles.renameTitle")}</DialogTitle>
							<DialogDescription>
								{t("profiles.renameDescription")}
							</DialogDescription>
						</DialogHeader>
						<div className="grid gap-2">
							<Label htmlFor="profile-rename-name">
								{t("profiles.profileName")}
							</Label>
							<Input
								id="profile-rename-name"
								value={renameName}
								required
								autoComplete="off"
								onChange={(event) => setRenameName(event.currentTarget.value)}
							/>
						</div>
						<DialogFooter>
							<Button
								type="button"
								variant="outline"
								disabled={renaming}
								onClick={() => setRenameDialogOpen(false)}
							>
								{t("common.cancel")}
							</Button>
							<Button type="submit" disabled={renaming || !renameName.trim()}>
								{renaming ? (
									<Icon name="Spinner" className="mr-2 size-4 animate-spin" />
								) : (
									<Icon name="PencilSimple" className="mr-2 size-4" />
								)}
								{t("common.save")}
							</Button>
						</DialogFooter>
					</form>
				</DialogContent>
			</Dialog>

			<Dialog
				open={deleteProfileDialogOpen}
				onOpenChange={setDeleteProfileDialogOpen}
			>
				<DialogContent className="sm:max-w-md">
					<DialogHeader>
						<DialogTitle>{t("profiles.deleteProfileTitle")}</DialogTitle>
						<DialogDescription>
							{selectedProfile
								? t("profiles.deleteProfileDescription", {
										name: selectedProfile.name,
									})
								: t("profiles.workbenchEmptyHint")}
						</DialogDescription>
					</DialogHeader>
					{selectedProfile ? (
						<div className="rounded-lg border border-border/70 bg-muted/20 px-3 py-2 text-sm text-muted-foreground">
							{t("profiles.deleteProfileImpact")}
						</div>
					) : null}
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={deletingProfile}
							onClick={() => setDeleteProfileDialogOpen(false)}
						>
							{t("common.cancel")}
						</Button>
						<Button
							type="button"
							variant="destructive"
							disabled={deletingProfile || !selectedProfile}
							onClick={() => void deleteProfile()}
						>
							<Icon
								name={deletingProfile ? "Spinner" : "Trash"}
								className="size-4"
							/>
							{t("profiles.deleteProfileAction")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	);
}

function PreviewSkeleton() {
	return (
		<div className="overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs">
			<div className="flex min-h-12 items-center justify-between border-b border-border/70 px-4">
				<div className="space-y-2">
					<Skeleton className="h-4 w-32" />
					<Skeleton className="h-3 w-20" />
				</div>
				<Skeleton className="size-7 rounded-md" />
			</div>
			<Skeleton className="h-[26rem] rounded-none" />
		</div>
	);
}

function ProfileRowActionButton({
	ariaLabel,
	destructive = false,
	disabled,
	icon,
	label,
	onClick,
	testId,
}: {
	ariaLabel: string;
	destructive?: boolean;
	disabled?: boolean;
	icon: IconName;
	label: string;
	onClick: () => void;
	testId?: string;
}) {
	return (
		<Tooltip>
			<TooltipTrigger
				render={
					<Button
						type="button"
						size="icon"
						variant="ghost"
						aria-label={ariaLabel}
						disabled={disabled}
						data-testid={testId}
						onClick={onClick}
					/>
				}
			>
				<Icon
					name={icon}
					className={cn("size-4", destructive && "text-destructive")}
				/>
			</TooltipTrigger>
			<TooltipContent>{label}</TooltipContent>
		</Tooltip>
	);
}

function TextureSlotCard({
	disabled,
	loading,
	onDelete,
	onUpload,
	texture,
	title,
	typeLabel,
}: {
	disabled: boolean;
	loading: boolean;
	onDelete: () => void;
	onUpload: () => void;
	texture: MinecraftTextureMetadata | null;
	title: string;
	typeLabel: string;
}) {
	const { t } = useTranslation();
	const isDefaultTexture = texture?.source === "default";
	const hasBoundTexture = Boolean(texture && !isDefaultTexture);
	const description = texture
		? isDefaultTexture
			? t("profiles.textureSlotDefault", { type: typeLabel })
			: t("profiles.textureSlotReady", { type: typeLabel })
		: t("profiles.textureSlotEmpty", { type: typeLabel });

	if (loading) {
		return (
			<div className="p-4 text-sm text-muted-foreground">
				<Icon name="Spinner" className="mr-2 inline size-4" />
				{t("profiles.textureMetadataLoading")}
			</div>
		);
	}

	return (
		<div className="p-4">
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0">
					<div className="flex min-w-0 items-center gap-2">
						<Icon name="FileImage" className="size-4 shrink-0 text-primary" />
						<div className="truncate text-sm font-semibold">{title}</div>
					</div>
					<p className="mt-1 text-xs text-muted-foreground">{description}</p>
				</div>
				<Badge
					variant={hasBoundTexture ? "default" : "outline"}
					className="rounded-md"
				>
					{texture ? texture.texture_model : "empty"}
				</Badge>
			</div>

			{texture ? (
				<div className="mt-3 grid gap-1 text-sm text-muted-foreground">
					<div>
						{texture.width} x {texture.height}px ·{" "}
						{formatFileSize(texture.file_size)}
					</div>
					<div className="truncate font-mono text-xs">{texture.hash}</div>
				</div>
			) : null}

			<div className="mt-3 flex flex-wrap gap-2">
				<Button
					type="button"
					size="sm"
					variant={hasBoundTexture ? "outline" : "default"}
					disabled={disabled}
					onClick={onUpload}
				>
					<Icon name="Upload" className="size-4" />
					{hasBoundTexture
						? t("profiles.replaceTextureAction")
						: t("profiles.uploadTextureAction")}
				</Button>
				<Button
					type="button"
					size="sm"
					variant="ghost"
					disabled={disabled || !texture || isDefaultTexture}
					onClick={onDelete}
				>
					<Icon name="Trash" className="size-4" />
					{t("profiles.unbindTextureAction")}
				</Button>
			</div>
		</div>
	);
}

function formatFileSize(bytes: number) {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
