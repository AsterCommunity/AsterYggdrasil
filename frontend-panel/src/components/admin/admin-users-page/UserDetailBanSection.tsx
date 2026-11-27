import {
	type FormEvent,
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { DateTimeText } from "@/components/common/DateTimeText";
import { LoadingButton } from "@/components/common/LoadingButton";
import { RefreshButton } from "@/components/common/RefreshButton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import { Textarea } from "@/components/ui/textarea";
import { handleApiError } from "@/hooks/useApiError";
import {
	permissionLabel,
	permissionLabels,
	useRegistryPermissions,
} from "@/lib/permissionRegistry";
import { cn } from "@/lib/utils";
import { banFormSchema } from "@/lib/validation";
import { adminUserService } from "@/services/adminService";
import type {
	AdminPermissionInfo,
	CreateUserBanRequest,
	PermissionId,
	UpdateUserBanRequest,
	UserBanEventInfo,
	UserBanInfo,
	UserBanStatus,
} from "@/types/api";

type BanDialogState =
	| { mode: "create"; ban?: undefined }
	| { mode: "edit"; ban: UserBanInfo };

type BanFormState = {
	adminNote: string;
	expiresAt: string;
	publicReason: string;
	reason: string;
	scopes: PermissionId[];
	startsAt: string;
};

type EventDialogState = {
	ban: UserBanInfo;
	events: UserBanEventInfo[];
	loading: boolean;
};

function createBanFormState(ban?: UserBanInfo): BanFormState {
	return {
		adminNote: ban?.admin_note ?? "",
		expiresAt: toDateTimeLocal(ban?.expires_at),
		publicReason: ban?.public_reason ?? "",
		reason: ban?.reason ?? "",
		scopes: ban?.scopes?.length
			? [...ban.scopes]
			: ["com.astercosm.yggdrasil.capability.yggdrasil.access"],
		startsAt: toDateTimeLocal(ban?.starts_at),
	};
}

function normalizeOptionalText(value: string) {
	const trimmed = value.trim();
	return trimmed ? trimmed : null;
}

function toDateTimeLocal(value: string | null | undefined) {
	if (!value) return "";
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return "";
	const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
	return local.toISOString().slice(0, 16);
}

function fromDateTimeLocal(value: string) {
	if (!value.trim()) return null;
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) return null;
	return date.toISOString();
}

export function UserDetailBanSection({ userId }: { userId: number }) {
	const { t } = useTranslation();
	const [bans, setBans] = useState<UserBanInfo[]>([]);
	const [loading, setLoading] = useState(true);
	const [dialog, setDialog] = useState<BanDialogState | null>(null);
	const [form, setForm] = useState<BanFormState>(() => createBanFormState());
	const [formError, setFormError] = useState<string | null>(null);
	const [submitting, setSubmitting] = useState(false);
	const [revokeBan, setRevokeBan] = useState<UserBanInfo | null>(null);
	const [revokeNote, setRevokeNote] = useState("");
	const [eventDialog, setEventDialog] = useState<EventDialogState | null>(null);

	const activeBanCount = useMemo(
		() => bans.filter((ban) => ban.effective).length,
		[bans],
	);

	const reload = useCallback(async () => {
		setLoading(true);
		try {
			const page = await adminUserService.listBans({
				limit: 50,
				user_id: userId,
			});
			setBans(page.items);
		} catch (error) {
			handleApiError(error);
		} finally {
			setLoading(false);
		}
	}, [userId]);

	useEffect(() => {
		void reload();
	}, [reload]);

	function openCreateDialog() {
		setForm(createBanFormState());
		setFormError(null);
		setDialog({ mode: "create" });
	}

	function openEditDialog(ban: UserBanInfo) {
		setForm(createBanFormState(ban));
		setFormError(null);
		setDialog({ mode: "edit", ban });
	}

	function closeBanDialog() {
		if (submitting) return;
		setDialog(null);
		setFormError(null);
	}

	async function submitBanForm(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!dialog) return;

		const parsed = banFormSchema.safeParse({
			reason: form.reason,
			scopes: form.scopes,
		});
		if (!parsed.success) {
			setFormError(t(parsed.error.issues[0]?.message ?? ""));
			return;
		}
		const reason = parsed.data.reason;

		const startsAt = fromDateTimeLocal(form.startsAt);
		const expiresAt = fromDateTimeLocal(form.expiresAt);
		setSubmitting(true);
		try {
			if (dialog.mode === "create") {
				const payload: CreateUserBanRequest = {
					admin_note: normalizeOptionalText(form.adminNote),
					expires_at: expiresAt,
					public_reason: normalizeOptionalText(form.publicReason),
					reason,
					scopes: form.scopes,
					starts_at: startsAt,
				};
				await adminUserService.createBan(userId, payload);
				toast.success(t("admin.users.bans.created"));
			} else {
				const payload: UpdateUserBanRequest = {
					admin_note: normalizeOptionalText(form.adminNote),
					expires_at: expiresAt,
					public_reason: normalizeOptionalText(form.publicReason),
					reason,
					scopes: form.scopes,
					starts_at: startsAt ?? undefined,
				};
				await adminUserService.updateBan(dialog.ban.id, payload);
				toast.success(t("admin.users.bans.updated"));
			}
			setDialog(null);
			await reload();
		} catch (error) {
			handleApiError(error);
		} finally {
			setSubmitting(false);
		}
	}

	async function submitRevoke() {
		if (!revokeBan) return;
		setSubmitting(true);
		try {
			await adminUserService.revokeBan(revokeBan.id, {
				revoke_note: normalizeOptionalText(revokeNote),
			});
			toast.success(t("admin.users.bans.revoked"));
			setRevokeBan(null);
			setRevokeNote("");
			await reload();
		} catch (error) {
			handleApiError(error);
		} finally {
			setSubmitting(false);
		}
	}

	async function openEventsDialog(ban: UserBanInfo) {
		setEventDialog({ ban, events: [], loading: true });
		try {
			const events = await adminUserService.listBanEvents(ban.id);
			setEventDialog({ ban, events, loading: false });
		} catch (error) {
			handleApiError(error);
			setEventDialog({ ban, events: [], loading: false });
		}
	}

	return (
		<section className="overflow-hidden rounded-lg border border-border/70 bg-background/55 dark:border-white/10 dark:bg-input/10">
			<div className="flex flex-col gap-3 border-b border-border/70 p-4 dark:border-white/10 sm:flex-row sm:items-start sm:justify-between">
				<div>
					<div className="flex flex-wrap items-center gap-2">
						<h3 className="font-medium text-foreground">
							{t("admin.users.bans.section")}
						</h3>
						<Badge
							variant={activeBanCount > 0 ? "destructive" : "outline"}
							className="rounded-md"
						>
							{t("admin.users.bans.activeCount", { count: activeBanCount })}
						</Badge>
					</div>
					<p className="mt-1 text-sm leading-6 text-muted-foreground">
						{t("admin.users.bans.sectionDescription")}
					</p>
				</div>
				<div className="flex shrink-0 gap-2">
					<RefreshButton loading={loading} onClick={() => void reload()} />
					<Button type="button" size="sm" onClick={openCreateDialog}>
						<Icon name="Plus" className="size-4" />
						{t("admin.users.bans.create")}
					</Button>
				</div>
			</div>

			<div className="divide-y divide-border/70 dark:divide-white/10">
				{loading ? (
					<p className="p-4 text-sm text-muted-foreground">
						{t("common.loading")}
					</p>
				) : bans.length ? (
					bans.map((ban) => (
						<BanRow
							key={ban.id}
							ban={ban}
							onEdit={() => openEditDialog(ban)}
							onEvents={() => void openEventsDialog(ban)}
							onRevoke={() => {
								setRevokeBan(ban);
								setRevokeNote("");
							}}
						/>
					))
				) : (
					<div className="p-4 text-sm text-muted-foreground">
						{t("admin.users.bans.empty")}
					</div>
				)}
			</div>

			<BanFormDialog
				dialog={dialog}
				error={formError}
				form={form}
				open={dialog !== null}
				submitting={submitting}
				onClose={closeBanDialog}
				onFormChange={setForm}
				onSubmit={submitBanForm}
			/>
			<RevokeBanDialog
				ban={revokeBan}
				note={revokeNote}
				open={revokeBan !== null}
				submitting={submitting}
				onClose={() => {
					if (submitting) return;
					setRevokeBan(null);
					setRevokeNote("");
				}}
				onNoteChange={setRevokeNote}
				onSubmit={() => void submitRevoke()}
			/>
			<BanEventsDialog
				state={eventDialog}
				open={eventDialog !== null}
				onClose={() => setEventDialog(null)}
			/>
		</section>
	);
}

function BanRow({
	ban,
	onEdit,
	onEvents,
	onRevoke,
}: {
	ban: UserBanInfo;
	onEdit: () => void;
	onEvents: () => void;
	onRevoke: () => void;
}) {
	const { t } = useTranslation();
	const { permissions: capabilityList } = useRegistryPermissions("capability");
	const capabilityLabels = permissionLabels(capabilityList);
	const active = ban.effective;
	return (
		<div className="grid gap-3 p-4 lg:grid-cols-[minmax(13rem,1.1fr)_minmax(0,1.4fr)_minmax(15rem,1fr)_auto] lg:items-center">
			<div className="min-w-0">
				<div className="flex flex-wrap items-center gap-2">
					<Badge
						variant={active ? "destructive" : "outline"}
						className="rounded-md"
					>
						{t(`admin.users.bans.status.${ban.effective_status}`)}
					</Badge>
					<span className="font-medium text-sm">
						{formatScopes(t, ban.scopes, capabilityLabels)}
					</span>
				</div>
				<p className="mt-1 text-xs text-muted-foreground">#{ban.id}</p>
			</div>
			<div className="min-w-0">
				<p className="truncate text-sm">{ban.reason}</p>
				{ban.public_reason ? (
					<p className="mt-1 line-clamp-2 text-muted-foreground text-xs">
						{ban.public_reason}
					</p>
				) : null}
			</div>
			<div className="grid gap-1 text-xs text-muted-foreground sm:grid-cols-2 lg:grid-cols-1">
				<div>
					<span>{t("admin.users.bans.startsAt")}: </span>
					<DateTimeText value={ban.starts_at} />
				</div>
				<div>
					<span>{t("admin.users.bans.expiresAt")}: </span>
					<DateTimeText value={ban.expires_at} />
				</div>
			</div>
			<div className="flex justify-start gap-1.5 lg:justify-end">
				<Button type="button" variant="outline" size="sm" onClick={onEvents}>
					<Icon name="Clock" className="size-4" />
					{t("admin.users.bans.events")}
				</Button>
				<Button
					type="button"
					variant="outline"
					size="sm"
					disabled={ban.status !== "active" || !ban.effective}
					onClick={onEdit}
				>
					<Icon name="PencilSimple" className="size-4" />
					{t("admin.users.bans.edit")}
				</Button>
				<Button
					type="button"
					variant="destructive"
					size="sm"
					disabled={ban.status !== "active" || !ban.effective}
					onClick={onRevoke}
				>
					<Icon name="LockOpen" className="size-4" />
					{t("admin.users.bans.revoke")}
				</Button>
			</div>
		</div>
	);
}

function BanFormDialog({
	dialog,
	error,
	form,
	onClose,
	onFormChange,
	onSubmit,
	open,
	submitting,
}: {
	dialog: BanDialogState | null;
	error: string | null;
	form: BanFormState;
	onClose: () => void;
	onFormChange: (value: BanFormState) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	open: boolean;
	submitting: boolean;
}) {
	const { t } = useTranslation();
	const {
		permissions: capabilities,
		status: registryStatus,
		retry,
	} = useRegistryPermissions("capability");
	const mode = dialog?.mode ?? "create";
	// A failing or in-flight registry must not let an unshown default
	// capability get submitted silently.
	const registryReady = registryStatus === "ready" && capabilities.length > 0;
	return (
		<Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && onClose()}>
			<DialogContent
				keepMounted
				className="flex max-h-[min(720px,calc(100dvh-2rem))] flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl"
			>
				<DialogHeader className="shrink-0 border-border/70 border-b px-5 pt-5 pb-4 dark:border-white/10">
					<DialogTitle>{t(`admin.users.bans.${mode}Title`)}</DialogTitle>
					<DialogDescription>
						{t(`admin.users.bans.${mode}Description`)}
					</DialogDescription>
				</DialogHeader>

				<form
					id="user-ban-form"
					className="min-h-0 flex-1 space-y-4 overflow-y-auto px-5 py-4"
					onSubmit={onSubmit}
				>
					<div className="grid gap-4 md:grid-cols-2">
						<div className="grid gap-2">
							<Label>{t("admin.users.bans.scopeLabel")}</Label>
							{registryStatus !== "ready" ? (
								<div className="flex items-center justify-between gap-3 rounded-md border border-border/70 bg-muted/20 px-3 py-2 text-sm dark:border-white/10 dark:bg-white/[0.03]">
									<span className="text-muted-foreground">
										{registryStatus === "loading"
											? t("admin.users.bans.registryLoading")
											: t("admin.users.bans.registryFailed")}
									</span>
									{registryStatus === "error" ? (
										<Button
											type="button"
											variant="outline"
											size="sm"
											onClick={retry}
										>
											{t("admin.users.retryPermissionsLoad")}
										</Button>
									) : null}
								</div>
							) : null}
							<div className="grid gap-2 rounded-md border border-border/70 bg-muted/20 p-2 dark:border-white/10 dark:bg-white/[0.03]">
								{capabilities.map((permission) => (
									<label
										key={permission.id}
										htmlFor={`user-ban-scope-${permission.id}`}
										className={cn(
											"flex cursor-pointer items-center gap-2 rounded-md border border-transparent px-2.5 py-2 text-sm transition-colors hover:bg-background/70 dark:hover:bg-white/[0.04]",
											form.scopes.includes(permission.id)
												? "border-primary/25 bg-primary/8 text-foreground"
												: "text-muted-foreground",
											submitting && "cursor-not-allowed opacity-60",
										)}
									>
										<Checkbox
											id={`user-ban-scope-${permission.id}`}
											checked={form.scopes.includes(permission.id)}
											disabled={submitting}
											onCheckedChange={(checked) => {
												const nextScopes = checked
													? [...form.scopes, permission.id]
													: form.scopes.filter(
															(item) => item !== permission.id,
														);
												onFormChange({ ...form, scopes: nextScopes });
											}}
										/>
										<span className="min-w-0 flex-1">
											{permissionLabel(permission, t)}
										</span>
									</label>
								))}
							</div>
						</div>
						<div className="grid gap-2">
							<Label htmlFor="user-ban-reason">
								{t("admin.users.bans.reasonLabel")}
							</Label>
							<Input
								id="user-ban-reason"
								value={form.reason}
								maxLength={128}
								disabled={submitting}
								onChange={(event) =>
									onFormChange({ ...form, reason: event.currentTarget.value })
								}
							/>
							{error ? (
								<p className="text-destructive text-xs">{error}</p>
							) : null}
						</div>
						<div className="grid gap-2">
							<Label htmlFor="user-ban-starts-at">
								{t("admin.users.bans.startsAt")}
							</Label>
							<Input
								id="user-ban-starts-at"
								type="datetime-local"
								value={form.startsAt}
								disabled={submitting}
								onChange={(event) =>
									onFormChange({ ...form, startsAt: event.currentTarget.value })
								}
							/>
						</div>
						<div className="grid gap-2">
							<Label htmlFor="user-ban-expires-at">
								{t("admin.users.bans.expiresAt")}
							</Label>
							<Input
								id="user-ban-expires-at"
								type="datetime-local"
								value={form.expiresAt}
								disabled={submitting}
								onChange={(event) =>
									onFormChange({
										...form,
										expiresAt: event.currentTarget.value,
									})
								}
							/>
						</div>
					</div>
					<div className="grid gap-2">
						<Label htmlFor="user-ban-public-reason">
							{t("admin.users.bans.publicReasonLabel")}
						</Label>
						<Textarea
							id="user-ban-public-reason"
							value={form.publicReason}
							maxLength={1000}
							disabled={submitting}
							onChange={(event) =>
								onFormChange({
									...form,
									publicReason: event.currentTarget.value,
								})
							}
						/>
					</div>
					<div className="grid gap-2">
						<Label htmlFor="user-ban-admin-note">
							{t("admin.users.bans.adminNoteLabel")}
						</Label>
						<Textarea
							id="user-ban-admin-note"
							value={form.adminNote}
							maxLength={1000}
							disabled={submitting}
							onChange={(event) =>
								onFormChange({ ...form, adminNote: event.currentTarget.value })
							}
						/>
					</div>
				</form>

				<DialogFooter className="shrink-0 px-5 pt-4 pb-5">
					<Button
						type="button"
						variant="outline"
						disabled={submitting}
						onClick={onClose}
					>
						{t("common.cancel")}
					</Button>
					<LoadingButton
						type="submit"
						form="user-ban-form"
						loading={submitting}
						disabled={!registryReady}
						iconClassName="size-4"
					>
						{t(mode === "create" ? "admin.users.bans.create" : "common.save")}
					</LoadingButton>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function RevokeBanDialog({
	ban,
	note,
	onClose,
	onNoteChange,
	onSubmit,
	open,
	submitting,
}: {
	ban: UserBanInfo | null;
	note: string;
	onClose: () => void;
	onNoteChange: (value: string) => void;
	onSubmit: () => void;
	open: boolean;
	submitting: boolean;
}) {
	const { t } = useTranslation();
	const { permissions: capabilityList } = useRegistryPermissions("capability");
	const capabilityLabels = permissionLabels(capabilityList);
	const descriptionScope = ban
		? formatScopes(t, ban.scopes, capabilityLabels)
		: "";
	return (
		<Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && onClose()}>
			<DialogContent keepMounted className="sm:max-w-lg">
				<DialogHeader>
					<DialogTitle>{t("admin.users.bans.revokeTitle")}</DialogTitle>
					<DialogDescription>
						{ban
							? t("admin.users.bans.revokeDescription", {
									scope: descriptionScope,
								})
							: t("admin.users.bans.revokeTitle")}
					</DialogDescription>
				</DialogHeader>
				<div className="grid gap-2">
					<Label htmlFor="user-ban-revoke-note">
						{t("admin.users.bans.revokeNoteLabel")}
					</Label>
					<Textarea
						id="user-ban-revoke-note"
						value={note}
						maxLength={1000}
						disabled={submitting}
						onChange={(event) => onNoteChange(event.currentTarget.value)}
					/>
				</div>
				<DialogFooter>
					<Button
						type="button"
						variant="outline"
						disabled={submitting}
						onClick={onClose}
					>
						{t("common.cancel")}
					</Button>
					<LoadingButton
						type="button"
						variant="destructive"
						loading={submitting}
						disabled={!ban}
						iconClassName="size-4"
						onClick={onSubmit}
					>
						{t("admin.users.bans.revoke")}
					</LoadingButton>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function BanEventsDialog({
	onClose,
	open,
	state,
}: {
	onClose: () => void;
	open: boolean;
	state: EventDialogState | null;
}) {
	const { t } = useTranslation();
	const { permissions: capabilityList } = useRegistryPermissions("capability");
	const capabilityLabels = permissionLabels(capabilityList);
	return (
		<Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && onClose()}>
			<DialogContent
				keepMounted
				className="flex max-h-[min(720px,calc(100dvh-2rem))] flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl"
			>
				<DialogHeader className="shrink-0 border-border/70 border-b px-5 pt-5 pb-4 dark:border-white/10">
					<DialogTitle>{t("admin.users.bans.eventsTitle")}</DialogTitle>
					<DialogDescription>
						{state
							? t("admin.users.bans.eventsDescription", { id: state.ban.id })
							: t("admin.users.bans.eventsTitle")}
					</DialogDescription>
				</DialogHeader>
				<div className="min-h-0 flex-1 overflow-y-auto">
					{state?.loading ? (
						<p className="p-4 text-sm text-muted-foreground">
							{t("common.loading")}
						</p>
					) : state?.events.length ? (
						<div className="divide-y divide-border/70 dark:divide-white/10">
							{state.events.map((event) => (
								<div key={event.id} className="grid gap-2 p-3 text-sm">
									<div className="flex flex-wrap items-center justify-between gap-2">
										<div className="flex flex-wrap items-center gap-2">
											<Badge variant="outline" className="rounded-md">
												{t(`admin.users.bans.event.${event.event_type}`)}
											</Badge>
											<span className="text-muted-foreground text-xs">
												{t("admin.users.bans.actor", {
													id: event.actor_user_id ?? "-",
												})}
											</span>
										</div>
										<DateTimeText
											value={event.created_at}
											className="text-muted-foreground text-xs"
										/>
									</div>
									<div className="grid gap-1 text-muted-foreground text-xs sm:grid-cols-2">
										<EventTransition
											label={t("admin.users.bans.statusLabel")}
											next={event.next_status}
											previous={event.previous_status}
											translate={(value) =>
												t(`admin.users.bans.status.${value as UserBanStatus}`)
											}
										/>
										<EventTransition
											label={t("admin.users.bans.scopeLabel")}
											next={event.next_scopes}
											previous={event.previous_scopes}
											translate={(value) =>
												formatScopes(t, value, capabilityLabels)
											}
										/>
										<EventTransition
											label={t("admin.users.bans.expiresAt")}
											next={event.next_expires_at}
											previous={event.previous_expires_at}
											translate={(value) => <DateTimeText value={value} />}
										/>
									</div>
									{event.note ? (
										<p className="rounded-md bg-muted/40 p-2 text-xs">
											{event.note}
										</p>
									) : null}
								</div>
							))}
						</div>
					) : (
						<p className="p-4 text-sm text-muted-foreground">
							{t("admin.users.bans.noEvents")}
						</p>
					)}
				</div>
				<DialogFooter className="shrink-0 px-5 pt-4 pb-5">
					<Button type="button" variant="outline" onClick={onClose}>
						{t("common.close")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function EventTransition<T extends string | string[] | null | undefined>({
	label,
	next,
	previous,
	translate,
}: {
	label: string;
	next: T;
	previous: T;
	translate: (value: Exclude<T, null | undefined>) => ReactNode;
}) {
	return (
		<div>
			<span>{label}: </span>
			<span>
				{previous ? translate(previous as Exclude<T, null | undefined>) : "-"}
			</span>
			<span className="px-1">{"->"}</span>
			<span>
				{next ? translate(next as Exclude<T, null | undefined>) : "-"}
			</span>
		</div>
	);
}

function formatScopes(
	t: ReturnType<typeof useTranslation>["t"],
	scopes: readonly PermissionId[],
	labels: Map<PermissionId, AdminPermissionInfo>,
) {
	return scopes
		.map((scope) => {
			const descriptor = labels.get(scope);
			return descriptor ? permissionLabel(descriptor, t) : scope;
		})
		.join(", ");
}
