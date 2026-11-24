import {
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useReducer,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { DateTimeText } from "@/components/common/DateTimeText";
import { SessionPlatformIcon } from "@/components/common/SessionPlatformIcon";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon, type IconName } from "@/components/ui/icon";
import { formatUserAgentLabel } from "@/lib/userAgent";
import { cn } from "@/lib/utils";
import { authService } from "@/services/authService";
import { formatUnknownError } from "@/services/http";
import { useAuthStore } from "@/stores/authStore";
import type { AuthSessionInfo, DateTimeStringCursor } from "@/types/api";

const SESSION_PAGE_SIZE_OPTIONS = [5, 10] as const;
const DEFAULT_SESSION_PAGE_SIZE = 5;
const RELATIVE_TIME_FORMATTER = new Intl.RelativeTimeFormat(undefined, {
	numeric: "auto",
});

type LoginDevicesState = {
	loading: boolean;
	pageSize: number;
	revokeBusyId: string | null;
	revokeOthersBusy: boolean;
	sessionTotal: number;
	sessions: AuthSessionInfo[];
};

type LoginDevicesAction =
	| { type: "loaded"; sessions: AuthSessionInfo[]; total: number }
	| { type: "set_loading"; value: boolean }
	| { type: "set_page_size"; value: number }
	| { type: "set_revoke_busy_id"; value: string | null }
	| { type: "set_revoke_others_busy"; value: boolean };

const loginDevicesInitialState: LoginDevicesState = {
	loading: true,
	pageSize: DEFAULT_SESSION_PAGE_SIZE,
	revokeBusyId: null,
	revokeOthersBusy: false,
	sessionTotal: 0,
	sessions: [],
};

function loginDevicesReducer(
	state: LoginDevicesState,
	action: LoginDevicesAction,
): LoginDevicesState {
	switch (action.type) {
		case "loaded":
			return {
				...state,
				sessionTotal: action.total,
				sessions: sortSessions(action.sessions),
			};
		case "set_loading":
			return { ...state, loading: action.value };
		case "set_page_size":
			return { ...state, pageSize: action.value };
		case "set_revoke_busy_id":
			return { ...state, revokeBusyId: action.value };
		case "set_revoke_others_busy":
			return { ...state, revokeOthersBusy: action.value };
	}
}

export function LoginDevicesSection() {
	const { t } = useTranslation();
	const clearAuth = useAuthStore((state) => state.clear);
	const [state, dispatch] = useReducer(
		loginDevicesReducer,
		loginDevicesInitialState,
	);
	const [cursorStack, setCursorStack] = useState<DateTimeStringCursor[]>([]);
	const [nextCursor, setNextCursor] = useState<DateTimeStringCursor | null>(
		null,
	);
	const {
		loading,
		pageSize,
		revokeBusyId,
		revokeOthersBusy,
		sessionTotal,
		sessions,
	} = state;

	const loadSessions = useCallback(
		async (stack: DateTimeStringCursor[] = cursorStack) => {
			dispatch({ type: "set_loading", value: true });
			try {
				const cursor = stack.at(-1);
				const next = await authService.sessionsPage({
					limit: pageSize,
					after_last_seen_at: cursor?.value,
					after_id: cursor?.id,
				});
				if (next.items.length === 0 && next.total > 0 && stack.length > 0) {
					setCursorStack((current) => current.slice(0, -1));
					setNextCursor(null);
					return;
				}
				dispatch({
					type: "loaded",
					sessions: next.items,
					total: next.total,
				});
				setNextCursor(next.next_cursor ?? null);
			} catch (nextError: unknown) {
				toast.error(formatUnknownError(nextError));
			} finally {
				dispatch({ type: "set_loading", value: false });
			}
		},
		[cursorStack, pageSize],
	);

	useEffect(() => {
		void loadSessions();
	}, [loadSessions]);

	const visibleSessions = useMemo(
		() => sessions.filter((session) => isActiveSession(session)),
		[sessions],
	);
	const activeSessions = visibleSessions.length;
	const activeOtherSessions = useMemo(
		() => visibleSessions.filter((session) => !session.is_current).length,
		[visibleSessions],
	);
	const latestActivity = findLatestDate(
		visibleSessions.map((session) => session.last_seen_at),
	);
	const nearestExpiry = findNearestFutureDate(
		visibleSessions.map((session) => session.refresh_expires_at),
	);
	const revokeSession = useCallback(
		async (session: AuthSessionInfo) => {
			dispatch({ type: "set_revoke_busy_id", value: session.id });
			try {
				if (session.is_current) {
					await authService.revokeSession(session.id);
					toast.success(t("sessions.currentSessionRevoked"));
					clearAuth();
					return;
				}
				await authService.revokeSession(session.id);
				await loadSessions();
				toast.success(t("sessions.sessionRevoked"));
			} catch (nextError: unknown) {
				toast.error(formatUnknownError(nextError));
			} finally {
				dispatch({ type: "set_revoke_busy_id", value: null });
			}
		},
		[clearAuth, loadSessions, t],
	);
	const revokeOtherSessions = useCallback(async () => {
		dispatch({ type: "set_revoke_others_busy", value: true });
		try {
			await authService.revokeOtherSessions();
			setCursorStack([]);
			setNextCursor(null);
			await loadSessions([]);
			toast.success(t("sessions.otherSessionsRevoked"));
		} catch (nextError: unknown) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "set_revoke_others_busy", value: false });
		}
	}, [loadSessions, t]);

	function changePageSize(value: string | null) {
		const parsed = Number(value);
		const nextPageSize = SESSION_PAGE_SIZE_OPTIONS.includes(
			parsed as (typeof SESSION_PAGE_SIZE_OPTIONS)[number],
		)
			? parsed
			: DEFAULT_SESSION_PAGE_SIZE;
		dispatch({ type: "set_page_size", value: nextPageSize });
		setCursorStack([]);
		setNextCursor(null);
	}

	return (
		<div className="@container/devices rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
			<div className="flex flex-col gap-3 @min-[42rem]/devices:flex-row @min-[42rem]/devices:items-center @min-[42rem]/devices:justify-between">
				<div className="min-w-0">
					<h3 className="text-sm font-semibold">
						{t("sessions.deviceListTitle")}
					</h3>
					<p className="mt-1 text-xs leading-5 text-muted-foreground">
						{t("sessions.deviceListDescription")}
					</p>
				</div>
				<div className="flex flex-wrap gap-2">
					<Badge
						variant="outline"
						className="rounded-md border-emerald-200 text-emerald-700 dark:border-emerald-400/30 dark:text-emerald-200"
					>
						{t("sessions.activeCount", { count: activeSessions })}
					</Badge>
					<Button
						type="button"
						variant="outline"
						size="sm"
						disabled={loading || revokeOthersBusy || activeOtherSessions === 0}
						onClick={() => {
							void revokeOtherSessions();
						}}
						className="rounded-md"
					>
						<Icon
							name={revokeOthersBusy ? "Spinner" : "SignOut"}
							className={cn("size-3.5", revokeOthersBusy && "animate-spin")}
						/>
						{t("sessions.revokeOtherSessions")}
					</Button>
					<Button
						type="button"
						variant="outline"
						size="sm"
						disabled={loading}
						onClick={() => {
							void loadSessions();
						}}
						className="rounded-md"
					>
						<Icon
							name={loading ? "Spinner" : "ArrowClockwise"}
							className={cn("size-3.5", loading && "animate-spin")}
						/>
						{t("common.refresh")}
					</Button>
				</div>
			</div>
			<div className="mt-4 grid overflow-hidden rounded-lg border border-border/70 bg-muted/20 dark:border-white/10 dark:bg-muted/10 @min-[48rem]/devices:grid-cols-3">
				<SummaryMetric
					icon="WifiHigh"
					label={t("sessions.activeSessions")}
					value={String(activeSessions)}
					tone="green"
				/>
				<SummaryMetric
					icon="Clock"
					label={t("sessions.latestActivity")}
					value={
						latestActivity
							? formatRelativeDateTime(latestActivity)
							: t("sessions.noActivity")
					}
					tone="blue"
				/>
				<SummaryMetric
					icon="Key"
					label={t("sessions.nearestExpiry")}
					value={
						nearestExpiry
							? formatRelativeDateTime(nearestExpiry)
							: t("sessions.noExpiry")
					}
					tone="violet"
				/>
			</div>
			<div className="mt-4">
				{loading ? (
					<div className="grid min-h-52 place-items-center rounded-lg border border-dashed border-border/80 bg-muted/20 text-sm text-muted-foreground dark:border-white/12 dark:bg-muted/12">
						<span className="inline-flex items-center gap-2">
							<Icon name="Spinner" className="size-4 animate-spin" />
							{t("common.loading")}
						</span>
					</div>
				) : visibleSessions.length === 0 ? (
					<div className="grid min-h-52 place-items-center rounded-lg border border-dashed border-border/80 bg-muted/20 px-4 text-center dark:border-white/12 dark:bg-muted/12">
						<div>
							<div className="mx-auto grid size-11 place-items-center rounded-lg bg-background text-muted-foreground shadow-xs dark:bg-input/20 dark:shadow-none">
								<Icon name="Browsers" className="size-5" />
							</div>
							<p className="mt-3 text-sm font-semibold">
								{t("sessions.noSessions")}
							</p>
							<p className="mt-1 text-sm text-muted-foreground">
								{t("sessions.noSessionsDescription")}
							</p>
						</div>
					</div>
				) : (
					<div className="space-y-3">
						{visibleSessions.map((session) => (
							<SessionRow
								key={session.id}
								session={session}
								busy={revokeBusyId === session.id}
								onRevoke={revokeSession}
							/>
						))}
						<AdminOffsetPagination
							currentPage={cursorStack.length + 1}
							nextDisabled={!nextCursor}
							onNext={() => {
								if (!nextCursor) return;
								setCursorStack((current) => [...current, nextCursor]);
							}}
							onPageSizeChange={changePageSize}
							onPrevious={() =>
								setCursorStack((current) => current.slice(0, -1))
							}
							pageSize={String(pageSize)}
							pageSizeOptions={SESSION_PAGE_SIZE_OPTIONS.map((size) => ({
								label: t("admin.pagination.pageSizeOption", { count: size }),
								value: String(size),
							}))}
							prevDisabled={cursorStack.length === 0}
							total={sessionTotal}
							totalPages={Math.max(
								cursorStack.length + (nextCursor ? 2 : 1),
								1,
							)}
						/>
					</div>
				)}
			</div>
		</div>
	);
}

function SessionRow({
	busy,
	onRevoke,
	session,
}: {
	busy: boolean;
	onRevoke: (session: AuthSessionInfo) => void | Promise<void>;
	session: AuthSessionInfo;
}) {
	const { t } = useTranslation();
	const userAgentLabel = formatUserAgentLabel(session.user_agent, {
		desktop: t("sessions.desktopDevice"),
		mobile: t("sessions.mobileDevice"),
		tablet: t("sessions.tabletDevice"),
		unknown: t("sessions.unknownDevice"),
	});
	return (
		<article className="@container/session rounded-lg border border-border/70 bg-muted/18 p-3 transition-[border-color,background-color] hover:border-emerald-300/80 hover:bg-emerald-50/45 dark:border-white/10 dark:bg-muted/10 dark:hover:border-emerald-400/35 dark:hover:bg-emerald-400/10 ">
			<div className="flex flex-col gap-3 @min-[44rem]/session:flex-row @min-[44rem]/session:items-center @min-[44rem]/session:justify-between">
				<div className="flex min-w-0 items-center gap-3">
					<div className="grid size-10 shrink-0 place-items-center rounded-lg bg-background shadow-xs dark:bg-input/20 dark:shadow-none">
						<SessionPlatformIcon userAgent={session.user_agent} />
					</div>
					<div className="min-w-0">
						<div className="flex flex-wrap items-center gap-2">
							<h3 className="truncate text-sm font-semibold text-foreground">
								{userAgentLabel}
							</h3>
							<Badge
								variant="outline"
								className="rounded-md border-emerald-200 bg-emerald-50 px-1.5 py-0 text-[11px] text-emerald-700 dark:border-emerald-400/30 dark:bg-emerald-400/10 dark:text-emerald-200"
							>
								{t("sessions.status.active")}
							</Badge>
							{session.is_current ? (
								<Badge
									variant="secondary"
									className="rounded-md bg-emerald-100 px-1.5 py-0 text-[11px] text-emerald-800 dark:bg-emerald-400/15 dark:text-emerald-200"
								>
									{t("sessions.currentSession")}
								</Badge>
							) : null}
						</div>
						<div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
							<span>
								{t("sessions.lastSeen")}{" "}
								<DateTimeText
									value={session.last_seen_at}
									className="font-medium text-foreground"
								/>
							</span>
						</div>
					</div>
				</div>

				<div className="flex shrink-0 flex-wrap gap-2 text-xs @min-[44rem]/session:justify-end">
					<SessionMetaPill
						label={t("sessions.ipAddress")}
						value={session.ip_address ?? t("sessions.unknownIp")}
					/>
					<SessionMetaPill
						label={t("sessions.expiresAt")}
						value={<DateTimeText value={session.refresh_expires_at} />}
					/>
					<Button
						type="button"
						variant={session.is_current ? "destructive" : "outline"}
						size="sm"
						disabled={busy}
						onClick={() => {
							void onRevoke(session);
						}}
						className={cn(
							"h-8 rounded-md px-2 text-xs",
							!session.is_current &&
								"border-border/70 bg-background/70 dark:border-white/10 dark:bg-input/18",
						)}
					>
						<Icon
							name={busy ? "Spinner" : "SignOut"}
							className={cn("size-3.5", busy && "animate-spin")}
						/>
						{session.is_current
							? t("sessions.revokeCurrentSession")
							: t("sessions.revokeSession")}
					</Button>
				</div>
			</div>
		</article>
	);
}

function SummaryMetric({
	icon,
	label,
	value,
	tone,
}: {
	icon: IconName;
	label: string;
	value: string;
	tone: "green" | "blue" | "violet";
}) {
	const toneClass = {
		green:
			"bg-emerald-100 text-emerald-700 dark:bg-emerald-500/15 dark:text-emerald-200",
		blue: "bg-blue-100 text-blue-700 dark:bg-blue-500/15 dark:text-blue-200",
		violet:
			"bg-violet-100 text-violet-700 dark:bg-violet-500/15 dark:text-violet-200",
	}[tone];
	return (
		<div className="flex min-h-20 items-center gap-3 border-t border-border/70 px-4 py-3 first:border-t-0 dark:border-white/10 @min-[48rem]/devices:border-t-0 @min-[48rem]/devices:border-l @min-[48rem]/devices:first:border-l-0">
			<div
				className={cn("grid size-9 place-items-center rounded-lg", toneClass)}
			>
				<Icon name={icon} className="size-4" />
			</div>
			<div className="min-w-0">
				<div className="text-xs font-medium text-muted-foreground">{label}</div>
				<div className="mt-0.5 truncate text-base font-semibold text-foreground">
					{value}
				</div>
			</div>
		</div>
	);
}

function SessionMetaPill({
	label,
	value,
}: {
	label: string;
	value: ReactNode;
}) {
	return (
		<div className="inline-flex min-h-8 max-w-full items-center gap-2 rounded-md border border-border/60 bg-background/70 px-2.5 text-muted-foreground dark:border-white/10 dark:bg-input/18">
			<span className="font-medium">{label}</span>
			<span className="truncate text-foreground">{value}</span>
		</div>
	);
}

function isActiveSession(session: AuthSessionInfo) {
	if (session.revoked) return false;
	const expiresAt = new Date(session.refresh_expires_at).getTime();
	return !Number.isFinite(expiresAt) || expiresAt > Date.now();
}

function sortSessions(sessions: AuthSessionInfo[]) {
	return sessions.toSorted(
		(left, right) =>
			new Date(right.last_seen_at).getTime() -
			new Date(left.last_seen_at).getTime(),
	);
}

function findLatestDate(values: string[]) {
	const timestamp = values.reduce((latest, value) => {
		const next = new Date(value).getTime();
		return Number.isFinite(next) && next > latest ? next : latest;
	}, 0);
	return timestamp > 0 ? new Date(timestamp).toISOString() : null;
}

function findNearestFutureDate(values: string[]) {
	const now = Date.now();
	const timestamp = values.reduce((nearest, value) => {
		const next = new Date(value).getTime();
		if (!Number.isFinite(next) || next <= now) return nearest;
		return nearest === 0 || next < nearest ? next : nearest;
	}, 0);
	return timestamp > 0 ? new Date(timestamp).toISOString() : null;
}

function formatRelativeDateTime(value: string) {
	const date = new Date(value);
	const timestamp = date.getTime();
	if (!Number.isFinite(timestamp)) return value;
	const differenceSeconds = Math.round((timestamp - Date.now()) / 1000);
	const absoluteSeconds = Math.abs(differenceSeconds);

	if (absoluteSeconds < 60)
		return RELATIVE_TIME_FORMATTER.format(differenceSeconds, "second");
	if (absoluteSeconds < 3600) {
		return RELATIVE_TIME_FORMATTER.format(
			Math.round(differenceSeconds / 60),
			"minute",
		);
	}
	if (absoluteSeconds < 86400) {
		return RELATIVE_TIME_FORMATTER.format(
			Math.round(differenceSeconds / 3600),
			"hour",
		);
	}
	return RELATIVE_TIME_FORMATTER.format(
		Math.round(differenceSeconds / 86400),
		"day",
	);
}
