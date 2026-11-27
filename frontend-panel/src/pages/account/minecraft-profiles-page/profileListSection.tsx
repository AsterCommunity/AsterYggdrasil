import type { FormEvent } from "react";
import { useTranslation } from "react-i18next";
import type { MinecraftProfilesPageController } from "@/components/account/profiles-page/useMinecraftProfilesPageController";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon, type IconName } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { MinecraftSkinAvatar } from "@/components/yggdrasil/MinecraftSkinAvatar";
import { cn } from "@/lib/utils";
import type { YggdrasilProfile } from "@/types/api";

const PROFILE_PAGE_SIZE_OPTIONS = [5, 10] as const;

export function ProfilesPageHeader() {
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

export function ProfileListSection({
	controller,
}: {
	controller: MinecraftProfilesPageController;
}) {
	const { t } = useTranslation();
	const {
		deletingProfile,
		dispatch,
		loading,
		onChangePageSize,
		onCreateProfile,
		onNextProfilePage,
		onOpenRenameDialog,
		onPreviousProfilePage,
		profileCurrentPage,
		profileName,
		profileNextDisabled,
		profilePageSize,
		profilePrevDisabled,
		profileSkinUrls,
		profileTotal,
		profileTotalPages,
		profiles,
		query,
		searchBusy,
		selectedUuid,
	} = controller;

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
					profileSkinUrls={profileSkinUrls}
					profiles={profiles}
					query={query}
					selectedUuid={selectedUuid}
					onOpenRenameDialog={onOpenRenameDialog}
				/>
				<AdminOffsetPagination
					currentPage={profileCurrentPage}
					nextDisabled={profileNextDisabled}
					onNext={onNextProfilePage}
					onPageSizeChange={onChangePageSize}
					onPrevious={onPreviousProfilePage}
					pageSize={String(profilePageSize)}
					pageSizeOptions={PROFILE_PAGE_SIZE_OPTIONS.map((pageSize) => ({
						label: t("admin.pagination.pageSizeOption", {
							count: pageSize,
						}),
						value: String(pageSize),
					}))}
					prevDisabled={profilePrevDisabled}
					total={profileTotal}
					totalPages={profileTotalPages}
				/>
			</div>
		</section>
	);
}

type ProfilesDispatch = MinecraftProfilesPageController["dispatch"];

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
	profileSkinUrls,
	profiles,
	query,
	selectedUuid,
}: {
	deletingProfile: boolean;
	dispatch: ProfilesDispatch;
	onOpenRenameDialog: (profile: { id: string; name: string }) => void;
	profileSkinUrls: Record<string, string | null>;
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
						skinUrl={profileSkinUrls[profile.id] ?? null}
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
	skinUrl,
}: {
	deletingProfile: boolean;
	dispatch: ProfilesDispatch;
	onOpenRenameDialog: (profile: { id: string; name: string }) => void;
	profile: YggdrasilProfile;
	selected: boolean;
	skinUrl: string | null;
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
				<div className="flex min-w-0 items-center gap-2.5">
					<MinecraftSkinAvatar
						name={profile.name}
						testId={`profile-skin-avatar-${profile.id}`}
						imageTestId={`profile-skin-avatar-image-${profile.id}`}
						skinUrl={skinUrl}
					/>
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
