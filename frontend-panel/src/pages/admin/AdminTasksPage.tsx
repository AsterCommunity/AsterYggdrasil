import type { FormEvent, ReactNode } from "react";
import { useCallback, useMemo, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { AdminTaskCleanupDialog } from "@/components/admin/admin-tasks-page/AdminTaskCleanupDialog";
import { AdminTaskFiltersToolbar } from "@/components/admin/admin-tasks-page/AdminTaskFiltersToolbar";
import {
	AdminTaskDetailDialog,
	AdminTaskTableHeader,
	AdminTaskTableRow,
} from "@/components/admin/admin-tasks-page/AdminTaskTable";
import { AdminTableList } from "@/components/common/AdminTableList";
import { DateTimeText } from "@/components/common/DateTimeText";
import { EmptyState } from "@/components/common/EmptyState";
import { RefreshButton } from "@/components/common/RefreshButton";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { handleApiError } from "@/hooks/useApiError";
import { useCursorList } from "@/hooks/useCursorList";
import {
	managedEnumField,
	managedPageSizeField,
	useManagedListQueryState,
} from "@/hooks/useManagedListQueryState";
import { usePageTitle } from "@/hooks/usePageTitle";
import { usePendingAction } from "@/hooks/usePendingAction";
import { usePendingId } from "@/hooks/usePendingId";
import { dateTimeLocalToIso } from "@/lib/form";
import { parsePageSizeOption } from "@/lib/pagination";
import { formatTaskStatusLabel } from "@/lib/tasks";
import { adminTaskService } from "@/services/adminService";
import type {
	AdminTaskCleanupRequest,
	BackgroundTaskStatus,
	TaskInfo,
} from "@/types/api";

const TASK_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
const DEFAULT_TASK_PAGE_SIZE = 20 as const;
const TASK_STATUS_FILTER_VALUES = [
	"pending",
	"processing",
	"retry",
	"succeeded",
	"failed",
	"canceled",
] as const satisfies readonly BackgroundTaskStatus[];
const TASK_TERMINAL_STATUS_FILTER_VALUES = [
	"succeeded",
	"failed",
	"canceled",
] as const satisfies readonly BackgroundTaskStatus[];
const DEFAULT_TASK_CLEANUP_LOOKBACK_HOURS = 24;

type TaskStatusFilter = "__all__" | BackgroundTaskStatus;
type TaskTerminalStatusFilter =
	| "__all__"
	| (typeof TASK_TERMINAL_STATUS_FILTER_VALUES)[number];
type ManagedTaskQuery = {
	pageSize: (typeof TASK_PAGE_SIZE_OPTIONS)[number];
	status: TaskStatusFilter;
};
type TaskCursor = { id: number; value: string };
type AdminTasksUiState = {
	cleanupDialogOpen: boolean;
	cleanupFinishedBefore: string;
	cleanupStatusFilter: TaskTerminalStatusFilter;
	detailDialogTaskId: number | null;
};
type AdminTasksUiAction =
	| { open: boolean; type: "set_cleanup_dialog_open" }
	| { taskId: number | null; type: "set_detail_dialog_task" }
	| { value: string; type: "set_cleanup_finished_before" }
	| { value: TaskTerminalStatusFilter; type: "set_cleanup_status_filter" }
	| { type: "reset_cleanup_conditions" };

function parseTaskTerminalStatus(
	value: string | null,
): TaskTerminalStatusFilter {
	return TASK_TERMINAL_STATUS_FILTER_VALUES.includes(
		value as (typeof TASK_TERMINAL_STATUS_FILTER_VALUES)[number],
	)
		? (value as TaskTerminalStatusFilter)
		: "__all__";
}

function defaultCleanupFinishedBeforeValue() {
	const date = new Date(
		Date.now() - DEFAULT_TASK_CLEANUP_LOOKBACK_HOURS * 60 * 60 * 1000,
	);
	const offset = date.getTimezoneOffset();
	const local = new Date(date.getTime() - offset * 60 * 1000);
	return local.toISOString().slice(0, 16);
}

function createInitialAdminTasksUiState(): AdminTasksUiState {
	return {
		cleanupDialogOpen: false,
		cleanupFinishedBefore: defaultCleanupFinishedBeforeValue(),
		cleanupStatusFilter: "__all__",
		detailDialogTaskId: null,
	};
}

function adminTasksUiReducer(
	state: AdminTasksUiState,
	action: AdminTasksUiAction,
): AdminTasksUiState {
	switch (action.type) {
		case "set_cleanup_dialog_open":
			return { ...state, cleanupDialogOpen: action.open };
		case "set_detail_dialog_task":
			return { ...state, detailDialogTaskId: action.taskId };
		case "set_cleanup_finished_before":
			return { ...state, cleanupFinishedBefore: action.value };
		case "set_cleanup_status_filter":
			return { ...state, cleanupStatusFilter: action.value };
		case "reset_cleanup_conditions":
			return {
				...state,
				cleanupFinishedBefore: defaultCleanupFinishedBeforeValue(),
				cleanupStatusFilter: "__all__",
			};
	}
}

function buildCleanupRequest(
	cleanupFinishedBefore: string,
	cleanupStatusFilter: TaskTerminalStatusFilter,
): AdminTaskCleanupRequest | null {
	const finishedBefore = dateTimeLocalToIso(cleanupFinishedBefore);
	if (finishedBefore == null) {
		return null;
	}
	return {
		finished_before: finishedBefore,
		...(cleanupStatusFilter !== "__all__"
			? { status: cleanupStatusFilter }
			: {}),
	};
}

function taskSourceLabel(
	t: ReturnType<typeof useTranslation>["t"],
	task: TaskInfo,
): ReactNode {
	if (task.creator) {
		return (
			<div className="min-w-0">
				<div className="truncate text-sm font-medium text-foreground">
					{task.creator.username}
				</div>
				<div className="truncate text-xs text-muted-foreground">
					{task.creator.email}
				</div>
			</div>
		);
	}
	if (task.creator_user_id != null) {
		return t("admin.tasks.source.userId", { id: task.creator_user_id });
	}
	return t("admin.tasks.source.system");
}

export default function AdminTasksPage() {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t("admin.tasks.title"));

	const { query, setQuery } = useManagedListQueryState<ManagedTaskQuery>({
		schema: {
			pageSize: managedPageSizeField(
				TASK_PAGE_SIZE_OPTIONS,
				DEFAULT_TASK_PAGE_SIZE,
			),
			status: managedEnumField("status", TASK_STATUS_FILTER_VALUES, "__all__"),
		},
		searchParams,
		setSearchParams,
	});
	const { pageSize, status: statusFilter } = query;
	const { pending: cleanupSubmitting, runWithPending } = usePendingAction();
	const { pendingId: retryingTaskId, runWithPending: runRetry } =
		usePendingId<number>();
	const [uiState, dispatchUi] = useReducer(
		adminTasksUiReducer,
		undefined,
		createInitialAdminTasksUiState,
	);
	const {
		cleanupDialogOpen,
		cleanupFinishedBefore,
		cleanupStatusFilter,
		detailDialogTaskId,
	} = uiState;

	const loadTaskPage = useCallback(
		async ({ cursor }: { cursor: TaskCursor | null }) => {
			const page = await adminTaskService.list({
				limit: pageSize,
				after_updated_at: cursor?.value,
				after_id: cursor?.id,
				...(statusFilter !== "__all__" ? { status: statusFilter } : {}),
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[pageSize, statusFilter],
	);
	const list = useCursorList<TaskInfo, TaskCursor>({
		deps: [pageSize, statusFilter],
		loadPage: loadTaskPage,
	});
	const { currentPage, error, items, loading, total, totalPages } = list;

	const activeFilterCount = statusFilter !== "__all__" ? 1 : 0;
	const hasServerFilters = activeFilterCount > 0;
	const visibleDetailTaskId =
		detailDialogTaskId != null &&
		items.some((task) => task.id === detailDialogTaskId)
			? detailDialogTaskId
			: null;
	const detailTask =
		items.find((task) => task.id === visibleDetailTaskId) ?? null;
	const pageSizeOptions = TASK_PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));
	const taskStatusFilterOptions = [
		{ label: t("admin.tasks.filters.allStatuses"), value: "__all__" },
		...TASK_STATUS_FILTER_VALUES.map((value) => ({
			label: formatTaskStatusLabel(t, value),
			value,
		})),
	] satisfies ReadonlyArray<{ label: string; value: string }>;
	const cleanupStatusFilterOptions = [
		{ label: t("admin.tasks.cleanup.allCompletedStatuses"), value: "__all__" },
		...TASK_TERMINAL_STATUS_FILTER_VALUES.map((value) => ({
			label: formatTaskStatusLabel(t, value),
			value,
		})),
	] satisfies ReadonlyArray<{ label: string; value: string }>;
	const cleanupRequest = buildCleanupRequest(
		cleanupFinishedBefore,
		cleanupStatusFilter,
	);

	const resetFilters = useCallback(() => {
		setQuery({ status: "__all__" });
	}, [setQuery]);
	const resetCleanupConditions = () => {
		dispatchUi({ type: "reset_cleanup_conditions" });
	};
	const handlePageSizeChange = useCallback(
		(value: string | null) => {
			const next = parsePageSizeOption(value, TASK_PAGE_SIZE_OPTIONS);
			if (next == null) {
				return;
			}
			// The cursor list resets to the first page as pageSize is a dep.
			setQuery({ pageSize: next });
		},
		[setQuery],
	);
	const handleStatusFilterChange = (value: string | null) => {
		// managedEnumField parses the raw param, rejecting values outside the
		// task status set.
		setQuery({ status: (value ?? "__all__") as TaskStatusFilter });
	};
	const handleCleanupSubmit = async (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (cleanupRequest == null) {
			return;
		}

		await runWithPending(async () => {
			try {
				const result = await adminTaskService.cleanup(cleanupRequest);
				toast.success(
					t("admin.tasks.cleanup.success", { count: result.removed }),
				);
				dispatchUi({ open: false, type: "set_cleanup_dialog_open" });
				await list.reloadFirstPage();
			} catch (error) {
				handleApiError(error);
			}
		});
	};
	const handleRetry = async (taskId: number) => {
		await runRetry(taskId, async () => {
			try {
				await adminTaskService.retry(taskId);
				toast.success(t("admin.tasks.retryQueued"));
				await list.reload();
			} catch (error) {
				handleApiError(error);
			}
		});
	};
	const cleanupDescription =
		cleanupRequest == null ? (
			t("admin.tasks.cleanup.invalidDescription")
		) : (
			<>
				{t("admin.tasks.cleanup.confirmDescriptionPrefix")}{" "}
				<DateTimeText value={cleanupRequest.finished_before} />{" "}
				{t("admin.tasks.cleanup.confirmDescriptionSuffix", {
					status:
						cleanupRequest.status != null
							? formatTaskStatusLabel(t, cleanupRequest.status)
							: t("admin.tasks.cleanup.allCompletedStatuses"),
				})}
			</>
		);
	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				currentPage={currentPage}
				nextDisabled={list.nextPageDisabled}
				onNext={list.goNext}
				onPageSizeChange={handlePageSizeChange}
				onPrevious={list.goPrevious}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={list.prevPageDisabled}
				total={total}
				totalPages={totalPages}
			/>
		),
		[
			currentPage,
			handlePageSizeChange,
			list.goNext,
			list.goPrevious,
			list.nextPageDisabled,
			list.prevPageDisabled,
			pageSize,
			pageSizeOptions,
			total,
			totalPages,
		],
	);
	const toolbar = (
		<AdminTaskFiltersToolbar
			activeFilterCount={activeFilterCount}
			onResetFilters={resetFilters}
			onStatusChange={handleStatusFilterChange}
			statusFilter={statusFilter}
			statusLabel={t("common.status")}
			statusOptions={taskStatusFilterOptions}
		/>
	);
	const headerRow = useMemo(() => <AdminTaskTableHeader />, []);
	const emptyIcon = useMemo(
		() => <Icon name="Queue" className="size-10" />,
		[],
	);
	const emptyAction = hasServerFilters ? (
		<Button type="button" variant="outline" onClick={resetFilters}>
			{t("admin.clearFilters")}
		</Button>
	) : null;

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.tasks.title")}
				description={t("admin.tasks.description")}
				actions={
					<>
						<Button
							type="button"
							variant="outline"
							size="sm"
							onClick={() =>
								dispatchUi({ open: true, type: "set_cleanup_dialog_open" })
							}
							disabled={cleanupSubmitting}
						>
							<Icon name="Trash" className="size-4" />
							{t("admin.tasks.cleanup.action")}
						</Button>
						<RefreshButton
							loading={loading}
							disabled={cleanupSubmitting}
							onClick={() => void list.reload()}
						/>
					</>
				}
				toolbar={toolbar}
			/>

			{error && items.length === 0 ? (
				<AdminSurface padded={false}>
					<EmptyState
						icon={<Icon name="CircleAlert" className="size-5" />}
						title={t("admin.tasks.loadErrorTitle")}
						description={error}
						action={
							<Button
								type="button"
								variant="outline"
								onClick={() => void list.reload()}
							>
								{t("common.refresh")}
							</Button>
						}
					/>
				</AdminSurface>
			) : (
				<AdminTableList
					columns={7}
					emptyDescription={t("admin.tasks.emptyDescription")}
					emptyIcon={emptyIcon}
					emptyTitle={t("admin.tasks.emptyTitle")}
					filtered={hasServerFilters}
					filteredEmptyAction={emptyAction}
					filteredEmptyDescription={t("admin.tasks.filteredEmptyDescription")}
					filteredEmptyTitle={t("admin.tasks.filteredEmptyTitle")}
					headerRow={headerRow}
					items={items}
					loading={loading}
					pagination={pagination}
					renderRow={(task) => (
						<AdminTaskTableRow
							key={task.id}
							formatTaskSource={(item) => taskSourceLabel(t, item)}
							onOpenDetail={(taskId) =>
								dispatchUi({
									taskId,
									type: "set_detail_dialog_task",
								})
							}
							onRetry={(taskId) => void handleRetry(taskId)}
							retryingTaskId={retryingTaskId}
							task={task}
						/>
					)}
					rows={6}
				/>
			)}

			<AdminTaskDetailDialog
				detailTask={detailTask}
				formatTaskSource={(task) => taskSourceLabel(t, task)}
				onOpenDetailChange={(open) => {
					if (!open) {
						dispatchUi({
							taskId: null,
							type: "set_detail_dialog_task",
						});
					}
				}}
				onRetry={(taskId) => void handleRetry(taskId)}
				retryingTaskId={retryingTaskId}
			/>

			<AdminTaskCleanupDialog
				description={cleanupDescription}
				finishedBefore={cleanupFinishedBefore}
				onFinishedBeforeChange={(value) =>
					dispatchUi({
						type: "set_cleanup_finished_before",
						value,
					})
				}
				onOpenChange={(open) =>
					dispatchUi({ open, type: "set_cleanup_dialog_open" })
				}
				onResetConditions={resetCleanupConditions}
				onStatusFilterChange={(value) =>
					dispatchUi({
						type: "set_cleanup_status_filter",
						value: parseTaskTerminalStatus(value),
					})
				}
				onSubmit={handleCleanupSubmit}
				open={cleanupDialogOpen}
				statusFilter={cleanupStatusFilter}
				statusOptions={cleanupStatusFilterOptions}
				submitDisabled={cleanupRequest == null || cleanupSubmitting}
				submitting={cleanupSubmitting}
			/>
		</AdminPageShell>
	);
}
