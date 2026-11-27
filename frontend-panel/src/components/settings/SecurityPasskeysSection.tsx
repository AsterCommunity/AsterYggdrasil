import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { DateTimeText } from "@/components/common/DateTimeText";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import {
	createPasskeyCredential,
	isWebAuthnSupported,
	WebAuthnCancelledError,
	WebAuthnUnsupportedError,
} from "@/lib/webauthn";
import { authService, type PasskeyInfo } from "@/services/authService";
import { formatUnknownError } from "@/services/http";
import type { DateTimeIdCursor } from "@/types/api";

const PASSKEY_PAGE_SIZE = 20;

type PasskeysState = {
	busyIds: Set<number>;
	creating: boolean;
	editingId: number | null;
	editingName: string;
	loading: boolean;
	name: string;
	passkeys: PasskeyInfo[];
	total: number;
};

type PasskeysAction =
	| { type: "set_loading"; value: boolean }
	| { type: "set_passkeys"; value: PasskeyInfo[]; total: number }
	| { type: "set_creating"; value: boolean }
	| { type: "set_name"; value: string }
	| { type: "created"; value: PasskeyInfo }
	| { type: "start_edit"; id: number; name: string }
	| { type: "cancel_edit" }
	| { type: "set_editing_name"; value: string }
	| { type: "renamed"; value: PasskeyInfo }
	| { type: "deleted"; id: number }
	| { type: "set_busy"; id: number; value: boolean };

const initialPasskeysState: PasskeysState = {
	busyIds: new Set(),
	creating: false,
	editingId: null,
	editingName: "",
	loading: false,
	name: "",
	passkeys: [],
	total: 0,
};

function passkeysReducer(
	state: PasskeysState,
	action: PasskeysAction,
): PasskeysState {
	switch (action.type) {
		case "set_loading":
			return { ...state, loading: action.value };
		case "set_passkeys":
			return { ...state, passkeys: action.value, total: action.total };
		case "set_creating":
			return { ...state, creating: action.value };
		case "set_name":
			return { ...state, name: action.value };
		case "created":
			return {
				...state,
				name: "",
				total: state.total + 1,
				passkeys: [action.value, ...state.passkeys],
			};
		case "start_edit":
			return {
				...state,
				editingId: action.id,
				editingName: action.name,
			};
		case "cancel_edit":
			return { ...state, editingId: null, editingName: "" };
		case "set_editing_name":
			return { ...state, editingName: action.value };
		case "renamed":
			return {
				...state,
				editingId: null,
				editingName: "",
				passkeys: state.passkeys.map((passkey) =>
					passkey.id === action.value.id ? action.value : passkey,
				),
			};
		case "deleted":
			return {
				...state,
				passkeys: state.passkeys.filter((passkey) => passkey.id !== action.id),
				total: Math.max(0, state.total - 1),
			};
		case "set_busy": {
			const busyIds = new Set(state.busyIds);
			if (action.value) busyIds.add(action.id);
			else busyIds.delete(action.id);
			return { ...state, busyIds };
		}
	}
}

export function SecurityPasskeysSection() {
	const { t } = useTranslation();
	const [state, dispatch] = useReducer(passkeysReducer, initialPasskeysState);
	const [cursorStack, setCursorStack] = useState<DateTimeIdCursor[]>([]);
	const [nextCursor, setNextCursor] = useState<DateTimeIdCursor | null>(null);
	const supported = useMemo(() => isWebAuthnSupported(), []);

	const reload = useCallback(
		async (stack: DateTimeIdCursor[] = cursorStack) => {
			dispatch({ type: "set_loading", value: true });
			try {
				const cursor = stack.at(-1);
				const page = await authService.listPasskeysPage({
					limit: PASSKEY_PAGE_SIZE,
					after_created_at: cursor?.value,
					after_id: cursor?.id,
				});
				if (page.items.length === 0 && page.total > 0 && stack.length > 0) {
					setCursorStack((current) => current.slice(0, -1));
					setNextCursor(null);
					return;
				}
				dispatch({
					type: "set_passkeys",
					total: page.total,
					value: page.items,
				});
				setNextCursor(page.next_cursor ?? null);
			} catch (error) {
				toast.error(formatUnknownError(error));
			} finally {
				dispatch({ type: "set_loading", value: false });
			}
		},
		[cursorStack],
	);

	useEffect(() => {
		void reload();
	}, [reload]);

	async function createPasskey() {
		if (!supported) {
			toast.error(t("personalSettings.passkeysUnsupported"));
			return;
		}
		dispatch({ type: "set_creating", value: true });
		try {
			const start = await authService.startPasskeyRegistration({
				name: state.name.trim() || null,
			});
			const credential = await createPasskeyCredential(start.public_key);
			const created = await authService.finishPasskeyRegistration(
				start.flow_id,
				credential,
				state.name.trim() || null,
			);
			setCursorStack([]);
			setNextCursor(null);
			dispatch({ type: "created", value: created });
			toast.success(t("personalSettings.passkeysCreated"));
			await reload([]);
		} catch (error) {
			if (error instanceof WebAuthnUnsupportedError) {
				toast.error(t("personalSettings.passkeysUnsupported"));
				return;
			}
			if (error instanceof WebAuthnCancelledError) {
				toast.error(t("personalSettings.passkeysCancelled"));
				return;
			}
			toast.error(formatUnknownError(error));
		} finally {
			dispatch({ type: "set_creating", value: false });
		}
	}

	async function saveRename(id: number) {
		const finalName = state.editingName.trim();
		if (!finalName) return;
		dispatch({ type: "set_busy", id, value: true });
		try {
			const updated = await authService.renamePasskey(id, { name: finalName });
			dispatch({ type: "renamed", value: updated });
			toast.success(t("personalSettings.passkeysRenamed"));
		} catch (error) {
			toast.error(formatUnknownError(error));
		} finally {
			dispatch({ type: "set_busy", id, value: false });
		}
	}

	async function deletePasskey(id: number) {
		dispatch({ type: "set_busy", id, value: true });
		try {
			await authService.deletePasskey(id);
			dispatch({ type: "deleted", id });
			toast.success(t("personalSettings.passkeysDeleted"));
			await reload();
		} catch (error) {
			toast.error(formatUnknownError(error));
		} finally {
			dispatch({ type: "set_busy", id, value: false });
		}
	}

	return (
		<div className="rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
			<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
				<div className="min-w-0">
					<h3 className="text-sm font-semibold">
						{t("personalSettings.passkeysTitle")}
					</h3>
					<p className="mt-1 text-xs leading-5 text-muted-foreground">
						{t("personalSettings.passkeysDescription")}
					</p>
				</div>
				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={() => void reload()}
				>
					<Icon name="ArrowClockwise" className="mr-2 size-4" />
					{t("common.refresh")}
				</Button>
			</div>

			<div className="mt-4 grid gap-3 rounded-lg border border-border/70 bg-background/50 p-4 dark:border-white/10">
				<div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_auto] md:items-center">
					<Input
						value={state.name}
						disabled={state.creating}
						maxLength={128}
						placeholder={t("personalSettings.passkeysNamePlaceholder")}
						onChange={(event) =>
							dispatch({ type: "set_name", value: event.currentTarget.value })
						}
					/>
					<Button
						type="button"
						disabled={state.creating || !supported}
						onClick={() => void createPasskey()}
					>
						<Icon
							name={state.creating ? "Spinner" : "Plus"}
							className="mr-2 size-4"
						/>
						{t("personalSettings.passkeysAdd")}
					</Button>
				</div>
				<p className="text-xs text-muted-foreground">
					{supported
						? t("personalSettings.passkeysHint")
						: t("personalSettings.passkeysUnsupported")}
				</p>
			</div>

			<div className="mt-4 space-y-2">
				{state.loading ? (
					<div
						className="rounded-lg border border-dashed border-border/70 px-4 py-4"
						role="status"
					>
						<span className="sr-only">{t("common.loading")}</span>
						<div className="space-y-3">
							<Skeleton className="h-11 w-full rounded-lg" />
							<Skeleton className="h-11 w-5/6 rounded-lg" />
						</div>
					</div>
				) : state.passkeys.length === 0 ? (
					<div className="rounded-lg border border-dashed border-border/70 px-4 py-6 text-sm text-muted-foreground">
						{t("personalSettings.passkeysEmpty")}
					</div>
				) : (
					state.passkeys.map((passkey) => {
						const busy = state.busyIds.has(passkey.id);
						const editing = state.editingId === passkey.id;
						return (
							<div
								key={passkey.id}
								className="grid gap-3 rounded-lg border border-border/70 bg-background/70 p-4 dark:border-white/10"
							>
								<div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
									<div className="min-w-0">
										{editing ? (
											<Input
												value={state.editingName}
												disabled={busy}
												maxLength={128}
												onChange={(event) =>
													dispatch({
														type: "set_editing_name",
														value: event.currentTarget.value,
													})
												}
											/>
										) : (
											<div className="truncate text-sm font-semibold">
												{passkey.name}
											</div>
										)}
										<div className="mt-1 text-xs text-muted-foreground">
											{t("personalSettings.passkeysLastUsed")}:{" "}
											<DateTimeText
												value={passkey.last_used_at}
												fallback={t("personalSettings.passkeysNeverUsed")}
											/>
										</div>
									</div>
									<div className="flex flex-wrap gap-2">
										{editing ? (
											<>
												<Button
													type="button"
													size="sm"
													disabled={busy || !state.editingName.trim()}
													onClick={() => void saveRename(passkey.id)}
												>
													<Icon name="Check" className="mr-2 size-4" />
													{t("common.save")}
												</Button>
												<Button
													type="button"
													size="sm"
													variant="outline"
													disabled={busy}
													onClick={() => dispatch({ type: "cancel_edit" })}
												>
													{t("common.cancel")}
												</Button>
											</>
										) : (
											<Button
												type="button"
												size="sm"
												variant="outline"
												disabled={busy}
												onClick={() =>
													dispatch({
														type: "start_edit",
														id: passkey.id,
														name: passkey.name,
													})
												}
											>
												<Icon name="PencilSimple" className="mr-2 size-4" />
												{t("personalSettings.passkeysRename")}
											</Button>
										)}
										<Button
											type="button"
											size="sm"
											variant="destructive"
											disabled={busy}
											onClick={() => void deletePasskey(passkey.id)}
										>
											<Icon name="Trash" className="mr-2 size-4" />
											{t("personalSettings.passkeysDelete")}
										</Button>
									</div>
								</div>
								<div className="grid gap-2 text-xs text-muted-foreground md:grid-cols-2">
									<div>
										{t("personalSettings.passkeysCreatedAt")}:{" "}
										<DateTimeText value={passkey.created_at} />
									</div>
									<div>
										{t("personalSettings.passkeysUpdated")}:{" "}
										<DateTimeText value={passkey.updated_at} />
									</div>
								</div>
							</div>
						);
					})
				)}
				<AdminOffsetPagination
					currentPage={cursorStack.length + 1}
					nextDisabled={!nextCursor}
					onNext={() => {
						if (!nextCursor) return;
						setCursorStack((current) => [...current, nextCursor]);
					}}
					onPageSizeChange={() => {}}
					onPrevious={() => setCursorStack((current) => current.slice(0, -1))}
					pageSize={String(PASSKEY_PAGE_SIZE)}
					pageSizeOptions={[
						{
							label: t("admin.pagination.pageSizeOption", {
								count: PASSKEY_PAGE_SIZE,
							}),
							value: String(PASSKEY_PAGE_SIZE),
						},
					]}
					prevDisabled={cursorStack.length === 0}
					total={state.total}
					totalPages={Math.max(cursorStack.length + (nextCursor ? 2 : 1), 1)}
				/>
			</div>
		</div>
	);
}
