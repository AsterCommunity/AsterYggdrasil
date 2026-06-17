import {
	type FormEvent,
	lazy,
	Suspense,
	useCallback,
	useEffect,
	useMemo,
	useReducer,
} from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { ProfileSummaryPanel } from "@/components/admin/admin-minecraft-profile-page/ProfileSummaryPanel";
import { ProfileTextureList } from "@/components/admin/admin-minecraft-profile-page/ProfileTextureList";
import type {
	AdminMinecraftProfileInfo,
	MinecraftTextureMetadata,
} from "@/components/admin/admin-minecraft-profile-page/types";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
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
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import { adminPaths } from "@/routes/routePaths";
import { adminMinecraftProfileService } from "@/services/adminService";

const MinecraftPreview = lazy(() =>
	import("@/components/yggdrasil/MinecraftPreview").then((module) => ({
		default: module.MinecraftPreview,
	})),
);

function parseUuid(value: string | undefined) {
	if (!value) return null;
	return value.trim() || null;
}

type PageState = {
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
				profile: action.profile,
				textures: action.textures,
				loading: false,
			};
		case "loadError":
			return { ...state, profile: null, textures: [], loading: false };
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
	const navigate = useNavigate();
	const params = useParams();
	const uuid = parseUuid(params.uuid);
	const [state, dispatch] = useReducer(pageReducer, initialPageState);
	const {
		deleteDialogOpen,
		deletingProfile,
		deletingTexture,
		loading,
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
			const [nextProfile, nextTextures] = await Promise.all([
				adminMinecraftProfileService.get(uuid),
				adminMinecraftProfileService.listTextures(uuid),
			]);
			dispatch({
				type: "loadSuccess",
				profile: nextProfile as AdminMinecraftProfileInfo,
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
			navigate(adminPaths.users);
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

	const previewSkinUrl = skinTexture?.url ?? null;
	const previewCapeUrl = capeTexture?.url ?? null;
	const headerActions = useMemo(
		() => (
			<>
				<Button
					type="button"
					variant="outline"
					size="sm"
					render={<Link to={adminPaths.users} />}
				>
					<Icon name="ArrowLeft" className="mr-2 size-4" />
					{t("admin.minecraftProfilePage.backToUsers")}
				</Button>
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={() => void load()}
					disabled={loading}
				>
					<Icon
						name={loading ? "Spinner" : "ArrowsClockwise"}
						className={loading ? "mr-2 size-4 animate-spin" : "mr-2 size-4"}
					/>
					{t("common.refresh")}
				</Button>
				<Button
					type="button"
					variant="destructive"
					size="sm"
					onClick={() => dispatch({ type: "deleteDialogOpen", value: true })}
					disabled={deletingProfile}
				>
					<Icon name="Trash" className="mr-2 size-4" />
					{t("common.delete")}
				</Button>
			</>
		),
		[deletingProfile, load, loading, t],
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

			<div className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)]">
				<AdminSurface className="grid gap-4">
					<ProfileSummaryPanel
						capeTexture={capeTexture}
						profile={profile}
						skinTexture={skinTexture}
						uuid={uuid}
						onRename={openRenameDialog}
						onSelectTextureDelete={(texture) =>
							dispatch({ type: "textureToDelete", value: texture })
						}
					/>
				</AdminSurface>

				<div className="grid gap-4">
					<AdminSurface padded={false} className="overflow-hidden">
						<Suspense fallback={<PreviewSkeleton />}>
							<MinecraftPreview
								label={t("admin.minecraftProfilePage.preview")}
								skinUrl={previewSkinUrl}
								capeUrl={previewCapeUrl}
								model={profile?.texture_model ?? "default"}
								className="rounded-none border-0 shadow-none"
							/>
						</Suspense>
					</AdminSurface>

					<AdminSurface className="grid gap-3">
						<ProfileTextureList
							deletingTexture={deletingTexture}
							loading={loading}
							textures={textures}
							onRefresh={() => void load()}
							onSelectDelete={(texture) =>
								dispatch({ type: "textureToDelete", value: texture })
							}
						/>
					</AdminSurface>
				</div>
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

function PreviewSkeleton() {
	return (
		<div className="h-[29rem] rounded-none border-0 bg-card shadow-none">
			<div className="flex min-h-12 items-center justify-between border-b border-border/70 px-4">
				<div className="grid gap-2">
					<div className="h-4 w-28 rounded bg-muted" />
					<div className="h-3 w-20 rounded bg-muted" />
				</div>
				<div className="flex items-center gap-1.5">
					<div className="size-4 rounded bg-muted" />
					<div className="size-4 rounded bg-muted" />
				</div>
			</div>
			<div className="h-[26rem] bg-muted/30" />
		</div>
	);
}
