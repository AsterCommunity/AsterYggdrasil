import {
	type FormEvent,
	type ReactNode,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import {
	ADMIN_TABLE_MONO_TEXT_CLASS,
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminTableCell,
	AdminTableHead,
	AdminTableHeader,
	AdminTableRow,
} from "@/components/common/AdminTable";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { DateTimeText } from "@/components/common/DateTimeText";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
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
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	parseOffsetSearchParam,
	parsePageSizeOption,
	parsePageSizeSearchParam,
} from "@/lib/pagination";
import { cn } from "@/lib/utils";
import { adminYggdrasilSessionForwardService } from "@/services/adminService";
import type {
	AdminYggdrasilSessionForwardServerInfo,
	CreateYggdrasilSessionForwardServerRequest,
	UpdateYggdrasilSessionForwardServerRequest,
	YggdrasilSessionForwardServerSortBy,
} from "@/types/api";

const PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
const DEFAULT_PAGE_SIZE = 20 as const;
const DIALOG_EXIT_ANIMATION_MS = 160;
const SORT_OPTIONS = [
	"call_order",
	"id",
] as const satisfies readonly YggdrasilSessionForwardServerSortBy[];
const DEFAULT_SORT = "call_order" satisfies ForwardSort;

type ForwardSort = YggdrasilSessionForwardServerSortBy;

type ForwardForm = {
	baseUrl: string;
	displayName: string;
	enabled: boolean;
	priority: string;
	textureForwardEnabled: boolean;
	timeoutMs: string;
	weight: string;
};

function emptyForm(): ForwardForm {
	return {
		baseUrl: "",
		displayName: "",
		enabled: true,
		priority: "100",
		textureForwardEnabled: false,
		timeoutMs: "1500",
		weight: "1",
	};
}

function formFromServer(
	server: AdminYggdrasilSessionForwardServerInfo,
): ForwardForm {
	return {
		baseUrl: server.base_url ?? "",
		displayName: server.display_name,
		enabled: server.enabled,
		priority: String(server.priority),
		textureForwardEnabled: server.texture_forward_enabled,
		timeoutMs: String(server.timeout_ms),
		weight: String(server.weight),
	};
}

function numberFromField(value: string, fallback: number) {
	const parsed = Number.parseInt(value, 10);
	return Number.isFinite(parsed) ? parsed : fallback;
}

function parseForwardSort(value: string | null): ForwardSort {
	return SORT_OPTIONS.includes(value as ForwardSort)
		? (value as ForwardSort)
		: DEFAULT_SORT;
}

function createPayload(
	form: ForwardForm,
): CreateYggdrasilSessionForwardServerRequest {
	return {
		base_url: form.baseUrl.trim(),
		display_name: form.displayName.trim(),
		enabled: form.enabled,
		priority: numberFromField(form.priority, 100),
		texture_forward_enabled: form.textureForwardEnabled,
		timeout_ms: numberFromField(form.timeoutMs, 1500),
		weight: numberFromField(form.weight, 1),
	};
}

function updatePayload(
	form: ForwardForm,
	server: AdminYggdrasilSessionForwardServerInfo,
): UpdateYggdrasilSessionForwardServerRequest {
	return {
		...(form.displayName.trim() !== server.display_name
			? { display_name: form.displayName.trim() }
			: {}),
		...(!server.local && form.baseUrl.trim() !== (server.base_url ?? "")
			? { base_url: form.baseUrl.trim() }
			: {}),
		enabled: form.enabled,
		priority: numberFromField(form.priority, server.priority),
		texture_forward_enabled: server.local ? false : form.textureForwardEnabled,
		timeout_ms: numberFromField(form.timeoutMs, server.timeout_ms),
		weight: numberFromField(form.weight, server.weight),
	};
}

function providerLabelKey(server: AdminYggdrasilSessionForwardServerInfo) {
	return server.provider_kind === "local"
		? "admin.yggdrasilForwarding.provider.local"
		: "admin.yggdrasilForwarding.provider.remote";
}

function isMojangTestingServer(server: AdminYggdrasilSessionForwardServerInfo) {
	const endpointKind = (server as { endpoint_kind?: string }).endpoint_kind;
	return endpointKind === "mojang_session" || server.display_name === "Mojang";
}

export default function AdminYggdrasilForwardingPage() {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();
	usePageTitle(t("admin.yggdrasilForwarding.title"));

	const offset = parseOffsetSearchParam(searchParams.get("offset"));
	const pageSize = parsePageSizeSearchParam(
		searchParams.get("pageSize"),
		PAGE_SIZE_OPTIONS,
		DEFAULT_PAGE_SIZE,
	);
	const sort = parseForwardSort(searchParams.get("sort_by"));
	const currentPage = Math.floor(offset / pageSize) + 1;
	const [loading, setLoading] = useState(true);
	const [servers, setServers] = useState<
		AdminYggdrasilSessionForwardServerInfo[]
	>([]);
	const [total, setTotal] = useState(0);
	const [dialogOpen, setDialogOpen] = useState(false);
	const [editingServer, setEditingServer] =
		useState<AdminYggdrasilSessionForwardServerInfo | null>(null);
	const [deletingServer, setDeletingServer] =
		useState<AdminYggdrasilSessionForwardServerInfo | null>(null);
	const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
	const [form, setForm] = useState<ForwardForm>(() => emptyForm());
	const [submitting, setSubmitting] = useState(false);
	const [updatingId, setUpdatingId] = useState<number | null>(null);
	const editDialogCleanupRef = useRef<ReturnType<typeof setTimeout> | null>(
		null,
	);
	const deleteDialogCleanupRef = useRef<ReturnType<typeof setTimeout> | null>(
		null,
	);
	const totalPages = Math.max(1, Math.ceil(total / pageSize));
	const pageSizeOptions = PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));
	const sortOptions = SORT_OPTIONS.map((value) => ({
		label: t(`admin.yggdrasilForwarding.sort.${value}`),
		value,
	}));
	const selectedSortLabel =
		sortOptions.find((option) => option.value === sort)?.label ??
		t(`admin.yggdrasilForwarding.sort.${DEFAULT_SORT}`);

	const updatePagination = useCallback(
		({
			offset: nextOffset = offset,
			pageSize: nextPageSize = pageSize,
			sort: nextSort = sort,
		}: {
			offset?: number;
			pageSize?: (typeof PAGE_SIZE_OPTIONS)[number];
			sort?: ForwardSort;
		}) => {
			const next = new URLSearchParams(searchParams);
			if (nextOffset > 0) next.set("offset", String(nextOffset));
			else next.delete("offset");
			if (nextPageSize !== DEFAULT_PAGE_SIZE) {
				next.set("pageSize", String(nextPageSize));
			} else {
				next.delete("pageSize");
			}
			if (nextSort !== DEFAULT_SORT) next.set("sort_by", nextSort);
			else next.delete("sort_by");
			if (next.toString() !== searchParams.toString()) {
				setSearchParams(next, { replace: true });
			}
		},
		[offset, pageSize, searchParams, setSearchParams, sort],
	);

	const loadServers = useCallback(async () => {
		try {
			setLoading(true);
			const page = await adminYggdrasilSessionForwardService.list({
				limit: pageSize,
				offset,
				sort_by: sort,
			});
			if (page.items.length === 0 && page.total > 0 && offset > 0) {
				updatePagination({
					offset: Math.max(
						0,
						Math.floor((page.total - 1) / pageSize) * pageSize,
					),
				});
				return;
			}
			setServers(page.items);
			setTotal(page.total);
		} catch (error) {
			handleApiError(error);
		} finally {
			setLoading(false);
		}
	}, [offset, pageSize, sort, updatePagination]);

	useEffect(() => {
		void loadServers();
	}, [loadServers]);

	useEffect(
		() => () => {
			if (editDialogCleanupRef.current) {
				clearTimeout(editDialogCleanupRef.current);
			}
			if (deleteDialogCleanupRef.current) {
				clearTimeout(deleteDialogCleanupRef.current);
			}
		},
		[],
	);

	function cancelEditDialogCleanup() {
		if (!editDialogCleanupRef.current) return;
		clearTimeout(editDialogCleanupRef.current);
		editDialogCleanupRef.current = null;
	}

	function cancelDeleteDialogCleanup() {
		if (!deleteDialogCleanupRef.current) return;
		clearTimeout(deleteDialogCleanupRef.current);
		deleteDialogCleanupRef.current = null;
	}

	function closeForwardServerDialog() {
		setDialogOpen(false);
		cancelEditDialogCleanup();
		editDialogCleanupRef.current = setTimeout(() => {
			setEditingServer(null);
			editDialogCleanupRef.current = null;
		}, DIALOG_EXIT_ANIMATION_MS);
	}

	function closeDeleteDialog() {
		setDeleteDialogOpen(false);
		cancelDeleteDialogCleanup();
		deleteDialogCleanupRef.current = setTimeout(() => {
			setDeletingServer(null);
			deleteDialogCleanupRef.current = null;
		}, DIALOG_EXIT_ANIMATION_MS);
	}

	function openCreate() {
		cancelEditDialogCleanup();
		setEditingServer(null);
		setForm(emptyForm());
		setDialogOpen(true);
	}

	function openEdit(server: AdminYggdrasilSessionForwardServerInfo) {
		cancelEditDialogCleanup();
		setEditingServer(server);
		setForm(formFromServer(server));
		setDialogOpen(true);
	}

	function openDelete(server: AdminYggdrasilSessionForwardServerInfo) {
		cancelDeleteDialogCleanup();
		setDeletingServer(server);
		setDeleteDialogOpen(true);
	}

	function setField<K extends keyof ForwardForm>(
		key: K,
		value: ForwardForm[K],
	) {
		setForm((current) => ({ ...current, [key]: value }));
	}

	async function submitForm(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!form.displayName.trim()) return;
		if (!editingServer && !form.baseUrl.trim()) return;

		try {
			setSubmitting(true);
			if (editingServer) {
				await adminYggdrasilSessionForwardService.update(
					editingServer.id,
					updatePayload(form, editingServer),
				);
				toast.success(t("admin.yggdrasilForwarding.updateSuccess"));
				await loadServers();
			} else {
				await adminYggdrasilSessionForwardService.create(createPayload(form));
				toast.success(t("admin.yggdrasilForwarding.createSuccess"));
				await loadServers();
			}
			closeForwardServerDialog();
		} catch (error) {
			handleApiError(error);
		} finally {
			setSubmitting(false);
		}
	}

	async function patchServer(
		server: AdminYggdrasilSessionForwardServerInfo,
		patch: UpdateYggdrasilSessionForwardServerRequest,
	) {
		try {
			setUpdatingId(server.id);
			await adminYggdrasilSessionForwardService.update(server.id, patch);
			toast.success(t("admin.yggdrasilForwarding.updateSuccess"));
			await loadServers();
		} catch (error) {
			handleApiError(error);
		} finally {
			setUpdatingId(null);
		}
	}

	async function deleteServer() {
		if (!deletingServer) return;
		try {
			setUpdatingId(deletingServer.id);
			await adminYggdrasilSessionForwardService.delete(deletingServer.id);
			closeDeleteDialog();
			toast.success(t("admin.yggdrasilForwarding.deleteSuccess"));
			await loadServers();
		} catch (error) {
			handleApiError(error);
		} finally {
			setUpdatingId(null);
		}
	}

	const headerRow = useMemo(
		() => (
			<AdminTableHeader>
				<AdminTableRow>
					<AdminTableHead>
						{t("admin.yggdrasilForwarding.server")}
					</AdminTableHead>
					<AdminTableHead>
						{t("admin.yggdrasilForwarding.order")}
					</AdminTableHead>
					<AdminTableHead>
						{t("admin.yggdrasilForwarding.state")}
					</AdminTableHead>
					<AdminTableHead>
						{t("admin.yggdrasilForwarding.lastCheck")}
					</AdminTableHead>
					<AdminTableHead className="w-[1%] text-right">
						{t("admin.yggdrasilForwarding.actions")}
					</AdminTableHead>
				</AdminTableRow>
			</AdminTableHeader>
		),
		[t],
	);

	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				total={total}
				currentPage={currentPage}
				totalPages={totalPages}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={offset === 0}
				nextDisabled={offset + pageSize >= total}
				onPrevious={() =>
					updatePagination({ offset: Math.max(0, offset - pageSize) })
				}
				onNext={() => updatePagination({ offset: offset + pageSize })}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(value, PAGE_SIZE_OPTIONS);
					if (next == null) return;
					updatePagination({ offset: 0, pageSize: next });
				}}
			/>
		),
		[
			currentPage,
			offset,
			pageSize,
			pageSizeOptions,
			total,
			totalPages,
			updatePagination,
		],
	);

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.yggdrasilForwarding.title")}
				description={t("admin.yggdrasilForwarding.description")}
				actions={
					<>
						<Select
							value={sort}
							onValueChange={(value) =>
								updatePagination({
									offset: 0,
									sort: parseForwardSort(value),
								})
							}
						>
							<SelectTrigger
								width="fit"
								aria-label={t("admin.yggdrasilForwarding.sort.label")}
							>
								<Icon name="Shuffle" className="size-4 text-muted-foreground" />
								<SelectValue>{selectedSortLabel}</SelectValue>
							</SelectTrigger>
							<SelectContent align="end">
								{sortOptions.map((option) => (
									<SelectItem key={option.value} value={option.value}>
										{option.label}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
						<Button type="button" size="sm" onClick={openCreate}>
							<Icon name="Plus" className="mr-2 size-4" />
							{t("admin.yggdrasilForwarding.createAction")}
						</Button>
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={loading}
							onClick={() => void loadServers()}
						>
							<Icon
								name={loading ? "Spinner" : "ArrowsClockwise"}
								className={cn("mr-2 size-4", loading && "animate-spin")}
							/>
							{t("common.refresh")}
						</Button>
					</>
				}
			/>

			<TooltipProvider delay={100}>
				<AdminTableList
					loading={loading}
					items={servers}
					columns={5}
					rows={6}
					emptyIcon={<Icon name="Shuffle" className="size-5" />}
					emptyTitle={t("admin.yggdrasilForwarding.emptyTitle")}
					emptyDescription={t("admin.yggdrasilForwarding.emptyDescription")}
					emptyAction={
						<Button type="button" size="sm" onClick={openCreate}>
							<Icon name="Plus" className="mr-2 size-4" />
							{t("admin.yggdrasilForwarding.createAction")}
						</Button>
					}
					headerRow={headerRow}
					pagination={pagination}
					renderRow={(server) => (
						<ForwardServerRow
							key={server.id}
							server={server}
							updating={updatingId === server.id}
							onEdit={openEdit}
							onDelete={openDelete}
							onPatch={(patch) => void patchServer(server, patch)}
						/>
					)}
				/>
			</TooltipProvider>

			<ForwardServerDialog
				form={form}
				open={dialogOpen}
				server={editingServer}
				submitting={submitting}
				onOpenChange={(open) => {
					if (open) {
						cancelEditDialogCleanup();
						setDialogOpen(true);
					} else {
						closeForwardServerDialog();
					}
				}}
				onSubmit={submitForm}
				onFieldChange={setField}
			/>

			<ConfirmDialog
				open={deleteDialogOpen}
				loading={updatingId === deletingServer?.id}
				title={t("admin.yggdrasilForwarding.deleteTitle")}
				description={t("admin.yggdrasilForwarding.deleteDescription", {
					name: deletingServer?.display_name ?? "",
				})}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("common.delete")}
				variant="destructive"
				onOpenChange={(open) => {
					if (open) {
						cancelDeleteDialogCleanup();
						setDeleteDialogOpen(true);
					} else {
						closeDeleteDialog();
					}
				}}
				onConfirm={() => void deleteServer()}
			/>
		</AdminPageShell>
	);
}

function ForwardServerRow({
	onDelete,
	onEdit,
	onPatch,
	server,
	updating,
}: {
	onDelete: (server: AdminYggdrasilSessionForwardServerInfo) => void;
	onEdit: (server: AdminYggdrasilSessionForwardServerInfo) => void;
	onPatch: (patch: UpdateYggdrasilSessionForwardServerRequest) => void;
	server: AdminYggdrasilSessionForwardServerInfo;
	updating: boolean;
}) {
	const { t } = useTranslation();
	const healthKey = server.last_failure_at
		? "admin.yggdrasilForwarding.health.failed"
		: server.last_success_at
			? "admin.yggdrasilForwarding.health.ok"
			: "admin.yggdrasilForwarding.health.pending";

	return (
		<AdminTableRow>
			<AdminTableCell>
				<div className="min-w-0">
					<div className="flex min-w-0 flex-wrap items-center gap-2">
						<span className="truncate font-medium text-foreground">
							{server.display_name}
						</span>
						<Badge
							variant="outline"
							className={cn(
								"rounded-md",
								server.enabled
									? "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-200"
									: "border-muted-foreground/30 bg-muted/50 text-muted-foreground",
							)}
						>
							{server.enabled
								? t("admin.common.enabled")
								: t("admin.common.disabled")}
						</Badge>
						<Badge variant={server.local ? "secondary" : "outline"}>
							{t(providerLabelKey(server))}
						</Badge>
						{isMojangTestingServer(server) ? (
							<Badge variant="secondary">
								{t("admin.yggdrasilForwarding.testing")}
							</Badge>
						) : null}
					</div>
					<div
						className={cn(
							"mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1",
							ADMIN_TABLE_MUTED_TEXT_CLASS,
						)}
					>
						<span className={ADMIN_TABLE_MONO_TEXT_CLASS}>#{server.id}</span>
						<span
							className={cn(
								"max-w-[28rem] truncate",
								ADMIN_TABLE_MONO_TEXT_CLASS,
							)}
							title={server.base_url ?? undefined}
						>
							{server.base_url ?? t("admin.yggdrasilForwarding.localBaseUrl")}
						</span>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="flex min-w-[9rem] flex-wrap gap-1.5">
					<Badge
						variant="outline"
						className="rounded-md border-sky-500/30 bg-sky-500/10 font-mono text-sky-700 dark:text-sky-200"
					>
						{t("admin.yggdrasilForwarding.priority")}: {server.priority}
					</Badge>
					<Badge
						variant="outline"
						className="rounded-md border-violet-500/30 bg-violet-500/10 font-mono text-violet-700 dark:text-violet-200"
					>
						{t("admin.yggdrasilForwarding.weight")}: {server.weight}
					</Badge>
					<Badge
						variant="outline"
						className="rounded-md border-amber-500/30 bg-amber-500/10 font-mono text-amber-700 dark:text-amber-200"
					>
						{t("admin.yggdrasilForwarding.timeout")}: {server.timeout_ms} ms
					</Badge>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="flex flex-col gap-2">
					<div className="flex items-center gap-2 text-xs">
						<Switch
							size="sm"
							checked={server.enabled}
							disabled={updating}
							onCheckedChange={(enabled) => onPatch({ enabled })}
							aria-label={t("admin.yggdrasilForwarding.enabled")}
						/>
						<span>
							{server.enabled
								? t("admin.common.enabled")
								: t("admin.common.disabled")}
						</span>
					</div>
					<div className="flex items-center gap-2 text-xs">
						<Switch
							size="sm"
							checked={server.texture_forward_enabled}
							disabled={updating || server.local}
							onCheckedChange={(texture_forward_enabled) =>
								onPatch({ texture_forward_enabled })
							}
							aria-label={t("admin.yggdrasilForwarding.textureForward")}
						/>
						<span>{t("admin.yggdrasilForwarding.textureForward")}</span>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="min-w-[10rem]">
					<div className="text-sm font-medium">{t(healthKey)}</div>
					<DateTimeText
						value={server.last_checked_at}
						className={ADMIN_TABLE_MUTED_TEXT_CLASS}
						fallback={t("admin.yggdrasilForwarding.neverChecked")}
					/>
					{server.last_failure_message ? (
						<div
							className="mt-1 max-w-[16rem] truncate text-xs text-destructive"
							title={server.last_failure_message}
						>
							{server.last_failure_message}
						</div>
					) : null}
				</div>
			</AdminTableCell>
			<AdminTableCell className="text-right">
				<div className="flex justify-end gap-1">
					<ActionTooltipButton
						label={t("admin.yggdrasilForwarding.editAction")}
						onClick={() => onEdit(server)}
					>
						<Icon name="PencilSimple" className="size-4" />
					</ActionTooltipButton>
					<ActionTooltipButton
						label={t("admin.yggdrasilForwarding.deleteAction")}
						disabled={!server.deletable || updating}
						className="text-destructive hover:text-destructive"
						onClick={() => onDelete(server)}
					>
						<Icon
							name={updating ? "Spinner" : "Trash"}
							className={cn("size-4", updating && "animate-spin")}
						/>
					</ActionTooltipButton>
				</div>
			</AdminTableCell>
		</AdminTableRow>
	);
}

function ActionTooltipButton({
	children,
	className,
	disabled,
	label,
	onClick,
}: {
	children: ReactNode;
	className?: string;
	disabled?: boolean;
	label: string;
	onClick: () => void;
}) {
	return (
		<Tooltip>
			<TooltipTrigger render={<span className="inline-flex size-8" />}>
				<Button
					type="button"
					variant="ghost"
					size="icon"
					aria-label={label}
					disabled={disabled}
					className={cn("text-muted-foreground", className)}
					onClick={onClick}
				>
					{children}
				</Button>
			</TooltipTrigger>
			<TooltipContent>{label}</TooltipContent>
		</Tooltip>
	);
}

function ForwardServerDialog({
	form,
	onFieldChange,
	onOpenChange,
	onSubmit,
	open,
	server,
	submitting,
}: {
	form: ForwardForm;
	onFieldChange: <K extends keyof ForwardForm>(
		key: K,
		value: ForwardForm[K],
	) => void;
	onOpenChange: (open: boolean) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	open: boolean;
	server: AdminYggdrasilSessionForwardServerInfo | null;
	submitting: boolean;
}) {
	const { t } = useTranslation();
	const editingLocal = server?.local === true;
	const editing = server !== null;

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent keepMounted className="sm:max-w-2xl">
				<form onSubmit={onSubmit} className="grid min-h-0 gap-4">
					<DialogHeader>
						<DialogTitle>
							{editing
								? t("admin.yggdrasilForwarding.editTitle")
								: t("admin.yggdrasilForwarding.createTitle")}
						</DialogTitle>
						<DialogDescription>
							{editingLocal
								? t("admin.yggdrasilForwarding.localEditDescription")
								: t("admin.yggdrasilForwarding.dialogDescription")}
						</DialogDescription>
					</DialogHeader>

					<div className="grid gap-4 sm:grid-cols-2">
						<div className="grid gap-1.5">
							<Label htmlFor="forward-display-name">
								{t("admin.yggdrasilForwarding.displayName")}
							</Label>
							<Input
								id="forward-display-name"
								value={form.displayName}
								required
								maxLength={128}
								onChange={(event) =>
									onFieldChange("displayName", event.currentTarget.value)
								}
							/>
						</div>
						<div className="grid gap-1.5">
							<Label htmlFor="forward-base-url">
								{t("admin.yggdrasilForwarding.baseUrl")}
							</Label>
							<Input
								id="forward-base-url"
								type="url"
								value={form.baseUrl}
								required={!editingLocal}
								disabled={editingLocal}
								placeholder="https://auth.example.com/yggdrasil"
								onChange={(event) =>
									onFieldChange("baseUrl", event.currentTarget.value)
								}
							/>
						</div>
						<NumberField
							id="forward-priority"
							label={t("admin.yggdrasilForwarding.priority")}
							value={form.priority}
							min={-10000}
							max={10000}
							onChange={(value) => onFieldChange("priority", value)}
						/>
						<NumberField
							id="forward-weight"
							label={t("admin.yggdrasilForwarding.weight")}
							value={form.weight}
							min={1}
							max={1000}
							onChange={(value) => onFieldChange("weight", value)}
						/>
						<NumberField
							id="forward-timeout"
							label={t("admin.yggdrasilForwarding.timeoutMs")}
							value={form.timeoutMs}
							min={100}
							max={10000}
							onChange={(value) => onFieldChange("timeoutMs", value)}
						/>
						<div className="flex flex-col justify-end gap-3 rounded-lg border border-border/70 bg-muted/20 px-3 py-2">
							<div className="flex items-center justify-between gap-3">
								<span className="text-sm font-medium">
									{t("admin.yggdrasilForwarding.enabled")}
								</span>
								<Switch
									checked={form.enabled}
									aria-label={t("admin.yggdrasilForwarding.enabled")}
									onCheckedChange={(value) => onFieldChange("enabled", value)}
								/>
							</div>
							<div className="flex items-center justify-between gap-3">
								<span className="text-sm font-medium">
									{t("admin.yggdrasilForwarding.textureForward")}
								</span>
								<Switch
									checked={form.textureForwardEnabled}
									disabled={editingLocal}
									aria-label={t("admin.yggdrasilForwarding.textureForward")}
									onCheckedChange={(value) =>
										onFieldChange("textureForwardEnabled", value)
									}
								/>
							</div>
						</div>
					</div>

					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={submitting}
							onClick={() => onOpenChange(false)}
						>
							{t("common.cancel")}
						</Button>
						<Button type="submit" disabled={submitting}>
							{submitting ? (
								<Icon name="Spinner" className="mr-2 size-4 animate-spin" />
							) : null}
							{editing
								? t("admin.yggdrasilForwarding.saveAction")
								: t("admin.yggdrasilForwarding.createAction")}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}

function NumberField({
	id,
	label,
	max,
	min,
	onChange,
	value,
}: {
	id: string;
	label: string;
	max: number;
	min: number;
	onChange: (value: string) => void;
	value: string;
}) {
	return (
		<div className="grid gap-1.5">
			<Label htmlFor={id}>{label}</Label>
			<Input
				id={id}
				type="number"
				value={value}
				min={min}
				max={max}
				onChange={(event) => onChange(event.currentTarget.value)}
			/>
		</div>
	);
}
