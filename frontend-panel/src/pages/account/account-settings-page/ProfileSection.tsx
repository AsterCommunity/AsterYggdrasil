import { type ChangeEvent, useReducer, useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { AvatarCropDialog } from "@/components/settings/AvatarCropDialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { handleApiError } from "@/hooks/useApiError";
import { SectionBlock } from "@/pages/account/account-settings-page/settingsSection";
import { useAuthStore } from "@/stores/authStore";
import type { AuthUserInfo, AvatarSource } from "@/types/api";

type ProfileSectionState = {
	cropOpen: boolean;
	displayName: string;
	savingAvatar: boolean;
	savingProfile: boolean;
	selectedFile: File | null;
};

type ProfileSectionAction =
	| { type: "avatar_file_selected"; file: File }
	| { type: "close_crop" }
	| { type: "set_crop_open"; value: boolean }
	| { type: "set_display_name"; value: string }
	| { type: "set_saving_avatar"; value: boolean }
	| { type: "set_saving_profile"; value: boolean };

function profileSectionReducer(
	state: ProfileSectionState,
	action: ProfileSectionAction,
): ProfileSectionState {
	switch (action.type) {
		case "avatar_file_selected":
			return { ...state, cropOpen: true, selectedFile: action.file };
		case "close_crop":
			return { ...state, cropOpen: false, selectedFile: null };
		case "set_crop_open":
			return { ...state, cropOpen: action.value };
		case "set_display_name":
			return { ...state, displayName: action.value };
		case "set_saving_avatar":
			return { ...state, savingAvatar: action.value };
		case "set_saving_profile":
			return { ...state, savingProfile: action.value };
	}
}

export function ProfileSection({ user }: { user: AuthUserInfo }) {
	const { t } = useTranslation();
	const updateProfile = useAuthStore((state) => state.updateProfile);
	const updateAvatarSource = useAuthStore((state) => state.setAvatarSource);
	const updateAvatarFile = useAuthStore((state) => state.uploadAvatar);
	const fileInputRef = useRef<HTMLInputElement | null>(null);
	const [state, dispatch] = useReducer(profileSectionReducer, {
		cropOpen: false,
		displayName: user.profile?.display_name ?? "",
		savingAvatar: false,
		savingProfile: false,
		selectedFile: null,
	} satisfies ProfileSectionState);
	const { cropOpen, displayName, savingAvatar, savingProfile, selectedFile } =
		state;
	const profileName = user.profile?.display_name?.trim() || user.username;
	const avatarSource = user.profile?.avatar.source ?? "none";
	const profileDirty = displayName !== (user.profile?.display_name ?? "");
	const actionBusy = savingProfile || savingAvatar;

	const handleAvatarFileChange = (event: ChangeEvent<HTMLInputElement>) => {
		const file = event.currentTarget.files?.[0] ?? null;
		event.currentTarget.value = "";
		if (!file) return;
		dispatch({ type: "avatar_file_selected", file });
	};

	const saveProfile = async () => {
		dispatch({ type: "set_saving_profile", value: true });
		try {
			await updateProfile({
				display_name: displayName.trim() || null,
			});
			toast.success(t("personalSettings.profileUpdated"));
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "set_saving_profile", value: false });
		}
	};

	const uploadAvatar = async (file: File) => {
		dispatch({ type: "set_saving_avatar", value: true });
		try {
			await updateAvatarFile(file);
			toast.success(t("personalSettings.avatarUpdated"));
			return true;
		} catch (error) {
			handleApiError(error);
			return false;
		} finally {
			dispatch({ type: "set_saving_avatar", value: false });
		}
	};

	const setAvatarSource = async (source: Exclude<AvatarSource, "upload">) => {
		dispatch({ type: "set_saving_avatar", value: true });
		try {
			await updateAvatarSource({ source });
			toast.success(t("personalSettings.avatarSourceUpdated"));
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "set_saving_avatar", value: false });
		}
	};

	return (
		<SectionBlock
			id="profile"
			icon="User"
			title={t("personalSettings.sections.profile")}
			description={t("personalSettings.sectionDescriptions.profile")}
		>
			<div className="space-y-4">
				<div className="rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
					<div className="flex flex-col gap-5 md:flex-row md:items-start md:justify-between">
						<div className="min-w-0">
							<h3 className="text-sm font-semibold">
								{t("personalSettings.avatarTitle")}
							</h3>
							<p className="mt-1 text-xs leading-5 text-muted-foreground">
								{t("personalSettings.avatarDescription")}
							</p>
							<div className="mt-4 flex flex-wrap gap-2">
								<input
									ref={fileInputRef}
									type="file"
									accept="image/png,image/jpeg,image/webp"
									aria-label={t("personalSettings.avatarUpload")}
									className="sr-only"
									onChange={handleAvatarFileChange}
								/>
								<Button
									type="button"
									size="sm"
									disabled={actionBusy}
									onClick={() => fileInputRef.current?.click()}
								>
									<Icon name="Upload" className="mr-2 size-4" />
									{t("personalSettings.avatarUpload")}
								</Button>
								<Button
									type="button"
									size="sm"
									variant="outline"
									disabled={actionBusy || avatarSource === "gravatar"}
									onClick={() => void setAvatarSource("gravatar")}
								>
									<Icon name="EnvelopeSimple" className="mr-2 size-4" />
									{t("personalSettings.avatarUseGravatar")}
								</Button>
								<Button
									type="button"
									size="sm"
									variant="outline"
									disabled={actionBusy || avatarSource === "none"}
									onClick={() => void setAvatarSource("none")}
									className="border-destructive/35 text-destructive hover:border-destructive/60 hover:bg-destructive/10 hover:text-destructive"
								>
									<Icon name="Trash" className="mr-2 size-4" />
									{t("personalSettings.avatarRemove")}
								</Button>
							</div>
						</div>
						<div className="flex items-center gap-4">
							<UserAvatarImage
								name={profileName}
								avatar={user.profile?.avatar}
								size="xl"
								className="rounded-2xl"
							/>
							<Badge variant="outline" className="rounded-md md:hidden">
								{t(`personalSettings.avatarSource_${avatarSource}`)}
							</Badge>
						</div>
					</div>
				</div>

				<div className="rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
					<label
						htmlFor="display-name"
						className="text-sm font-medium text-foreground"
					>
						{t("personalSettings.displayName")}
					</label>
					<p className="mt-1 text-xs leading-5 text-muted-foreground">
						{t("personalSettings.displayNameDescription")}
					</p>
					<div className="mt-3 flex flex-col gap-3 sm:flex-row">
						<Input
							id="display-name"
							value={displayName}
							maxLength={64}
							placeholder={t("personalSettings.displayNamePlaceholder")}
							disabled={actionBusy}
							onChange={(event) =>
								dispatch({
									type: "set_display_name",
									value: event.currentTarget.value,
								})
							}
							className="h-10 rounded-lg"
						/>
						<Button
							type="button"
							disabled={!profileDirty || actionBusy}
							onClick={() => void saveProfile()}
							className="h-10 rounded-lg sm:w-36"
						>
							{savingProfile ? (
								<Icon name="Spinner" className="mr-2 size-4 animate-spin" />
							) : (
								<Icon name="FloppyDisk" className="mr-2 size-4" />
							)}
							{t("common.save")}
						</Button>
					</div>
				</div>
			</div>

			<AvatarCropDialog
				open={cropOpen}
				file={selectedFile}
				busy={savingAvatar}
				onOpenChange={(open) => {
					dispatch(
						open
							? { type: "set_crop_open", value: true }
							: { type: "close_crop" },
					);
				}}
				onConfirm={uploadAvatar}
			/>
		</SectionBlock>
	);
}
