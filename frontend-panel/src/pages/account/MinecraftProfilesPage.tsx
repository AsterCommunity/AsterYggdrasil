import {
	type Dispatch,
	type DragEvent,
	type FormEvent,
	lazy,
	Suspense,
} from "react";
import { useTranslation } from "react-i18next";
import { useMinecraftProfilesPageController } from "@/components/account/profiles-page/useMinecraftProfilesPageController";
import type {
	MinecraftProfilesPageAction,
	MinecraftProfilesPageState,
} from "@/components/account/profiles-page/useMinecraftProfilesPageState";
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
import { cn } from "@/lib/utils";
import type {
	MinecraftTextureMetadata,
	MinecraftTextureType,
	YggdrasilProfile,
} from "@/types/api";

const PROFILE_PAGE_SIZE_OPTIONS = [5, 10] as const;

const MinecraftPreview = lazy(() =>
	import("@/components/yggdrasil/MinecraftPreview").then((module) => ({
		default: module.MinecraftPreview,
	})),
);

export default function MinecraftProfilesPage() {
	const controller = useMinecraftProfilesPageController();
	return <MinecraftProfilesLayout {...controller} />;
}

type ProfilesDispatch = Dispatch<MinecraftProfilesPageAction>;

function MinecraftProfilesLayout({
	activeTexture,
	capeTexture,
	deletingProfile,
	dispatch,
	loading,
	model,
	onChangePageSize,
	onCreateProfile,
	onDeleteProfile,
	onDeleteTexture,
	onDragTextureFile,
	onDropTextureFile,
	onLeaveTextureDropZone,
	onOpenDeleteTextureDialog,
	onOpenRenameDialog,
	onOpenTextureDialog,
	onRenameProfile,
	onSelectTextureFile,
	onUploadTexture,
	profileName,
	profileOffset,
	profilePageSize,
	profiles,
	profileTotal,
	query,
	renameDialogOpen,
	renameName,
	renaming,
	searchBusy,
	selectedProfile,
	selectedUuid,
	skinTexture,
	state,
	texturesLoading,
}: {
	activeTexture: MinecraftTextureMetadata | null;
	capeTexture: MinecraftTextureMetadata | null;
	deletingProfile: boolean;
	dispatch: ProfilesDispatch;
	loading: boolean;
	model: MinecraftProfilesPageState["model"];
	onChangePageSize: (value: string | null) => void;
	onCreateProfile: (event: FormEvent<HTMLFormElement>) => void;
	onDeleteProfile: () => void;
	onDeleteTexture: () => void;
	onDragTextureFile: (event: DragEvent<HTMLLabelElement>) => void;
	onDropTextureFile: (event: DragEvent<HTMLLabelElement>) => void;
	onLeaveTextureDropZone: () => void;
	onOpenDeleteTextureDialog: (type: MinecraftTextureType) => void;
	onOpenRenameDialog: (profile: { id: string; name: string }) => void;
	onOpenTextureDialog: (type: MinecraftTextureType) => void;
	onRenameProfile: (event: FormEvent<HTMLFormElement>) => void;
	onSelectTextureFile: (file: File | null) => void;
	onUploadTexture: (event: FormEvent<HTMLFormElement>) => void;
	profileName: string;
	profileOffset: number;
	profilePageSize: number;
	profiles: YggdrasilProfile[];
	profileTotal: number;
	query: string;
	renameDialogOpen: boolean;
	renameName: string;
	renaming: boolean;
	searchBusy: boolean;
	selectedProfile: YggdrasilProfile | null;
	selectedUuid: string;
	skinTexture: MinecraftTextureMetadata | null;
	state: MinecraftProfilesPageState;
	texturesLoading: boolean;
}) {
	return (
		<div className="mx-auto w-full max-w-[96rem] px-4 py-5 sm:px-6 lg:px-7">
			<ProfilesPageHeader />
			<div className="grid items-start gap-5 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
				<ProfileListSection
					deletingProfile={deletingProfile}
					dispatch={dispatch}
					loading={loading}
					profileName={profileName}
					profileOffset={profileOffset}
					profilePageSize={profilePageSize}
					profileTotal={profileTotal}
					profiles={profiles}
					query={query}
					searchBusy={searchBusy}
					selectedUuid={selectedUuid}
					onChangePageSize={onChangePageSize}
					onCreateProfile={onCreateProfile}
					onOpenRenameDialog={onOpenRenameDialog}
				/>
				<ProfilePreviewPanel
					capeTexture={capeTexture}
					model={model}
					selectedProfile={selectedProfile}
					skinTexture={skinTexture}
				/>
			</div>
			<ProfileTextureDialogs
				activeTexture={activeTexture}
				capeTexture={capeTexture}
				dispatch={dispatch}
				loading={loading}
				model={model}
				selectedProfile={selectedProfile}
				selectedUuid={selectedUuid}
				skinTexture={skinTexture}
				state={state}
				texturesLoading={texturesLoading}
				onDeleteTexture={onDeleteTexture}
				onDragTextureFile={onDragTextureFile}
				onDropTextureFile={onDropTextureFile}
				onLeaveTextureDropZone={onLeaveTextureDropZone}
				onOpenDeleteTextureDialog={onOpenDeleteTextureDialog}
				onOpenTextureDialog={onOpenTextureDialog}
				onSelectTextureFile={onSelectTextureFile}
				onUploadTexture={onUploadTexture}
			/>
			<ProfileRenameDialog
				dispatch={dispatch}
				open={renameDialogOpen}
				renameName={renameName}
				renaming={renaming}
				onSubmit={onRenameProfile}
			/>
			<ProfileDeleteDialog
				deletingProfile={deletingProfile}
				dispatch={dispatch}
				open={state.deleteProfileDialogOpen}
				selectedProfile={selectedProfile}
				onDeleteProfile={onDeleteProfile}
			/>
		</div>
	);
}

function ProfilesPageHeader() {
	const { t } = useTranslation();

	return (
		<div className="mb-5 border-b border-border/70 pb-5 dark:border-white/10">
			<h1 className="text-2xl font-semibold tracking-normal text-foreground sm:text-3xl">
				{t("profiles.title")}
			</h1>
			<p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
				{t("profiles.description")}
			</p>
		</div>
	);
}

function ProfileListSection({
	deletingProfile,
	dispatch,
	loading,
	onChangePageSize,
	onCreateProfile,
	onOpenRenameDialog,
	profileName,
	profileOffset,
	profilePageSize,
	profiles,
	profileTotal,
	query,
	searchBusy,
	selectedUuid,
}: {
	deletingProfile: boolean;
	dispatch: ProfilesDispatch;
	loading: boolean;
	onChangePageSize: (value: string | null) => void;
	onCreateProfile: (event: FormEvent<HTMLFormElement>) => void;
	onOpenRenameDialog: (profile: { id: string; name: string }) => void;
	profileName: string;
	profileOffset: number;
	profilePageSize: number;
	profiles: YggdrasilProfile[];
	profileTotal: number;
	query: string;
	searchBusy: boolean;
	selectedUuid: string;
}) {
	const { t } = useTranslation();

	return (
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
				<ProfileCreateForm
					loading={loading}
					profileName={profileName}
					dispatch={dispatch}
					onSubmit={onCreateProfile}
				/>
				<ProfileList
					deletingProfile={deletingProfile}
					dispatch={dispatch}
					profiles={profiles}
					query={query}
					selectedUuid={selectedUuid}
					onOpenRenameDialog={onOpenRenameDialog}
				/>
				<AdminOffsetPagination
					currentPage={Math.floor(profileOffset / profilePageSize) + 1}
					nextDisabled={profileOffset + profilePageSize >= profileTotal}
					onNext={() =>
						dispatch({
							type: "profileOffset",
							value: (current) => current + profilePageSize,
						})
					}
					onPageSizeChange={onChangePageSize}
					onPrevious={() =>
						dispatch({
							type: "profileOffset",
							value: (current) => Math.max(0, current - profilePageSize),
						})
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
					totalPages={Math.max(1, Math.ceil(profileTotal / profilePageSize))}
				/>
			</div>
		</section>
	);
}

function ProfileCreateForm({
	dispatch,
	loading,
	onSubmit,
	profileName,
}: {
	dispatch: ProfilesDispatch;
	loading: boolean;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	profileName: string;
}) {
	const { t } = useTranslation();

	return (
		<form
			className="grid min-w-0 gap-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end"
			onSubmit={onSubmit}
		>
			<div className="grid gap-2">
				<Label htmlFor="profile-name">{t("profiles.profileName")}</Label>
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
	);
}

function ProfileList({
	deletingProfile,
	dispatch,
	onOpenRenameDialog,
	profiles,
	query,
	selectedUuid,
}: {
	deletingProfile: boolean;
	dispatch: ProfilesDispatch;
	onOpenRenameDialog: (profile: { id: string; name: string }) => void;
	profiles: YggdrasilProfile[];
	query: string;
	selectedUuid: string;
}) {
	const { t } = useTranslation();

	if (profiles.length === 0 && !query.trim()) {
		return (
			<div className="rounded-lg border border-dashed border-border bg-muted/20 px-4 py-10 text-center">
				<div className="font-medium">{t("profiles.noProfiles")}</div>
				<p className="mt-2 text-sm text-muted-foreground">
					{t("profiles.noProfilesDescription")}
				</p>
			</div>
		);
	}

	if (profiles.length === 0) {
		return (
			<div className="rounded-lg border border-dashed border-border bg-muted/20 px-4 py-8 text-center text-sm text-muted-foreground">
				{t("profiles.noSearchResults")}
			</div>
		);
	}

	return (
		<div className="overflow-hidden rounded-lg border border-border/70">
			<div className="grid grid-cols-[minmax(0,1fr)_7.5rem] border-b border-border/70 bg-muted/35 px-3 py-2 text-xs font-medium text-muted-foreground">
				<span>{t("profiles.profileName")}</span>
				<span>{t("common.actions")}</span>
			</div>
			<div className="divide-y divide-border/70">
				{profiles.map((profile) => (
					<ProfileListRow
						key={profile.id}
						deletingProfile={deletingProfile}
						dispatch={dispatch}
						profile={profile}
						selected={profile.id === selectedUuid}
						onOpenRenameDialog={onOpenRenameDialog}
					/>
				))}
			</div>
		</div>
	);
}

function ProfileListRow({
	deletingProfile,
	dispatch,
	onOpenRenameDialog,
	profile,
	selected,
}: {
	deletingProfile: boolean;
	dispatch: ProfilesDispatch;
	onOpenRenameDialog: (profile: { id: string; name: string }) => void;
	profile: YggdrasilProfile;
	selected: boolean;
}) {
	const { t } = useTranslation();

	return (
		<div
			className={cn(
				"grid grid-cols-[minmax(0,1fr)_7.5rem] items-center gap-3 px-3 py-3 transition-colors hover:bg-accent/35",
				selected && "bg-accent/45",
			)}
		>
			<button
				type="button"
				onClick={() => dispatch({ type: "selectedUuid", value: profile.id })}
				className="min-w-0 rounded-md text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/30"
			>
				<div className="flex min-w-0 items-center gap-2">
					<span className="truncate font-medium">{profile.name}</span>
					{selected ? (
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
							dispatch({ type: "selectedUuid", value: profile.id });
							dispatch({ type: "textureManageDialogOpen", value: true });
						}}
					/>
					<ProfileRowActionButton
						ariaLabel={t("profiles.renameAction", {
							name: profile.name,
						})}
						icon="PencilSimple"
						label={t("profiles.renameShortAction")}
						testId={`profile-rename-action-${profile.id}`}
						onClick={() => onOpenRenameDialog(profile)}
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
							dispatch({ type: "selectedUuid", value: profile.id });
							dispatch({ type: "deleteProfileDialogOpen", value: true });
						}}
					/>
				</div>
			</TooltipProvider>
		</div>
	);
}

function ProfilePreviewPanel({
	capeTexture,
	model,
	selectedProfile,
	skinTexture,
}: {
	capeTexture: MinecraftTextureMetadata | null;
	model: MinecraftProfilesPageState["model"];
	selectedProfile: YggdrasilProfile | null;
	skinTexture: MinecraftTextureMetadata | null;
}) {
	const { t } = useTranslation();

	return (
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
	);
}

function ProfileTextureDialogs({
	activeTexture,
	capeTexture,
	dispatch,
	loading,
	model,
	onDeleteTexture,
	onDragTextureFile,
	onDropTextureFile,
	onLeaveTextureDropZone,
	onOpenDeleteTextureDialog,
	onOpenTextureDialog,
	onSelectTextureFile,
	onUploadTexture,
	selectedProfile,
	selectedUuid,
	skinTexture,
	state,
	texturesLoading,
}: {
	activeTexture: MinecraftTextureMetadata | null;
	capeTexture: MinecraftTextureMetadata | null;
	dispatch: ProfilesDispatch;
	loading: boolean;
	model: MinecraftProfilesPageState["model"];
	onDeleteTexture: () => void;
	onDragTextureFile: (event: DragEvent<HTMLLabelElement>) => void;
	onDropTextureFile: (event: DragEvent<HTMLLabelElement>) => void;
	onLeaveTextureDropZone: () => void;
	onOpenDeleteTextureDialog: (type: MinecraftTextureType) => void;
	onOpenTextureDialog: (type: MinecraftTextureType) => void;
	onSelectTextureFile: (file: File | null) => void;
	onUploadTexture: (event: FormEvent<HTMLFormElement>) => void;
	selectedProfile: YggdrasilProfile | null;
	selectedUuid: string;
	skinTexture: MinecraftTextureMetadata | null;
	state: MinecraftProfilesPageState;
	texturesLoading: boolean;
}) {
	const { t } = useTranslation();

	return (
		<>
			<Dialog
				open={state.textureManageDialogOpen}
				onOpenChange={(open) =>
					dispatch({ type: "textureManageDialogOpen", value: open })
				}
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
								onUpload={() => onOpenTextureDialog("skin")}
								onDelete={() => onOpenDeleteTextureDialog("skin")}
							/>
							<TextureSlotCard
								title={t("home.textureTypeCape")}
								typeLabel={t("wardrobe.type.cape")}
								texture={capeTexture}
								loading={texturesLoading}
								disabled={!selectedProfile}
								onUpload={() => onOpenTextureDialog("cape")}
								onDelete={() => onOpenDeleteTextureDialog("cape")}
							/>
						</div>
					</div>
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							onClick={() =>
								dispatch({ type: "textureManageDialogOpen", value: false })
							}
						>
							{t("common.close")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>

			<Dialog
				open={state.textureDialogOpen}
				onOpenChange={(open) =>
					dispatch({ type: "textureDialogOpen", value: open })
				}
			>
				<DialogContent keepMounted className="sm:max-w-lg">
					<TextureUploadForm
						description={
							selectedProfile
								? t("profiles.uploadDialogDescription", {
										name: selectedProfile.name,
										type: t(`wardrobe.type.${state.uploadTextureType}`),
									})
								: t("profiles.workbenchEmptyHint")
						}
						dragActive={state.dragActive}
						file={state.file}
						fileInputId="profile-texture-file"
						model={model}
						submitDisabled={loading || !selectedUuid || !state.file}
						submitLabel={t("profiles.uploadAndBindAction")}
						submitting={loading}
						submittingLabel={t("profiles.uploadAndBindAction")}
						textureType={state.uploadTextureType}
						textureTypeLocked
						title={t("profiles.uploadDialogTitle", {
							type: t(`wardrobe.type.${state.uploadTextureType}`),
						})}
						visibility={state.visibility}
						onCancel={() =>
							dispatch({ type: "textureDialogOpen", value: false })
						}
						onDragEnter={onDragTextureFile}
						onDragLeave={onLeaveTextureDropZone}
						onDrop={onDropTextureFile}
						onFileChange={onSelectTextureFile}
						onModelChange={(nextModel) =>
							dispatch({ type: "model", value: nextModel })
						}
						onSubmit={onUploadTexture}
						onTextureTypeChange={(nextType) => {
							dispatch({ type: "textureType", value: nextType });
							dispatch({ type: "file", value: null });
							dispatch({ type: "dragActive", value: false });
						}}
						onVisibilityChange={(nextVisibility) =>
							dispatch({ type: "visibility", value: nextVisibility })
						}
					/>
				</DialogContent>
			</Dialog>

			<Dialog
				open={state.deleteDialogOpen}
				onOpenChange={(open) =>
					dispatch({ type: "deleteDialogOpen", value: open })
				}
			>
				<DialogContent keepMounted className="sm:max-w-md">
					<DialogHeader>
						<DialogTitle>
							{t("profiles.deleteDialogTitle", {
								type: t(`wardrobe.type.${state.textureType}`),
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
						<TextureDeleteSummary texture={activeTexture} />
					) : null}
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={loading}
							onClick={() =>
								dispatch({ type: "deleteDialogOpen", value: false })
							}
						>
							{t("common.cancel")}
						</Button>
						<Button
							type="button"
							variant="destructive"
							disabled={loading || !selectedUuid || !activeTexture}
							onClick={onDeleteTexture}
						>
							<Icon name={loading ? "Spinner" : "Trash"} className="size-4" />
							{t("profiles.unbindTextureAction")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</>
	);
}

function TextureDeleteSummary({
	texture,
}: {
	texture: MinecraftTextureMetadata;
}) {
	const { t } = useTranslation();

	return (
		<div className="grid gap-2 rounded-lg border border-border/70 bg-muted/20 p-3 text-sm">
			<div className="flex flex-wrap items-center gap-2">
				<Badge variant="secondary" className="rounded-md">
					{t(`wardrobe.type.${texture.texture_type}`)}
				</Badge>
				<Badge variant="outline" className="rounded-md">
					{texture.width}x{texture.height}
				</Badge>
			</div>
			<div className="truncate font-mono text-xs text-muted-foreground">
				{texture.hash}
			</div>
		</div>
	);
}

function ProfileRenameDialog({
	dispatch,
	onSubmit,
	open,
	renameName,
	renaming,
}: {
	dispatch: ProfilesDispatch;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	open: boolean;
	renameName: string;
	renaming: boolean;
}) {
	const { t } = useTranslation();

	return (
		<Dialog
			open={open}
			onOpenChange={(nextOpen) =>
				dispatch({ type: "renameDialogOpen", value: nextOpen })
			}
		>
			<DialogContent className="sm:max-w-md">
				<form className="grid gap-4" onSubmit={onSubmit}>
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
							onChange={(event) =>
								dispatch({
									type: "renameName",
									value: event.currentTarget.value,
								})
							}
						/>
					</div>
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={renaming}
							onClick={() =>
								dispatch({ type: "renameDialogOpen", value: false })
							}
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
	);
}

function ProfileDeleteDialog({
	deletingProfile,
	dispatch,
	onDeleteProfile,
	open,
	selectedProfile,
}: {
	deletingProfile: boolean;
	dispatch: ProfilesDispatch;
	onDeleteProfile: () => void;
	open: boolean;
	selectedProfile: YggdrasilProfile | null;
}) {
	const { t } = useTranslation();

	return (
		<Dialog
			open={open}
			onOpenChange={(nextOpen) =>
				dispatch({ type: "deleteProfileDialogOpen", value: nextOpen })
			}
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
						onClick={() =>
							dispatch({ type: "deleteProfileDialogOpen", value: false })
						}
					>
						{t("common.cancel")}
					</Button>
					<Button
						type="button"
						variant="destructive"
						disabled={deletingProfile || !selectedProfile}
						onClick={onDeleteProfile}
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
