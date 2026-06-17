import type { FormEvent, ReactNode, SetStateAction } from "react";
import { useCallback, useMemo, useReducer, useState } from "react";
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
import { EmptyState } from "@/components/common/EmptyState";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { handleApiError } from "@/hooks/useApiError";
import { useApiList } from "@/hooks/useApiList";
import { usePageTitle } from "@/hooks/usePageTitle";
import { formatDateTime as formatSharedDateTime } from "@/lib/dateTime";
import { dateTimeLocalToIso } from "@/lib/form";
import {
	buildOffsetPaginationSearchParams,
	parseOffsetSearchParam,
	parsePageSizeOption,
	parsePageSizeSearchParam,
	parseSortOrderSearchParam,
	parseSortSearchParam,
	type SortOrder,
} from "@/lib/pagination";
import { formatTaskStatusLabel } from "@/lib/tasks";
import { adminTaskService } from "@/services/adminService";
import type {
	AdminTaskCleanupRequest,
	AdminTaskSortBy,
	BackgroundTaskStatus,
	TaskInfo,
} from "@/types/api";

const TASK_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
const DEFAULT_TASK_PAGE_SIZE = 20 as const;
const TASK_MANAGED_QUERY_KEYS = [
	"offset",
	"pageSize",
	"sortBy",
	"sortOrder",
	"status",
] as const;
const TASK_SORT_BY_OPTIONS = [
	"display_name",
	"status",
	"progress",
	"created_at",
	"updated_at",
	"started_at",
	"finished_at",
] as const satisfies readonly AdminTaskSortBy[];
const DEFAULT_TASK_SORT_BY = "updated_at" as const satisfies AdminTaskSortBy;
const DEFAULT_TASK_SORT_ORDER = "desc" as const satisfies SortOrder;
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
	offset: number;
	pageSize: (typeof TASK_PAGE_SIZE_OPTIONS)[number];
	sortBy: AdminTaskSortBy;
	sortOrder: SortOrder;
	status: TaskStatusFilter;
};
type AdminTasksUiState = {
	cleanupDialogOpen: boolean;
	cleanupFinishedBefore: string;
	cleanupStatusFilter: TaskTerminalStatusFilter;
	cleanupSubmitting: boolean;
	detailDialogTaskId: number | null;
};
type AdminTasksUiAction =
	| { open: boolean; type: "set_cleanup_dialog_open" }
	| { taskId: number | null; type: "set_detail_dialog_task" }
	| { value: string; type: "set_cleanup_finished_before" }
	| { value: TaskTerminalStatusFilter; type: "set_cleanup_status_filter" }
	| { submitting: boolean; type: "set_cleanup_submitting" }
	| { type: "reset_cleanup_conditions" };

function normalizeOffset(offset: number) {
	return Math.max(0, Math.floor(offset));
}

function parseTaskStatusSearchParam(value: string | null): TaskStatusFilter {
	return TASK_STATUS_FILTER_VALUES.includes(
		value as (typeof TASK_STATUS_FILTER_VALUES)[number],
	)
		? (value as BackgroundTaskStatus)
		: "__all__";
}

function parseTaskTerminalStatus(
	value: string | null,
): TaskTerminalStatusFilter {
	return TASK_TERMINAL_STATUS_FILTER_VALUES.includes(
		value as (typeof TASK_TERMINAL_STATUS_FILTER_VALUES)[number],
	)
		? (value as TaskTerminalStatusFilter)
		: "__all__";
}

function buildManagedTaskSearchParams({
	offset,
	pageSize,
	sortBy,
	sortOrder,
	status,
}: ManagedTaskQuery) {
	return buildOffsetPaginationSearchParams({
		offset,
		pageSize,
		defaultPageSize: DEFAULT_TASK_PAGE_SIZE,
		extraParams: {
			sortBy: sortBy !== DEFAULT_TASK_SORT_BY ? sortBy : undefined,
			sortOrder: sortOrder !== DEFAULT_TASK_SORT_ORDER ? sortOrder : undefined,
			status: status !== "__all__" ? status : undefined,
		},
	});
}

function readManagedTaskQuery(searchParams: URLSearchParams): ManagedTaskQuery {
	return {
		offset: normalizeOffset(parseOffsetSearchParam(searchParams.get("offset"))),
		pageSize: parsePageSizeSearchParam(
			searchParams.get("pageSize"),
			TASK_PAGE_SIZE_OPTIONS,
			DEFAULT_TASK_PAGE_SIZE,
		),
		sortBy: parseSortSearchParam(
			searchParams.get("sortBy"),
			TASK_SORT_BY_OPTIONS,
			DEFAULT_TASK_SORT_BY,
		),
		sortOrder: parseSortOrderSearchParam(
			searchParams.get("sortOrder"),
			DEFAULT_TASK_SORT_ORDER,
		),
		status: parseTaskStatusSearchParam(searchParams.get("status")),
	};
}

function mergeManagedTaskSearchParams(
	searchParams: URLSearchParams,
	managedSearchParams: URLSearchParams,
) {
	const merged = new URLSearchParams(searchParams);
	for (const key of TASK_MANAGED_QUERY_KEYS) {
		merged.delete(key);
	}
	for (const [key, value] of managedSearchParams.entries()) {
		merged.set(key, value);
	}
	return merged;
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
		cleanupSubmitting: false,
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
		case "set_cleanup_submitting":
			return { ...state, cleanupSubmitting: action.submitting };
		case "reset_cleanup_conditions":
			return {
				...state,
				cleanupFinishedBefore: defaultCleanupFinishedBeforeValue(),
				cleanupStatusFilter: "__all__",
			};
	}
}

function formatDateTime(value: string, locale: string) {
	return formatSharedDateTime(value, locale);
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
	const { t, i18n } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t("admin.tasks.title"));

	const taskQuery = readManagedTaskQuery(searchParams);
	const {
		offset,
		pageSize,
		sortBy,
		sortOrder,
		status: statusFilter,
	} = taskQuery;
	const [retryingTaskId, setRetryingTaskId] = useState<number | null>(null);
	const [uiState, dispatchUi] = useReducer(
		adminTasksUiReducer,
		undefined,
		createInitialAdminTasksUiState,
	);
	const {
		cleanupDialogOpen,
		cleanupFinishedBefore,
		cleanupStatusFilter,
		cleanupSubmitting,
		detailDialogTaskId,
	} = uiState;
	const locale = i18n.language?.startsWith("zh") ? "zh-CN" : "en-US";
	const formatTaskDateTime = (value: string) => formatDateTime(value, locale);

	const setTaskQuery = useCallback(
		(updates: Partial<ManagedTaskQuery>) => {
			const nextManagedSearchParams = buildManagedTaskSearchParams({
				...taskQuery,
				...updates,
			});
			setSearchParams(
				mergeManagedTaskSearchParams(searchParams, nextManagedSearchParams),
				{ replace: true },
			);
		},
		[searchParams, setSearchParams, taskQuery],
	);
	const setOffset = useCallback(
		(value: SetStateAction<number>) => {
			setTaskQuery({
				offset: normalizeOffset(
					typeof value === "function" ? value(offset) : value,
				),
			});
		},
		[offset, setTaskQuery],
	);

	const { error, items, loading, reload, total } = useApiList(
		() =>
			adminTaskService.list({
				limit: pageSize,
				offset,
				...(statusFilter !== "__all__" ? { status: statusFilter } : {}),
				sort_by: sortBy,
				sort_order: sortOrder,
			}),
		[offset, pageSize, sortBy, sortOrder, statusFilter],
	);

	const activeFilterCount = statusFilter !== "__all__" ? 1 : 0;
	const hasServerFilters = activeFilterCount > 0;
	const totalPages = Math.max(1, Math.ceil(total / pageSize));
	const currentPage = Math.floor(offset / pageSize) + 1;
	const prevPageDisabled = offset === 0;
	const nextPageDisabled = offset + pageSize >= total;
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
		setTaskQuery({ offset: 0, status: "__all__" });
	}, [setTaskQuery]);
	const resetCleanupConditions = () => {
		dispatchUi({ type: "reset_cleanup_conditions" });
	};
	const handlePageSizeChange = useCallback(
		(value: string | null) => {
			const next = parsePageSizeOption(value, TASK_PAGE_SIZE_OPTIONS);
			if (next == null) {
				return;
			}
			setTaskQuery({ offset: 0, pageSize: next });
		},
		[setTaskQuery],
	);
	const handleStatusFilterChange = (value: string | null) => {
		setTaskQuery({
			offset: 0,
			status:
				value === "__all__" ? "__all__" : parseTaskStatusSearchParam(value),
		});
	};
	const handleSortChange = useCallback(
		(nextSortBy: AdminTaskSortBy, nextOrder: SortOrder) => {
			setTaskQuery({ offset: 0, sortBy: nextSortBy, sortOrder: nextOrder });
		},
		[setTaskQuery],
	);
	const handleCleanupSubmit = async (event: FormEvent<HTMLFormElement>) => {
		event.preventDefault();
		if (cleanupRequest == null) {
			return;
		}

		dispatchUi({ submitting: true, type: "set_cleanup_submitting" });
		try {
			const result = await adminTaskService.cleanup(cleanupRequest);
			toast.success(
				t("admin.tasks.cleanup.success", { count: result.removed }),
			);
			dispatchUi({ open: false, type: "set_cleanup_dialog_open" });
			if (offset !== 0) {
				setOffset(0);
			} else {
				await reload();
			}
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatchUi({ submitting: false, type: "set_cleanup_submitting" });
		}
	};
	const handleRetry = async (taskId: number) => {
		if (retryingTaskId !== null) {
			return;
		}

		setRetryingTaskId(taskId);
		try {
			await adminTaskService.retry(taskId);
			toast.success(t("admin.tasks.retryQueued"));
			await reload();
		} catch (error) {
			handleApiError(error);
		} finally {
			setRetryingTaskId(null);
		}
	};
	const cleanupDescription =
		cleanupRequest == null
			? t("admin.tasks.cleanup.invalidDescription")
			: t("admin.tasks.cleanup.confirmDescription", {
					finishedBefore: formatTaskDateTime(cleanupRequest.finished_before),
					status:
						cleanupRequest.status != null
							? formatTaskStatusLabel(t, cleanupRequest.status)
							: t("admin.tasks.cleanup.allCompletedStatuses"),
				});
	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				currentPage={currentPage}
				nextDisabled={nextPageDisabled}
				onNext={() => setOffset((current) => current + pageSize)}
				onPageSizeChange={handlePageSizeChange}
				onPrevious={() =>
					setOffset((current) => Math.max(0, current - pageSize))
				}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={prevPageDisabled}
				total={total}
				totalPages={totalPages}
			/>
		),
		[
			currentPage,
			handlePageSizeChange,
			nextPageDisabled,
			pageSize,
			pageSizeOptions,
			prevPageDisabled,
			total,
			totalPages,
			setOffset,
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
	const headerRow = useMemo(
		() => (
			<AdminTaskTableHeader
				sortBy={sortBy}
				sortOrder={sortOrder}
				onSortChange={handleSortChange}
			/>
		),
		[handleSortChange, sortBy, sortOrder],
	);
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
						<Button
							type="button"
							variant="outline"
							size="sm"
							onClick={() => void reload()}
							disabled={loading || cleanupSubmitting}
						>
							<Icon
								name={loading ? "Spinner" : "ArrowsClockwise"}
								className={loading ? "size-4 animate-spin" : "size-4"}
							/>
							{t("common.refresh")}
						</Button>
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
								onClick={() => void reload()}
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
							formatDateTime={formatTaskDateTime}
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
				formatDateTime={formatTaskDateTime}
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
