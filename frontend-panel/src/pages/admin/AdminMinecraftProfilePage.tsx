import {
	type FormEvent,
	useCallback,
	useEffect,
	useMemo,
	useReducer,
} from "react";
import { useTranslation } from "react-i18next";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { InfoTile } from "@/components/admin/admin-minecraft-profile-page/InfoTile";
import { ProfileTextureList } from "@/components/admin/admin-minecraft-profile-page/ProfileTextureList";
import type {
	AdminMinecraftProfileInfo,
	MinecraftTextureMetadata,
} from "@/components/admin/admin-minecraft-profile-page/types";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { DateTimeText } from "@/components/common/DateTimeText";
import { RefreshButton } from "@/components/common/RefreshButton";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
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
import { CopyField } from "@/components/yggdrasil/CopyField";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import { logger } from "@/lib/logger";
import { getUserDisplayName } from "@/lib/user";
import { adminPaths, adminUserPath } from "@/routes/routePaths";
import {
	adminMinecraftProfileService,
	adminUserService,
} from "@/services/adminService";
import type { AdminUserInfo } from "@/types/api";

function parseUuid(value: string | undefined) {
	if (!value) return null;
	return value.trim() || null;
}

function readReturnTo(value: unknown) {
	if (
		!value ||
		typeof value !== "object" ||
		!("returnTo" in value) ||
		typeof value.returnTo !== "string"
	) {
		return null;
	}

	const returnTo = value.returnTo.trim();
	return /^\/admin\/users\/[^/?#]+$/.test(returnTo) ? returnTo : null;
}

type PageState = {
	ownerUser: AdminUserInfo | null;
	profile: AdminMinecraftProfileInfo | null;
	textures: MinecraftTextureMetadata[];
	loading: boolean;
	deleteDialogOpen: boolean;
	textureToDelete: MinecraftTextureMetadata | null;
	deletingProfile: boolean;
	deletingTexture: boolean;
	renameDialogOpen: boolean;
	renameName: string;
	renamingProfile: boolean;
};

type PageAction =
	| { type: "loadStart" }
	| {
			type: "loadSuccess";
			ownerUser: AdminUserInfo | null;
			profile: AdminMinecraftProfileInfo;
			textures: MinecraftTextureMetadata[];
	  }
	| { type: "loadError" }
	| { type: "setLoading"; value: boolean }
	| { type: "deleteDialogOpen"; value: boolean }
	| { type: "textureToDelete"; value: MinecraftTextureMetadata | null }
	| { type: "deletingProfile"; value: boolean }
	| { type: "deletingTexture"; value: boolean }
	| { type: "renameDialogOpen"; value: boolean }
	| { type: "renameName"; value: string }
	| { type: "renamingProfile"; value: boolean };

const initialPageState: PageState = {
	ownerUser: null,
	profile: null,
	textures: [],
	loading: true,
	deleteDialogOpen: false,
	textureToDelete: null,
	deletingProfile: false,
	deletingTexture: false,
	renameDialogOpen: false,
	renameName: "",
	renamingProfile: false,
};

function pageReducer(state: PageState, action: PageAction): PageState {
	switch (action.type) {
		case "loadStart":
			return { ...state, loading: true };
		case "loadSuccess":
			return {
				...state,
				ownerUser: action.ownerUser,
				profile: action.profile,
				textures: action.textures,
				loading: false,
			};
		case "loadError":
			return {
				...state,
				ownerUser: null,
				profile: null,
				textures: [],
				loading: false,
			};
		case "setLoading":
			return { ...state, loading: action.value };
		case "deleteDialogOpen":
			return { ...state, deleteDialogOpen: action.value };
		case "textureToDelete":
			return { ...state, textureToDelete: action.value };
		case "deletingProfile":
			return { ...state, deletingProfile: action.value };
		case "deletingTexture":
			return { ...state, deletingTexture: action.value };
		case "renameDialogOpen":
			return { ...state, renameDialogOpen: action.value };
		case "renameName":
			return { ...state, renameName: action.value };
		case "renamingProfile":
			return { ...state, renamingProfile: action.value };
	}
}

export default function AdminMinecraftProfilePage() {
	const { t } = useTranslation();
	const location = useLocation();
	const navigate = useNavigate();
	const params = useParams();
	const uuid = parseUuid(params.uuid);
	const returnTo = readReturnTo(location.state);
	const backPath = returnTo ?? adminPaths.users;
	const [state, dispatch] = useReducer(pageReducer, initialPageState);
	const {
		deleteDialogOpen,
		deletingProfile,
		deletingTexture,
		loading,
		ownerUser,
		profile,
		renameDialogOpen,
		renameName,
		renamingProfile,
		textureToDelete,
		textures,
	} = state;

	usePageTitle(profile?.name ?? t("admin.minecraftProfilePage.title"));

	const load = useCallback(async () => {
		if (!uuid) {
			dispatch({ type: "setLoading", value: false });
			return;
		}
		try {
			dispatch({ type: "loadStart" });
			const nextProfile = (await adminMinecraftProfileService.get(
				uuid,
			)) as AdminMinecraftProfileInfo;
			const [nextTextures, nextOwnerUser] = await Promise.all([
				adminMinecraftProfileService.listTextures(uuid),
				adminUserService.get(nextProfile.user_id).catch((error) => {
					logger.warn("Failed to load Minecraft profile owner user", error);
					return null;
				}),
			]);
			dispatch({
				type: "loadSuccess",
				ownerUser: nextOwnerUser as AdminUserInfo | null,
				profile: nextProfile,
				textures: nextTextures as MinecraftTextureMetadata[],
			});
		} catch (error) {
			handleApiError(error);
			dispatch({ type: "loadError" });
		}
	}, [uuid]);

	useEffect(() => {
		void load();
	}, [load]);

	const skinTexture =
		textures.find((texture) => texture.texture_type === "skin") ?? null;
	const capeTexture =
		textures.find((texture) => texture.texture_type === "cape") ?? null;

	const handleDeleteProfile = async () => {
		if (!uuid) return;
		dispatch({ type: "deletingProfile", value: true });
		try {
			await adminMinecraftProfileService.delete(uuid);
			toast.success(t("admin.minecraftProfilePage.deleted"));
			navigate(backPath);
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "deletingProfile", value: false });
			dispatch({ type: "deleteDialogOpen", value: false });
		}
	};

	const openRenameDialog = () => {
		if (!profile) return;
		dispatch({ type: "renameName", value: profile.name });
		dispatch({ type: "renameDialogOpen", value: true });
	};

	const handleRenameProfile = async (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (!uuid || !renameName.trim()) return;
		dispatch({ type: "renamingProfile", value: true });
		try {
			const renamed = await adminMinecraftProfileService.rename(uuid, {
				name: renameName.trim(),
			});
			dispatch({
				type: "loadSuccess",
				ownerUser,
				profile: renamed as AdminMinecraftProfileInfo,
				textures,
			});
			dispatch({ type: "renameDialogOpen", value: false });
			dispatch({ type: "renameName", value: "" });
			toast.success(t("admin.minecraftProfilePage.renameSuccess"));
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "renamingProfile", value: false });
		}
	};

	const handleDeleteTexture = async () => {
		if (!uuid || !textureToDelete) return;
		dispatch({ type: "deletingTexture", value: true });
		try {
			await adminMinecraftProfileService.deleteTexture(
				uuid,
				textureToDelete.texture_type,
			);
			toast.success(
				t("admin.minecraftProfilePage.textureDeleted", {
					textureType: textureToDelete.texture_type.toUpperCase(),
				}),
			);
			dispatch({ type: "textureToDelete", value: null });
			await load();
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "deletingTexture", value: false });
		}
	};

	const previewModel =
		skinTexture?.texture_model ?? profile?.texture_model ?? "default";
	const headerActions = useMemo(
		() => (
			<>
				<Button
					type="button"
					variant="outline"
					size="sm"
					render={<Link to={backPath} />}
				>
					<Icon name="ArrowLeft" className="mr-2 size-4" />
					{t(
						returnTo
							? "admin.minecraftProfilePage.backToOwnerUser"
							: "admin.minecraftProfilePage.backToUsers",
					)}
				</Button>
				<RefreshButton loading={loading} onClick={() => void load()} />
			</>
		),
		[backPath, load, loading, returnTo, t],
	);

	if (!uuid) {
		return (
			<AdminPageShell>
				<AdminSurface>
					<div className="space-y-2">
						<h1 className="text-lg font-semibold">
							{t("admin.minecraftProfilePage.title")}
						</h1>
						<p className="text-sm text-muted-foreground">
							{t("admin.minecraftProfilePage.missingUuid")}
						</p>
					</div>
				</AdminSurface>
			</AdminPageShell>
		);
	}

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={profile?.name ?? t("admin.minecraftProfilePage.title")}
				description={t("admin.minecraftProfilePage.description")}
				actions={headerActions}
			/>

			<div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.72fr)]">
				<AdminSurface padded={false} className="min-w-0 overflow-hidden">
					<ProfileIdentityHeader
						profile={profile}
						uuid={uuid}
						onRename={openRenameDialog}
					/>
					<div className="grid gap-5 p-4 sm:p-5">
						<ProfileRecordSection
							ownerUser={ownerUser}
							profile={profile}
							uuid={uuid}
						/>
						<ProfileTextureList
							deletingTexture={deletingTexture}
							loading={loading}
							textures={textures}
							onRefresh={() => void load()}
							onSelectDelete={(texture) =>
								dispatch({ type: "textureToDelete", value: texture })
							}
						/>
					</div>
				</AdminSurface>

				<ProfilePreviewSidebar
					capeTexture={capeTexture}
					deletingProfile={deletingProfile}
					model={previewModel}
					profile={profile}
					skinTexture={skinTexture}
					onDelete={() => dispatch({ type: "deleteDialogOpen", value: true })}
				/>
			</div>

			<ConfirmDialog
				open={deleteDialogOpen}
				onOpenChange={(open) =>
					dispatch({ type: "deleteDialogOpen", value: open })
				}
				title={t("admin.minecraftProfilePage.deleteTitle")}
				description={t("admin.minecraftProfilePage.deleteDescription")}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("common.delete")}
				variant="destructive"
				loading={deletingProfile}
				onConfirm={() => void handleDeleteProfile()}
			/>

			<Dialog
				open={renameDialogOpen}
				onOpenChange={(open) =>
					dispatch({ type: "renameDialogOpen", value: open })
				}
			>
				<DialogContent className="sm:max-w-md">
					<form className="grid gap-4" onSubmit={handleRenameProfile}>
						<DialogHeader>
							<DialogTitle>
								{t("admin.minecraftProfilePage.renameTitle")}
							</DialogTitle>
							<DialogDescription>
								{t("admin.minecraftProfilePage.renameDescription")}
							</DialogDescription>
						</DialogHeader>
						<div className="grid gap-2">
							<Label htmlFor="admin-profile-rename-name">
								{t("admin.minecraftProfilePage.profileName")}
							</Label>
							<Input
								id="admin-profile-rename-name"
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
								disabled={renamingProfile}
								onClick={() =>
									dispatch({ type: "renameDialogOpen", value: false })
								}
							>
								{t("common.cancel")}
							</Button>
							<Button
								type="submit"
								disabled={renamingProfile || !renameName.trim()}
							>
								{renamingProfile ? (
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

			<ConfirmDialog
				open={textureToDelete != null}
				onOpenChange={(open) => {
					if (!open) dispatch({ type: "textureToDelete", value: null });
				}}
				title={t("admin.minecraftProfilePage.deleteTextureTitle")}
				description={t("admin.minecraftProfilePage.deleteTextureDescription", {
					textureType: textureToDelete?.texture_type.toUpperCase() ?? "",
				})}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("common.delete")}
				variant="destructive"
				loading={deletingTexture}
				onConfirm={() => void handleDeleteTexture()}
			/>
		</AdminPageShell>
	);
}

function ProfilePreviewSidebar({
	capeTexture,
	deletingProfile,
	model,
	onDelete,
	profile,
	skinTexture,
}: {
	capeTexture: MinecraftTextureMetadata | null;
	deletingProfile: boolean;
	model: "default" | "slim";
	onDelete: () => void;
	profile: AdminMinecraftProfileInfo | null;
	skinTexture: MinecraftTextureMetadata | null;
}) {
	const { t } = useTranslation();

	return (
		<aside className="grid min-w-0 max-w-full gap-3 xl:sticky xl:top-20 xl:self-start">
			<MinecraftPreviewPanel
				label={t("admin.minecraftProfilePage.preview")}
				playerName={profile?.name}
				skinUrl={skinTexture?.url ?? null}
				capeUrl={capeTexture?.url ?? null}
				model={model}
				emptyTitle={t("profiles.previewEmptyTitle")}
				emptyDescription={t("profiles.previewEmptyDescription")}
				failedTitle={t("profiles.previewFailedTitle")}
				failedDescription={t("profiles.previewFailedDescription")}
				noSkinLabel={t("profiles.noSkinTexture")}
				idleLabel={t("profiles.motionIdle")}
				walkLabel={t("profiles.motionWalk")}
				frameClassName="h-[34rem]"
				skeletonClassName="h-[38rem]"
			/>
			<div className="grid gap-3 rounded-lg border border-border/70 bg-card/95 p-4 shadow-xs dark:border-white/10">
				<div className="rounded-lg border border-border/70 bg-muted/20 p-3 text-sm dark:border-white/10">
					<div className="flex min-w-0 flex-wrap items-center gap-2">
						<Badge variant="secondary" className="rounded-md">
							{model}
						</Badge>
						<Badge variant="outline" className="rounded-md">
							{t("admin.minecraftProfilePage.skin")}:{" "}
							{skinTexture
								? t("admin.minecraftProfilePage.boundTexture")
								: t("admin.minecraftProfilePage.noTextureSlot")}
						</Badge>
						<Badge variant="outline" className="rounded-md">
							{t("admin.minecraftProfilePage.cape")}:{" "}
							{capeTexture
								? t("admin.minecraftProfilePage.boundTexture")
								: t("admin.minecraftProfilePage.noTextureSlot")}
						</Badge>
					</div>
				</div>
				<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-end">
					<Button
						type="button"
						variant="destructive"
						className="w-full sm:w-auto"
						disabled={!profile || deletingProfile}
						onClick={onDelete}
					>
						<Icon
							name={deletingProfile ? "Spinner" : "Trash"}
							className={deletingProfile ? "size-4 animate-spin" : "size-4"}
						/>
						{t("admin.minecraftProfilePage.deleteProfileAction")}
					</Button>
				</div>
			</div>
		</aside>
	);
}

function ProfileIdentityHeader({
	onRename,
	profile,
	uuid,
}: {
	onRename: () => void;
	profile: AdminMinecraftProfileInfo | null;
	uuid: string;
}) {
	const { t } = useTranslation();
	return (
		<div className="border-b border-border/70 bg-muted/16 px-4 py-4 dark:border-white/10 dark:bg-white/4 sm:px-5">
			<div className="flex min-w-0 flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
				<div className="min-w-0 max-w-full">
					<div className="flex min-w-0 items-center gap-2">
						<Icon name="User" className="size-5 shrink-0 text-primary" />
						<h2 className="break-words text-lg font-semibold text-foreground">
							{profile?.name ?? t("admin.minecraftProfilePage.title")}
						</h2>
					</div>
					<p className="mt-1 break-all font-mono text-xs text-muted-foreground">
						{profile?.uuid ?? uuid}
					</p>
				</div>
				<Button
					type="button"
					variant="outline"
					className="w-full sm:w-auto"
					disabled={!profile}
					onClick={onRename}
				>
					<Icon name="PencilSimple" className="size-4" />
					{t("admin.minecraftProfilePage.renameAction")}
				</Button>
			</div>
		</div>
	);
}

function ProfileRecordSection({
	ownerUser,
	profile,
	uuid,
}: {
	ownerUser: AdminUserInfo | null;
	profile: AdminMinecraftProfileInfo | null;
	uuid: string;
}) {
	const { t } = useTranslation();
	return (
		<section className="grid gap-3">
			<div>
				<h3 className="text-base font-semibold text-foreground">
					{t("admin.minecraftProfilePage.recordTitle")}
				</h3>
				<p className="mt-1 text-sm leading-6 text-muted-foreground">
					{t("admin.minecraftProfilePage.recordDescription")}
				</p>
			</div>
			<div className="grid min-w-0 gap-3 md:grid-cols-2">
				<CopyField
					label={t("admin.minecraftProfilePage.profileUuid")}
					value={profile?.uuid ?? uuid}
					compact
				/>
				<CopyField
					label={t("admin.minecraftProfilePage.profileName")}
					value={profile?.name ?? "-"}
					compact
				/>
			</div>
			<div className="grid min-w-0 gap-3 lg:grid-cols-[minmax(0,1.45fr)_repeat(3,minmax(0,1fr))]">
				<OwnerUserTile ownerUser={ownerUser} userId={profile?.user_id} />
				<InfoTile
					label={t("admin.minecraftProfilePage.profileId")}
					value={profile?.id?.toString() ?? "-"}
					mono
				/>
				<InfoTile
					label={t("admin.minecraftProfilePage.createdAt")}
					value={<DateTimeText value={profile?.created_at} />}
				/>
				<InfoTile
					label={t("admin.minecraftProfilePage.updatedAt")}
					value={<DateTimeText value={profile?.updated_at} />}
				/>
			</div>
		</section>
	);
}

function OwnerUserTile({
	ownerUser,
	userId,
}: {
	ownerUser: AdminUserInfo | null;
	userId: number | null | undefined;
}) {
	const { t } = useTranslation();
	const displayName = ownerUser ? getUserDisplayName(ownerUser) : null;

	return (
		<div className="min-w-0 rounded-lg border border-border/70 bg-background/60 p-3">
			<p className="text-xs uppercase tracking-wide text-muted-foreground">
				{t("admin.minecraftProfilePage.userId")}
			</p>
			{ownerUser ? (
				<Link
					to={adminUserPath(ownerUser.id)}
					className="mt-2 flex min-w-0 items-center gap-3 rounded-md outline-none transition-colors hover:bg-muted/40 focus-visible:ring-3 focus-visible:ring-ring/35"
				>
					<UserAvatarImage
						avatar={ownerUser.profile.avatar}
						name={displayName ?? ownerUser.username}
						alt=""
						size="sm"
						className="rounded-lg"
					/>
					<div className="min-w-0">
						<div className="truncate text-sm font-medium text-foreground">
							{displayName}
						</div>
						<div className="mt-1 truncate text-xs text-muted-foreground">
							@{ownerUser.username} · #{ownerUser.id}
						</div>
					</div>
				</Link>
			) : (
				<p className="mt-1 break-all font-mono text-sm">
					{userId != null ? `#${userId}` : "-"}
				</p>
			)}
		</div>
	);
}
