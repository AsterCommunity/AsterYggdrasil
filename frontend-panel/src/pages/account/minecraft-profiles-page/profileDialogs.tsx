import { useTranslation } from "react-i18next";
import type { MinecraftProfilesPageController } from "@/components/account/profiles-page/useMinecraftProfilesPageController";
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
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { TextureUploadForm } from "@/components/yggdrasil/TextureUploadForm";
import type { MinecraftTextureMetadata } from "@/types/api";

export function ProfileTextureDialogs({
	controller,
}: {
	controller: MinecraftProfilesPageController;
}) {
	const { t } = useTranslation();
	const {
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
	} = controller;

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
			<div className="p-4" role="status">
				<span className="sr-only">{t("profiles.textureMetadataLoading")}</span>
				<div className="space-y-3">
					<Skeleton className="h-4 w-1/2" />
					<Skeleton className="h-3 w-2/3" />
					<Skeleton className="h-16 w-full rounded-lg" />
				</div>
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

export function ProfileRenameDialog({
	controller,
}: {
	controller: MinecraftProfilesPageController;
}) {
	const { t } = useTranslation();
	const {
		dispatch,
		onRenameProfile: onSubmit,
		renameDialogOpen: open,
		renameName,
		renaming,
	} = controller;

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

export function ProfileDeleteDialog({
	controller,
}: {
	controller: MinecraftProfilesPageController;
}) {
	const { t } = useTranslation();
	const { deletingProfile, dispatch, onDeleteProfile, selectedProfile, state } =
		controller;

	return (
		<Dialog
			open={state.deleteProfileDialogOpen}
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
