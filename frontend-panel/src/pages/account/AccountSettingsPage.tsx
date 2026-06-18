import {
	type ChangeEvent,
	type FormEvent,
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useReducer,
	useRef,
	useState,
	useSyncExternalStore,
} from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { AvatarCropDialog } from "@/components/settings/AvatarCropDialog";
import { LoginDevicesSection } from "@/components/settings/LoginDevicesSection";
import { SecurityPasskeysSection } from "@/components/settings/SecurityPasskeysSection";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon, type IconName } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	clearContactVerificationRedirectSearch,
	getContactVerificationRedirectState,
} from "@/lib/contactVerificationRedirect";
import { externalAuthKindIconPath } from "@/lib/externalAuthProviders";
import { cn } from "@/lib/utils";
import { emailSchema } from "@/lib/validation";
import { accountPaths } from "@/routes/routePaths";
import { authService } from "@/services/authService";
import { externalAuthService } from "@/services/externalAuthService";
import { formatUnknownError } from "@/services/http";
import { useAuthStore } from "@/stores/authStore";
import type {
	AuthUserInfo,
	AvatarSource,
	ExternalAuthLinkInfo,
} from "@/types/api";

type SettingsSectionId = "profile" | "security" | "passkeys" | "external-auth";

const EXTERNAL_AUTH_LINK_PAGE_SIZE = 20;

type SectionDefinition = {
	id: SettingsSectionId;
	icon: IconName;
	label: string;
	description: string;
};
type ActiveSettingsSectionStore = {
	getSnapshot: () => SettingsSectionId;
	set: (next: SettingsSectionId) => void;
	subscribe: (listener: () => void) => () => void;
};
type ExternalAuthLinksState = {
	busyId: number | null;
	links: ExternalAuthLinkInfo[];
	linkTotal: number;
	loading: boolean;
	offset: number;
};
type ExternalAuthLinksAction =
	| { type: "loaded"; links: ExternalAuthLinkInfo[]; total: number }
	| { type: "set_busy_id"; value: number | null }
	| { type: "set_loading"; value: boolean }
	| { type: "set_offset"; value: number };
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

const SECTION_IDS: SettingsSectionId[] = [
	"profile",
	"security",
	"passkeys",
	"external-auth",
];

const externalAuthLinksInitialState: ExternalAuthLinksState = {
	busyId: null,
	links: [],
	linkTotal: 0,
	loading: false,
	offset: 0,
};

function displayNameForUser(user: AuthUserInfo) {
	return user.profile?.display_name?.trim() || user.username;
}

function settingSectionElementId(id: SettingsSectionId) {
	return `settings-${id}`;
}

function scrollToSettingsSection(id: SettingsSectionId) {
	const element = document.getElementById(settingSectionElementId(id));
	if (!element) return;
	window.history.replaceState(null, "", `#${id}`);
	element.scrollIntoView({ behavior: "smooth", block: "start" });
}

function initialActiveSettingsSection(): SettingsSectionId {
	if (typeof window === "undefined") {
		return "profile";
	}
	if (window.location.pathname === accountPaths.settingsSecurityCompat) {
		return "security";
	}
	const hash = window.location.hash.replace("#", "");
	return SECTION_IDS.includes(hash as SettingsSectionId)
		? (hash as SettingsSectionId)
		: "profile";
}

function createActiveSettingsSectionStore(
	initialValue: SettingsSectionId,
): ActiveSettingsSectionStore {
	let snapshot = initialValue;
	const listeners = new Set<() => void>();
	return {
		getSnapshot: () => snapshot,
		set: (next) => {
			if (next === snapshot) return;
			snapshot = next;
			for (const listener of listeners) listener();
		},
		subscribe: (listener) => {
			listeners.add(listener);
			return () => listeners.delete(listener);
		},
	};
}

function useActiveSettingsSection() {
	const storeRef = useRef<ActiveSettingsSectionStore | null>(null);
	if (storeRef.current === null) {
		storeRef.current = createActiveSettingsSectionStore(
			initialActiveSettingsSection(),
		);
	}
	const store = storeRef.current;
	const activeSection = useSyncExternalStore(
		store.subscribe,
		store.getSnapshot,
		(): SettingsSectionId => "profile",
	);

	useEffect(() => {
		const hash = window.location.hash.replace("#", "");
		if (SECTION_IDS.includes(hash as SettingsSectionId)) {
			const id = hash as SettingsSectionId;
			window.requestAnimationFrame(() => {
				document
					.getElementById(settingSectionElementId(id))
					?.scrollIntoView({ block: "start" });
			});
		}
	}, []);

	useEffect(() => {
		const elements = SECTION_IDS.map((id) =>
			document.getElementById(settingSectionElementId(id)),
		).filter((element): element is HTMLElement => Boolean(element));
		if (elements.length === 0) return;

		const observer = new IntersectionObserver(
			(entries) => {
				const visible = entries.reduce<IntersectionObserverEntry | null>(
					(best, entry) => {
						if (!entry.isIntersecting) return best;
						if (!best || entry.intersectionRatio > best.intersectionRatio) {
							return entry;
						}
						return best;
					},
					null,
				);
				const id = visible?.target.getAttribute(
					"data-settings-section",
				) as SettingsSectionId | null;
				if (id) store.set(id);
			},
			{
				root: null,
				rootMargin: "-18% 0px -62% 0px",
				threshold: [0.2, 0.4, 0.7],
			},
		);

		for (const element of elements) observer.observe(element);
		return () => observer.disconnect();
	}, [store]);

	return activeSection;
}

function SectionBlock({
	id,
	icon,
	title,
	description,
	children,
}: {
	id: SettingsSectionId;
	icon: IconName;
	title: string;
	description: string;
	children: ReactNode;
}) {
	return (
		<section
			id={settingSectionElementId(id)}
			data-settings-section={id}
			className="scroll-mt-24 border-b border-border/70 py-7 first:pt-0 last:border-b-0 dark:border-white/10"
		>
			<div className="grid gap-5 lg:grid-cols-[14rem_minmax(0,1fr)]">
				<div className="min-w-0">
					<div className="flex items-center gap-2">
						<span className="grid size-8 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary dark:bg-primary/15">
							<Icon name={icon} className="size-4" />
						</span>
						<h2 className="text-base font-semibold text-foreground">{title}</h2>
					</div>
					<p className="mt-2 text-sm leading-6 text-muted-foreground">
						{description}
					</p>
				</div>
				<div className="min-w-0">{children}</div>
			</div>
		</section>
	);
}

function DetailRow({
	label,
	value,
	action,
}: {
	label: string;
	value: ReactNode;
	action?: ReactNode;
}) {
	return (
		<div className="grid min-h-16 gap-1 rounded-md border border-border/60 bg-muted/18 px-3 py-2.5 dark:border-white/10 dark:bg-muted/10">
			<div className="text-xs font-medium text-muted-foreground">{label}</div>
			<div className="min-w-0 text-sm font-semibold text-foreground">
				{value}
			</div>
			{action ? <div className="pt-1">{action}</div> : null}
		</div>
	);
}

function SectionNav({
	sections,
	activeSection,
	onSelect,
}: {
	sections: SectionDefinition[];
	activeSection: SettingsSectionId;
	onSelect: (id: SettingsSectionId) => void;
}) {
	return (
		<nav className="space-y-1">
			{sections.map((section) => {
				const active = activeSection === section.id;
				return (
					<button
						key={section.id}
						type="button"
						onClick={() => onSelect(section.id)}
						className={cn(
							"flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors",
							active
								? "bg-accent text-accent-foreground"
								: "text-muted-foreground hover:bg-accent/45 hover:text-foreground",
						)}
					>
						<Icon name={section.icon} className="size-4 shrink-0" />
						<span className="min-w-0 truncate">{section.label}</span>
					</button>
				);
			})}
		</nav>
	);
}

function externalAuthLinksReducer(
	state: ExternalAuthLinksState,
	action: ExternalAuthLinksAction,
): ExternalAuthLinksState {
	switch (action.type) {
		case "loaded":
			return {
				...state,
				links: action.links,
				linkTotal: action.total,
			};
		case "set_busy_id":
			return { ...state, busyId: action.value };
		case "set_loading":
			return { ...state, loading: action.value };
		case "set_offset":
			return { ...state, offset: action.value };
	}
}

function ExternalAuthLinksSection() {
	const { t } = useTranslation();
	const [state, dispatch] = useReducer(
		externalAuthLinksReducer,
		externalAuthLinksInitialState,
	);
	const { busyId, links, linkTotal, loading, offset } = state;

	const reload = useCallback(
		async (nextOffset = offset) => {
			dispatch({ type: "set_loading", value: true });
			try {
				const page = await externalAuthService.listLinksPage({
					limit: EXTERNAL_AUTH_LINK_PAGE_SIZE,
					offset: nextOffset,
				});
				dispatch({
					type: "loaded",
					links: page.items,
					total: page.total,
				});
			} catch (error) {
				toast.error(formatUnknownError(error));
			} finally {
				dispatch({ type: "set_loading", value: false });
			}
		},
		[offset],
	);

	useEffect(() => {
		void reload();
	}, [reload]);

	async function unlink(link: ExternalAuthLinkInfo) {
		dispatch({ type: "set_busy_id", value: link.id });
		try {
			await externalAuthService.deleteLink(link.id);
			await reload();
			toast.success(t("personalSettings.externalAuthUnlinked"));
		} catch (error) {
			toast.error(formatUnknownError(error));
		} finally {
			dispatch({ type: "set_busy_id", value: null });
		}
	}

	return (
		<div className="rounded-lg border border-border/70 bg-background/55 dark:border-white/10 dark:bg-input/10">
			<div className="flex flex-col gap-3 border-b border-border/70 px-4 py-4 sm:flex-row sm:items-center sm:justify-between dark:border-white/10">
				<div>
					<h3 className="text-sm font-semibold">
						{t("personalSettings.externalAuthTitle")}
					</h3>
					<p className="mt-1 text-xs leading-5 text-muted-foreground">
						{t("personalSettings.externalAuthDescription")}
					</p>
				</div>
				<Button
					type="button"
					variant="outline"
					size="sm"
					disabled={loading}
					onClick={() => void reload()}
				>
					<Icon
						name={loading ? "Spinner" : "ArrowClockwise"}
						className={cn("mr-2 size-4", loading && "animate-spin")}
					/>
					{t("common.refresh")}
				</Button>
			</div>

			<div className="divide-y divide-border/70 dark:divide-white/10">
				{loading ? (
					<div className="px-4 py-6 text-sm text-muted-foreground">
						{t("common.loading")}
					</div>
				) : links.length === 0 ? (
					<div className="px-4 py-6 text-sm text-muted-foreground">
						{t("personalSettings.externalAuthEmpty")}
					</div>
				) : (
					links.map((link) => (
						<div
							key={link.id}
							className="grid gap-3 px-4 py-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center"
						>
							<div className="flex min-w-0 items-center gap-3">
								<img
									src={
										link.provider_icon_url ||
										externalAuthKindIconPath(link.provider_kind)
									}
									alt=""
									className="size-8 rounded-md border border-border/70 bg-background object-contain p-1 dark:border-white/10"
								/>
								<div className="min-w-0">
									<div className="truncate text-sm font-semibold">
										{link.provider_display_name}
									</div>
									<div className="mt-1 truncate text-xs text-muted-foreground">
										{link.email_snapshot ||
											link.display_name_snapshot ||
											link.subject}
									</div>
								</div>
							</div>
							<div className="flex flex-wrap items-center gap-2 sm:justify-end">
								<Badge variant="outline" className="rounded-md">
									{link.provider_kind}
								</Badge>
								<Button
									type="button"
									size="sm"
									variant="outline"
									disabled={busyId === link.id}
									onClick={() => void unlink(link)}
									className="border-destructive/35 text-destructive hover:border-destructive/60 hover:bg-destructive/10 hover:text-destructive"
								>
									<Icon
										name={busyId === link.id ? "Spinner" : "Trash"}
										className={cn(
											"mr-2 size-4",
											busyId === link.id && "animate-spin",
										)}
									/>
									{t("personalSettings.externalAuthUnlink")}
								</Button>
							</div>
						</div>
					))
				)}
			</div>
			<AdminOffsetPagination
				currentPage={Math.floor(offset / EXTERNAL_AUTH_LINK_PAGE_SIZE) + 1}
				nextDisabled={offset + EXTERNAL_AUTH_LINK_PAGE_SIZE >= linkTotal}
				onNext={() =>
					dispatch({
						type: "set_offset",
						value: offset + EXTERNAL_AUTH_LINK_PAGE_SIZE,
					})
				}
				onPageSizeChange={() => {}}
				onPrevious={() =>
					dispatch({
						type: "set_offset",
						value: Math.max(0, offset - EXTERNAL_AUTH_LINK_PAGE_SIZE),
					})
				}
				pageSize={String(EXTERNAL_AUTH_LINK_PAGE_SIZE)}
				pageSizeOptions={[
					{
						label: t("admin.pagination.pageSizeOption", {
							count: EXTERNAL_AUTH_LINK_PAGE_SIZE,
						}),
						value: String(EXTERNAL_AUTH_LINK_PAGE_SIZE),
					},
				]}
				prevDisabled={offset === 0}
				total={linkTotal}
				totalPages={Math.max(
					1,
					Math.ceil(linkTotal / EXTERNAL_AUTH_LINK_PAGE_SIZE),
				)}
			/>
		</div>
	);
}

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

function ProfileSection({ user }: { user: AuthUserInfo }) {
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
	const profileName = displayNameForUser(user);
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

function EmailChangeSection({ user }: { user: AuthUserInfo }) {
	const { t } = useTranslation();
	const refreshUser = useAuthStore((state) => state.refreshUser);
	const [newEmail, setNewEmail] = useState(user.pending_email ?? "");
	const [error, setError] = useState<string | null>(null);
	const [submitting, setSubmitting] = useState(false);
	const [resending, setResending] = useState(false);
	const pendingEmail = user.pending_email?.trim() || null;
	const normalizedCurrent = user.email.trim().toLowerCase();
	const normalizedPending = pendingEmail?.toLowerCase() ?? null;
	const normalizedNext = newEmail.trim().toLowerCase();
	const validation = emailSchema.safeParse(newEmail);
	const canSubmit =
		validation.success &&
		normalizedNext.length > 0 &&
		normalizedNext !== normalizedCurrent &&
		normalizedNext !== normalizedPending;

	async function submit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const result = emailSchema.safeParse(newEmail);
		if (!result.success) {
			setError(result.error.issues[0]?.message ?? "");
			return;
		}

		setError(null);
		setSubmitting(true);
		try {
			await authService.requestEmailChange({ new_email: result.data });
			await refreshUser();
			toast.success(t("personalSettings.emailChangeRequested"));
		} catch (nextError) {
			handleApiError(nextError);
		} finally {
			setSubmitting(false);
		}
	}

	async function resend() {
		setResending(true);
		try {
			await authService.resendEmailChange();
			toast.success(t("personalSettings.emailChangeResent"));
		} catch (nextError) {
			handleApiError(nextError);
		} finally {
			setResending(false);
		}
	}

	return (
		<div className="rounded-lg border border-border/70 bg-background/55 dark:border-white/10 dark:bg-input/10">
			<div className="flex flex-col gap-3 border-b border-border/70 px-4 py-4 sm:flex-row sm:items-start sm:justify-between dark:border-white/10">
				<div className="min-w-0">
					<h3 className="text-sm font-semibold">
						{t("personalSettings.emailSecurityTitle")}
					</h3>
					<p className="mt-1 text-xs leading-5 text-muted-foreground">
						{t("personalSettings.emailSecurityDescription")}
					</p>
				</div>
				<Badge variant="outline" className="w-fit rounded-md">
					{user.email_verified
						? t("personalSettings.emailVerified")
						: t("personalSettings.emailUnverified")}
				</Badge>
			</div>
			<div className="grid gap-4 px-4 py-4">
				<div className="grid gap-3 sm:grid-cols-2">
					<DetailRow
						label={t("personalSettings.currentEmail")}
						value={<span className="truncate">{user.email}</span>}
					/>
					<DetailRow
						label={t("personalSettings.pendingEmail")}
						value={
							pendingEmail ? (
								<span className="truncate">{pendingEmail}</span>
							) : (
								<span className="text-muted-foreground">
									{t("personalSettings.emptyValue")}
								</span>
							)
						}
					/>
				</div>
				<form className="grid gap-3" onSubmit={submit}>
					<div className="grid gap-2">
						<label
							htmlFor="new-email"
							className="text-xs font-medium text-muted-foreground"
						>
							{t("personalSettings.newEmail")}
						</label>
						<div className="flex flex-col gap-2 sm:flex-row">
							<Input
								id="new-email"
								type="email"
								value={newEmail}
								onChange={(event) => {
									const value = event.currentTarget.value;
									setNewEmail(value);
									if (error) {
										const next = emailSchema.safeParse(value);
										setError(
											next.success
												? null
												: (next.error.issues[0]?.message ?? ""),
										);
									}
								}}
								autoComplete="email"
								placeholder={t("personalSettings.newEmailPlaceholder")}
								aria-invalid={Boolean(error)}
								aria-describedby={error ? "new-email-error" : undefined}
							/>
							<Button type="submit" disabled={submitting || !canSubmit}>
								<Icon
									name={submitting ? "Spinner" : "EnvelopeSimple"}
									className={cn("mr-2 size-4", submitting && "animate-spin")}
								/>
								{t("personalSettings.emailChangeSubmit")}
							</Button>
						</div>
						{error ? (
							<p
								id="new-email-error"
								className="text-xs leading-5 text-destructive"
							>
								{t(error)}
							</p>
						) : null}
					</div>
				</form>
				{pendingEmail ? (
					<div className="flex flex-col gap-2 rounded-md border border-border/60 bg-muted/18 p-3 sm:flex-row sm:items-center sm:justify-between dark:border-white/10 dark:bg-muted/10">
						<div className="text-sm leading-6 text-muted-foreground">
							{t("personalSettings.emailChangePendingHint", {
								email: pendingEmail,
							})}
						</div>
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={resending}
							onClick={() => void resend()}
						>
							<Icon
								name={resending ? "Spinner" : "ArrowClockwise"}
								className={cn("mr-2 size-4", resending && "animate-spin")}
							/>
							{t("personalSettings.emailChangeResend")}
						</Button>
					</div>
				) : null}
			</div>
		</div>
	);
}

export default function AccountSettingsPage() {
	const { t } = useTranslation();
	const user = useAuthStore((state) => state.user);
	const refreshUser = useAuthStore((state) => state.refreshUser);
	const location = useLocation();
	const navigate = useNavigate();
	const locationSearch = location.search;
	const activeSection = useActiveSettingsSection();

	usePageTitle(t("personalSettings.title"));

	useEffect(() => {
		const redirect = getContactVerificationRedirectState(locationSearch);
		if (!redirect) return;
		if (redirect.status === "email-changed") {
			toast.success(
				redirect.email
					? t("personalSettings.emailChangeConfirmedWithEmail", {
							email: redirect.email,
						})
					: t("personalSettings.emailChangeConfirmed"),
			);
			void refreshUser();
		}

		navigate(
			{
				pathname: accountPaths.settings,
				search: clearContactVerificationRedirectSearch(locationSearch),
				hash: "security",
			},
			{ replace: true },
		);
	}, [locationSearch, navigate, refreshUser, t]);

	const sections = useMemo<SectionDefinition[]>(
		() => [
			{
				id: "profile",
				icon: "User",
				label: t("personalSettings.sections.profile"),
				description: t("personalSettings.sectionDescriptions.profile"),
			},
			{
				id: "security",
				icon: "Shield",
				label: t("personalSettings.sections.security"),
				description: t("personalSettings.sectionDescriptions.security"),
			},
			{
				id: "passkeys",
				icon: "Key",
				label: t("personalSettings.sections.passkeys"),
				description: t("personalSettings.sectionDescriptions.passkeys"),
			},
			{
				id: "external-auth",
				icon: "LinkSimple",
				label: t("personalSettings.sections.externalAuth"),
				description: t("personalSettings.sectionDescriptions.externalAuth"),
			},
		],
		[t],
	);

	if (!user) {
		return (
			<div className="mx-auto w-full max-w-[96rem] px-4 py-5 sm:px-6 lg:px-7">
				<div className="rounded-lg border border-border/70 bg-card p-5 text-card-foreground shadow-sm dark:border-white/10 dark:bg-card/90 dark:shadow-none">
					<div className="flex items-center gap-2 text-sm text-muted-foreground">
						<Icon name="Spinner" className="size-4 animate-spin" />
						{t("common.loading")}
					</div>
				</div>
			</div>
		);
	}

	const profileName = displayNameForUser(user);
	const avatarSource = user.profile?.avatar.source ?? "none";
	const statusLabel =
		t(`personalSettings.status_${user.status}`, {
			defaultValue: user.status,
		}) || user.status;

	const accountRows = [
		{
			label: t("personalSettings.username"),
			value: user.username || t("personalSettings.emptyValue"),
		},
		{
			label: t("personalSettings.email"),
			value: user.email || t("personalSettings.emptyValue"),
		},
		{
			label: t("personalSettings.role"),
			value: user.role || t("personalSettings.emptyValue"),
		},
		{
			label: t("personalSettings.accountStatus"),
			value: statusLabel,
		},
		{
			label: t("personalSettings.avatarSource"),
			value: t(`personalSettings.avatarSource_${avatarSource}`),
		},
	] as const;

	const userProfileKey = `${user.id}:${user.profile?.display_name ?? ""}`;

	return (
		<div className="mx-auto grid w-full max-w-[104rem] gap-6 px-4 py-5 sm:px-6 lg:grid-cols-[minmax(0,1fr)_15rem] lg:px-7">
			<div className="min-w-0">
				<div className="mb-5 border-b border-border/70 pb-5 dark:border-white/10">
					<div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
						<div className="min-w-0">
							<h1 className="text-2xl font-semibold tracking-normal text-foreground sm:text-3xl">
								{t("personalSettings.title")}
							</h1>
							<p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
								{t("personalSettings.description")}
							</p>
						</div>
						<div className="flex items-center gap-3 rounded-lg border border-border/70 bg-card px-3 py-2 dark:border-white/10 dark:bg-card/90">
							<UserAvatarImage
								name={profileName}
								avatar={user.profile?.avatar}
								size="md"
							/>
							<div className="min-w-0">
								<div className="truncate text-sm font-semibold">
									{profileName}
								</div>
								<div className="truncate text-xs text-muted-foreground">
									{user.email}
								</div>
							</div>
						</div>
					</div>

					<div className="mt-4 flex gap-2 overflow-x-auto pb-1 lg:hidden">
						{sections.map((section) => (
							<Button
								key={section.id}
								type="button"
								size="sm"
								variant={activeSection === section.id ? "default" : "outline"}
								onClick={() => scrollToSettingsSection(section.id)}
								className="shrink-0"
							>
								<Icon name={section.icon} className="mr-2 size-4" />
								{section.label}
							</Button>
						))}
					</div>
				</div>

				<div className="min-w-0">
					<ProfileSection key={userProfileKey} user={user} />

					<SectionBlock
						id="security"
						icon="Shield"
						title={t("personalSettings.sections.security")}
						description={t("personalSettings.sectionDescriptions.security")}
					>
						<div className="rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
							<div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
								{accountRows.map((row) => (
									<DetailRow
										key={row.label}
										label={row.label}
										value={
											row.label === t("personalSettings.accountStatus") ? (
												<Badge
													variant="outline"
													className="h-6 w-fit rounded-md px-2"
												>
													{row.value}
												</Badge>
											) : (
												<span className="truncate">{row.value}</span>
											)
										}
									/>
								))}
							</div>
							<Separator className="my-4" />
							<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
								<div className="text-sm leading-6 text-muted-foreground">
									{t("personalSettings.securityDescription")}
								</div>
							</div>
							<div className="mt-4">
								<EmailChangeSection user={user} />
							</div>
							<div className="mt-4">
								<LoginDevicesSection />
							</div>
						</div>
					</SectionBlock>

					<SectionBlock
						id="passkeys"
						icon="Key"
						title={t("personalSettings.sections.passkeys")}
						description={t("personalSettings.sectionDescriptions.passkeys")}
					>
						<SecurityPasskeysSection />
					</SectionBlock>

					<SectionBlock
						id="external-auth"
						icon="LinkSimple"
						title={t("personalSettings.sections.externalAuth")}
						description={t("personalSettings.sectionDescriptions.externalAuth")}
					>
						<ExternalAuthLinksSection />
					</SectionBlock>
				</div>
			</div>

			<aside className="hidden lg:block">
				<div className="sticky top-20 rounded-lg border border-border/70 bg-card p-2 shadow-sm dark:border-white/10 dark:bg-card/90 dark:shadow-none">
					<div className="px-3 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
						{t("personalSettings.sectionNav")}
					</div>
					<SectionNav
						sections={sections}
						activeSection={activeSection}
						onSelect={scrollToSettingsSection}
					/>
				</div>
			</aside>
		</div>
	);
}
