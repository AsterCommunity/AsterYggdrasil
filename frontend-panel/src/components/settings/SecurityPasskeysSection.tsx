import { useCallback, useMemo, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { DateTimeText } from "@/components/common/DateTimeText";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useCursorList } from "@/hooks/useCursorList";
import { usePendingAction } from "@/hooks/usePendingAction";
import {
	createPasskeyCredential,
	isWebAuthnSupported,
	WebAuthnCancelledError,
	WebAuthnUnsupportedError,
} from "@/lib/webauthn";
import { authService, type PasskeyInfo } from "@/services/authService";
import { formatUnknownError } from "@/services/http";

const PASSKEY_PAGE_SIZE = 20;

type PasskeysState = {
	busyIds: Set<number>;
	editingId: number | null;
	editingName: string;
	name: string;
};

type PasskeysAction =
	| { type: "set_name"; value: string }
	| { type: "created" }
	| { type: "start_edit"; id: number; name: string }
	| { type: "cancel_edit" }
	| { type: "set_editing_name"; value: string }
	| { type: "set_busy"; id: number; value: boolean };

const initialPasskeysState: PasskeysState = {
	busyIds: new Set(),
	editingId: null,
	editingName: "",
	name: "",
};

function passkeysReducer(
	state: PasskeysState,
	action: PasskeysAction,
): PasskeysState {
	switch (action.type) {
		case "set_name":
			return { ...state, name: action.value };
		case "created":
			return { ...state, name: "" };
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
	const supported = useMemo(() => isWebAuthnSupported(), []);
	const { pending: creating, runWithPending: runCreate } = usePendingAction();

	const loadPasskeyPage = useCallback(
		async ({ cursor }: { cursor: { id: number; value: string } | null }) => {
			const page = await authService.listPasskeysPage({
				limit: PASSKEY_PAGE_SIZE,
				after_created_at: cursor?.value,
				after_id: cursor?.id,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[],
	);
	const list = useCursorList<PasskeyInfo, { id: number; value: string }>({
		deps: [],
		loadPage: loadPasskeyPage,
		onError: (error) => toast.error(formatUnknownError(error)),
	});

	async function createPasskey() {
		if (!supported) {
			toast.error(t("personalSettings.passkeysUnsupported"));
			return;
		}
		await runCreate(async () => {
			try {
				const start = await authService.startPasskeyRegistration({
					name: state.name.trim() || null,
				});
				const credential = await createPasskeyCredential(start.public_key);
				await authService.finishPasskeyRegistration(
					start.flow_id,
					credential,
					state.name.trim() || null,
				);
				dispatch({ type: "created" });
				toast.success(t("personalSettings.passkeysCreated"));
				await list.reloadFirstPage();
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
			}
		});
	}

	async function saveRename(id: number) {
		const finalName = state.editingName.trim();
		if (!finalName) return;
		dispatch({ type: "set_busy", id, value: true });
		try {
			const updated = await authService.renamePasskey(id, { name: finalName });
			list.setItems((current) =>
				current.map((passkey) =>
					passkey.id === updated.id ? updated : passkey,
				),
			);
			dispatch({ type: "cancel_edit" });
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
			toast.success(t("personalSettings.passkeysDeleted"));
			await list.reload();
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
					onClick={() => void list.reload()}
				>
					<Icon name="ArrowClockwise" className="mr-2 size-4" />
					{t("common.refresh")}
				</Button>
			</div>

			<div className="mt-4 grid gap-3 rounded-lg border border-border/70 bg-background/50 p-4 dark:border-white/10">
				<div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_auto] md:items-center">
					<Input
						value={state.name}
						disabled={creating}
						maxLength={128}
						placeholder={t("personalSettings.passkeysNamePlaceholder")}
						onChange={(event) =>
							dispatch({ type: "set_name", value: event.currentTarget.value })
						}
					/>
					<Button
						type="button"
						disabled={creating || !supported}
						onClick={() => void createPasskey()}
					>
						<Icon
							name={creating ? "Spinner" : "Plus"}
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
				{list.loading ? (
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
				) : list.items.length === 0 ? (
					<div className="rounded-lg border border-dashed border-border/70 px-4 py-6 text-sm text-muted-foreground">
						{t("personalSettings.passkeysEmpty")}
					</div>
				) : (
					list.items.map((passkey) => {
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
					currentPage={list.currentPage}
					nextDisabled={list.nextPageDisabled}
					onNext={list.goNext}
					onPageSizeChange={() => {}}
					onPrevious={list.goPrevious}
					pageSize={String(PASSKEY_PAGE_SIZE)}
					pageSizeOptions={[
						{
							label: t("admin.pagination.pageSizeOption", {
								count: PASSKEY_PAGE_SIZE,
							}),
							value: String(PASSKEY_PAGE_SIZE),
						},
					]}
					prevDisabled={list.prevPageDisabled}
					total={list.total}
					totalPages={list.totalPages}
				/>
			</div>
		</div>
	);
}
