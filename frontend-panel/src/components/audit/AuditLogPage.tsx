import type { TFunction } from "i18next";
import type { ReactNode, SetStateAction } from "react";
import { useCallback, useEffect, useMemo, useReducer, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { AdminFilterToolbar } from "@/components/common/AdminFilterToolbar";
import {
	ADMIN_TABLE_MONO_TEXT_CLASS,
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminSortableTableHead,
	AdminTableCell,
	AdminTableHeader,
	AdminTableRow,
} from "@/components/common/AdminTable";
import { AdminTableList } from "@/components/common/AdminTableList";
import { EmptyState } from "@/components/common/EmptyState";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { useApiList } from "@/hooks/useApiList";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	AUDIT_ENTITY_TYPE_FILTER_VALUES,
	formatAuditDetail,
	formatAuditEntityType,
	formatAuditSummary,
	formatAuditTarget,
	formatAuditTargetType,
	getAuditActionBadgeClass,
	isAuditEntityType,
} from "@/lib/audit";
import {
	buildOffsetPaginationSearchParams,
	parseOffsetSearchParam,
	parsePageSizeOption,
	parsePageSizeSearchParam,
	parseSortOrderSearchParam,
	parseSortSearchParam,
	type SortOrder,
} from "@/lib/pagination";
import type {
	AuditEntityType,
	AuditLogEntry,
	AuditLogPage as AuditLogPageData,
	AuditLogSortBy,
} from "@/types/api";

const AUDIT_PAGE_SIZE_OPTIONS = [10, 20, 50] as const;
const DEFAULT_AUDIT_PAGE_SIZE = 20 as const;
const DEFAULT_AUDIT_SORT_BY = "created_at" as const satisfies AuditLogSortBy;
const DEFAULT_AUDIT_SORT_ORDER = "desc" as const satisfies SortOrder;
const AUDIT_MANAGED_QUERY_KEYS = [
	"action",
	"entityType",
	"offset",
	"pageSize",
	"sortBy",
	"sortOrder",
] as const;

type AuditEntityTypeFilter = "__all__" | AuditEntityType;
type AuditQueryState = {
	actionFilter: string;
	entityTypeFilter: AuditEntityTypeFilter;
	offset: number;
	pageSize: (typeof AUDIT_PAGE_SIZE_OPTIONS)[number];
	sortBy: AuditLogSortBy;
	sortOrder: SortOrder;
};
type AuditQueryAction =
	| { type: "replace"; value: AuditQueryState }
	| { type: "reset_filters" }
	| { type: "set_action_filter"; value: string }
	| { type: "set_entity_type_filter"; value: AuditEntityTypeFilter }
	| { type: "set_offset"; value: SetStateAction<number> }
	| { type: "set_page_size"; value: (typeof AUDIT_PAGE_SIZE_OPTIONS)[number] }
	| { type: "set_sort"; sortBy: AuditLogSortBy; sortOrder: SortOrder };

type AuditLogPageProps = {
	list: (query: {
		action?: string;
		entity_type?: AuditEntityType;
		limit: number;
		offset: number;
		sort_by: AuditLogSortBy;
		sort_order: SortOrder;
	}) => Promise<AuditLogPageData>;
	showActor: boolean;
	sortOptions: readonly AuditLogSortBy[];
	translationPrefix: "account.auditPage" | "admin.auditPage";
};

function normalizeOffset(offset: number) {
	return Math.max(0, Math.floor(offset));
}

function parseEntityTypeSearchParam(
	value: string | null,
): AuditEntityTypeFilter {
	const normalized = value?.trim();
	return normalized && isAuditEntityType(normalized) ? normalized : "__all__";
}

function buildManagedAuditSearchParams({
	action,
	entityType,
	offset,
	pageSize,
	sortBy,
	sortOrder,
}: {
	action: string;
	entityType: AuditEntityTypeFilter;
	offset: number;
	pageSize: (typeof AUDIT_PAGE_SIZE_OPTIONS)[number];
	sortBy: AuditLogSortBy;
	sortOrder: SortOrder;
}) {
	return buildOffsetPaginationSearchParams({
		defaultPageSize: DEFAULT_AUDIT_PAGE_SIZE,
		extraParams: {
			action: action.trim() || undefined,
			entityType: entityType !== "__all__" ? entityType : undefined,
			sortBy: sortBy !== DEFAULT_AUDIT_SORT_BY ? sortBy : undefined,
			sortOrder: sortOrder !== DEFAULT_AUDIT_SORT_ORDER ? sortOrder : undefined,
		},
		offset,
		pageSize,
	});
}

function getManagedAuditSearchString(
	searchParams: URLSearchParams,
	sortOptions: readonly AuditLogSortBy[],
) {
	return buildManagedAuditSearchParams({
		action: searchParams.get("action") ?? "",
		entityType: parseEntityTypeSearchParam(searchParams.get("entityType")),
		offset: normalizeOffset(parseOffsetSearchParam(searchParams.get("offset"))),
		pageSize: parsePageSizeSearchParam(
			searchParams.get("pageSize"),
			AUDIT_PAGE_SIZE_OPTIONS,
			DEFAULT_AUDIT_PAGE_SIZE,
		),
		sortBy: parseSortSearchParam(
			searchParams.get("sortBy"),
			sortOptions,
			DEFAULT_AUDIT_SORT_BY,
		),
		sortOrder: parseSortOrderSearchParam(
			searchParams.get("sortOrder"),
			DEFAULT_AUDIT_SORT_ORDER,
		),
	}).toString();
}

function parseAuditQueryState(
	searchParams: URLSearchParams,
	sortOptions: readonly AuditLogSortBy[],
): AuditQueryState {
	return {
		actionFilter: searchParams.get("action") ?? "",
		entityTypeFilter: parseEntityTypeSearchParam(
			searchParams.get("entityType"),
		),
		offset: normalizeOffset(parseOffsetSearchParam(searchParams.get("offset"))),
		pageSize: parsePageSizeSearchParam(
			searchParams.get("pageSize"),
			AUDIT_PAGE_SIZE_OPTIONS,
			DEFAULT_AUDIT_PAGE_SIZE,
		),
		sortBy: parseSortSearchParam(
			searchParams.get("sortBy"),
			sortOptions,
			DEFAULT_AUDIT_SORT_BY,
		),
		sortOrder: parseSortOrderSearchParam(
			searchParams.get("sortOrder"),
			DEFAULT_AUDIT_SORT_ORDER,
		),
	};
}

function auditQueryStatesEqual(left: AuditQueryState, right: AuditQueryState) {
	return (
		left.actionFilter === right.actionFilter &&
		left.entityTypeFilter === right.entityTypeFilter &&
		left.offset === right.offset &&
		left.pageSize === right.pageSize &&
		left.sortBy === right.sortBy &&
		left.sortOrder === right.sortOrder
	);
}

function auditQueryReducer(
	state: AuditQueryState,
	action: AuditQueryAction,
): AuditQueryState {
	switch (action.type) {
		case "replace":
			return auditQueryStatesEqual(state, action.value) ? state : action.value;
		case "reset_filters":
			return {
				...state,
				actionFilter: "",
				entityTypeFilter: "__all__",
				offset: 0,
			};
		case "set_action_filter":
			return { ...state, actionFilter: action.value, offset: 0 };
		case "set_entity_type_filter":
			return { ...state, entityTypeFilter: action.value, offset: 0 };
		case "set_offset": {
			const nextOffset = normalizeOffset(
				typeof action.value === "function"
					? action.value(state.offset)
					: action.value,
			);
			return nextOffset === state.offset
				? state
				: { ...state, offset: nextOffset };
		}
		case "set_page_size":
			return { ...state, pageSize: action.value, offset: 0 };
		case "set_sort":
			return {
				...state,
				offset: 0,
				sortBy: action.sortBy,
				sortOrder: action.sortOrder,
			};
	}
}

function mergeManagedAuditSearchParams(
	searchParams: URLSearchParams,
	managedSearchParams: URLSearchParams,
) {
	const merged = new URLSearchParams(searchParams);
	for (const key of AUDIT_MANAGED_QUERY_KEYS) {
		merged.delete(key);
	}
	for (const [key, value] of managedSearchParams.entries()) {
		merged.set(key, value);
	}
	return merged;
}

function formatTimestamp(value: string, formatter: Intl.DateTimeFormat) {
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) {
		return value;
	}
	return formatter.format(date);
}

export function AuditLogPage({
	list,
	showActor,
	sortOptions,
	translationPrefix,
}: AuditLogPageProps) {
	const { t, i18n } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t(`${translationPrefix}.title`));

	const [queryState, dispatchQuery] = useReducer(
		auditQueryReducer,
		searchParams,
		(params) => parseAuditQueryState(params, sortOptions),
	);
	const {
		actionFilter,
		entityTypeFilter,
		offset,
		pageSize,
		sortBy,
		sortOrder,
	} = queryState;
	const timestampLocale = i18n.language?.startsWith("zh") ? "zh-CN" : "en-US";
	const timestampFormatter = useMemo(
		() =>
			new Intl.DateTimeFormat(timestampLocale, {
				dateStyle: "medium",
				timeStyle: "short",
			}),
		[timestampLocale],
	);
	const lastWrittenSearchRef = useRef<string | null>(null);

	const setOffset = useCallback((value: SetStateAction<number>) => {
		dispatchQuery({ type: "set_offset", value });
	}, []);

	useEffect(() => {
		const managedSearch = getManagedAuditSearchString(
			searchParams,
			sortOptions,
		);
		if (managedSearch === lastWrittenSearchRef.current) {
			return;
		}

		dispatchQuery({
			type: "replace",
			value: parseAuditQueryState(searchParams, sortOptions),
		});
	}, [searchParams, sortOptions]);

	useEffect(() => {
		const nextManagedSearchParams = buildManagedAuditSearchParams({
			action: actionFilter,
			entityType: entityTypeFilter,
			offset,
			pageSize,
			sortBy,
			sortOrder,
		});
		const nextSearch = nextManagedSearchParams.toString();
		const currentSearch = getManagedAuditSearchString(
			searchParams,
			sortOptions,
		);

		if (
			currentSearch !== lastWrittenSearchRef.current &&
			currentSearch !== nextSearch
		) {
			return;
		}

		lastWrittenSearchRef.current = nextSearch;
		if (nextSearch === currentSearch) {
			return;
		}

		setSearchParams(
			mergeManagedAuditSearchParams(searchParams, nextManagedSearchParams),
			{ replace: true },
		);
	}, [
		actionFilter,
		entityTypeFilter,
		offset,
		pageSize,
		searchParams,
		setSearchParams,
		sortBy,
		sortOptions,
		sortOrder,
	]);

	const { error, items, loading, reload, total } = useApiList(
		() =>
			list({
				action: actionFilter.trim() || undefined,
				entity_type:
					entityTypeFilter === "__all__" ? undefined : entityTypeFilter,
				limit: pageSize,
				offset,
				sort_by: sortBy,
				sort_order: sortOrder,
			}),
		[actionFilter, entityTypeFilter, list, offset, pageSize, sortBy, sortOrder],
	);

	const activeFilterCount =
		(actionFilter.trim().length > 0 ? 1 : 0) +
		(entityTypeFilter !== "__all__" ? 1 : 0);
	const hasServerFilters = activeFilterCount > 0;
	const totalPages = Math.max(1, Math.ceil(total / pageSize));
	const currentPage = Math.floor(offset / pageSize) + 1;
	const prevPageDisabled = offset === 0;
	const nextPageDisabled = offset + pageSize >= total;
	const entityTypeOptions = [
		{
			label: t(`${translationPrefix}.filters.allTypes`),
			value: "__all__",
		},
		...AUDIT_ENTITY_TYPE_FILTER_VALUES.map((value) => ({
			label: formatAuditEntityType(t, value),
			value,
		})),
	] satisfies ReadonlyArray<{ label: string; value: AuditEntityTypeFilter }>;
	const pageSizeOptions = AUDIT_PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));

	const resetFilters = useCallback(() => {
		dispatchQuery({ type: "reset_filters" });
	}, []);

	const handlePageSizeChange = useCallback((value: string | null) => {
		const next = parsePageSizeOption(value, AUDIT_PAGE_SIZE_OPTIONS);
		if (next == null) {
			return;
		}
		dispatchQuery({ type: "set_page_size", value: next });
	}, []);

	const handleSortChange = useCallback(
		(nextSortBy: AuditLogSortBy, nextOrder: SortOrder) => {
			dispatchQuery({
				type: "set_sort",
				sortBy: nextSortBy,
				sortOrder: nextOrder,
			});
		},
		[],
	);

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
		<AuditFilterToolbar
			actionFilter={actionFilter}
			activeFilterCount={activeFilterCount}
			entityTypeFilter={entityTypeFilter}
			entityTypeOptions={entityTypeOptions}
			translationPrefix={translationPrefix}
			onActionFilterChange={(value) =>
				dispatchQuery({ type: "set_action_filter", value })
			}
			onEntityTypeFilterChange={(value) => {
				if (!value) return;
				dispatchQuery({
					type: "set_entity_type_filter",
					value: isAuditEntityType(value) ? value : "__all__",
				});
			}}
			onResetFilters={resetFilters}
		/>
	);

	const headerRow = useMemo(
		() => (
			<AuditTableHeader
				showActor={showActor}
				sortBy={sortBy}
				sortOrder={sortOrder}
				translationPrefix={translationPrefix}
				onSortChange={handleSortChange}
			/>
		),
		[handleSortChange, showActor, sortBy, sortOrder, translationPrefix],
	);
	const emptyIcon = useMemo(
		() => <Icon name="Scroll" className="size-10" />,
		[],
	);
	const filteredEmptyAction = useMemo(
		() => (
			<Button type="button" variant="outline" onClick={resetFilters}>
				{t("admin.clearFilters")}
			</Button>
		),
		[resetFilters, t],
	);

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t(`${translationPrefix}.title`)}
				description={t(`${translationPrefix}.description`)}
				actions={
					<Button
						type="button"
						variant="outline"
						size="sm"
						onClick={() => void reload()}
						disabled={loading}
					>
						<Icon
							name={loading ? "Spinner" : "ArrowsClockwise"}
							className={loading ? "size-4 animate-spin" : "size-4"}
						/>
						{t("common.refresh")}
					</Button>
				}
				toolbar={toolbar}
			/>

			<AuditLogTable
				emptyIcon={emptyIcon}
				error={error}
				filteredEmptyAction={filteredEmptyAction}
				hasServerFilters={hasServerFilters}
				headerRow={headerRow}
				items={items}
				loading={loading}
				pagination={pagination}
				reload={reload}
				showActor={showActor}
				t={t}
				timestampFormatter={timestampFormatter}
				translationPrefix={translationPrefix}
			/>
		</AdminPageShell>
	);
}

function AuditFilterToolbar({
	actionFilter,
	activeFilterCount,
	entityTypeFilter,
	entityTypeOptions,
	onActionFilterChange,
	onEntityTypeFilterChange,
	onResetFilters,
	translationPrefix,
}: {
	actionFilter: string;
	activeFilterCount: number;
	entityTypeFilter: AuditEntityTypeFilter;
	entityTypeOptions: ReadonlyArray<{
		label: string;
		value: AuditEntityTypeFilter;
	}>;
	onActionFilterChange: (value: string) => void;
	onEntityTypeFilterChange: (value: string | null) => void;
	onResetFilters: () => void;
	translationPrefix: AuditLogPageProps["translationPrefix"];
}) {
	const { t } = useTranslation();

	return (
		<AdminFilterToolbar
			activeFilterCount={activeFilterCount}
			inline
			onResetFilters={onResetFilters}
		>
			<div className="relative min-w-[14rem] flex-1 md:max-w-sm">
				<Icon
					name="MagnifyingGlass"
					className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
				/>
				<Input
					value={actionFilter}
					onChange={(event) => onActionFilterChange(event.target.value)}
					placeholder={t(`${translationPrefix}.filters.actionPlaceholder`)}
					className="pl-9"
				/>
			</div>
			<Select
				items={entityTypeOptions}
				value={entityTypeFilter}
				onValueChange={onEntityTypeFilterChange}
			>
				<SelectTrigger
					width="compact"
					aria-label={t(`${translationPrefix}.entity`)}
				>
					<SelectValue />
				</SelectTrigger>
				<SelectContent align="start">
					{entityTypeOptions.map((option) => (
						<SelectItem key={option.value} value={option.value}>
							{option.label}
						</SelectItem>
					))}
				</SelectContent>
			</Select>
		</AdminFilterToolbar>
	);
}

function AuditTableHeader({
	onSortChange,
	showActor,
	sortBy,
	sortOrder,
	translationPrefix,
}: {
	onSortChange: (sortBy: AuditLogSortBy, sortOrder: SortOrder) => void;
	showActor: boolean;
	sortBy: AuditLogSortBy;
	sortOrder: SortOrder;
	translationPrefix: AuditLogPageProps["translationPrefix"];
}) {
	const { t } = useTranslation();

	return (
		<AdminTableHeader>
			<AdminTableRow>
				<AdminSortableTableHead
					className="w-[12rem]"
					onSortChange={onSortChange}
					sortBy={sortBy}
					sortKey="created_at"
					sortOrder={sortOrder}
				>
					{t(`${translationPrefix}.created`)}
				</AdminSortableTableHead>
				{showActor ? (
					<AdminSortableTableHead
						className="w-[11rem]"
						onSortChange={onSortChange}
						sortBy={sortBy}
						sortKey="user_id"
						sortOrder={sortOrder}
					>
						{t("admin.auditPage.user")}
					</AdminSortableTableHead>
				) : null}
				<AdminSortableTableHead
					className="w-[15rem]"
					onSortChange={onSortChange}
					sortBy={sortBy}
					sortKey="action"
					sortOrder={sortOrder}
				>
					{t(`${translationPrefix}.action`)}
				</AdminSortableTableHead>
				<AdminSortableTableHead
					className="w-[10rem]"
					onSortChange={onSortChange}
					sortBy={sortBy}
					sortKey="entity_type"
					sortOrder={sortOrder}
				>
					{t(`${translationPrefix}.entity`)}
				</AdminSortableTableHead>
				<AdminSortableTableHead
					onSortChange={onSortChange}
					sortBy={sortBy}
					sortKey="entity_name"
					sortOrder={sortOrder}
				>
					{t("common.name")}
				</AdminSortableTableHead>
				<AdminSortableTableHead
					className="w-[10rem]"
					onSortChange={onSortChange}
					sortBy={sortBy}
					sortKey="ip_address"
					sortOrder={sortOrder}
				>
					IP
				</AdminSortableTableHead>
			</AdminTableRow>
		</AdminTableHeader>
	);
}

function AuditLogTable({
	emptyIcon,
	error,
	filteredEmptyAction,
	hasServerFilters,
	headerRow,
	items,
	loading,
	pagination,
	reload,
	showActor,
	t,
	timestampFormatter,
	translationPrefix,
}: {
	emptyIcon: ReactNode;
	error: string | null;
	filteredEmptyAction: ReactNode;
	hasServerFilters: boolean;
	headerRow: ReactNode;
	items: AuditLogEntry[];
	loading: boolean;
	pagination: ReactNode;
	reload: () => void;
	showActor: boolean;
	t: TFunction;
	timestampFormatter: Intl.DateTimeFormat;
	translationPrefix: AuditLogPageProps["translationPrefix"];
}) {
	if (error && items.length === 0) {
		return (
			<AdminSurface padded={false}>
				<EmptyState
					icon={<Icon name="CircleAlert" className="size-5" />}
					title={t(`${translationPrefix}.loadErrorTitle`)}
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
		);
	}

	return (
		<AdminTableList
			columns={showActor ? 6 : 5}
			emptyDescription={t(`${translationPrefix}.emptyDescription`)}
			emptyIcon={emptyIcon}
			emptyTitle={t(`${translationPrefix}.emptyTitle`)}
			filtered={hasServerFilters}
			filteredEmptyAction={filteredEmptyAction}
			filteredEmptyDescription={t(
				`${translationPrefix}.filteredEmptyDescription`,
			)}
			filteredEmptyTitle={t(`${translationPrefix}.filteredEmptyTitle`)}
			headerRow={headerRow}
			items={items}
			loading={loading}
			pagination={pagination}
			renderRow={(item) => (
				<AuditLogTableRow
					key={item.id}
					item={item}
					showActor={showActor}
					t={t}
					timestampFormatter={timestampFormatter}
					translationPrefix={translationPrefix}
				/>
			)}
			rows={6}
		/>
	);
}

function AuditLogTableRow({
	item,
	showActor,
	t,
	timestampFormatter,
	translationPrefix,
}: {
	item: AuditLogEntry;
	showActor: boolean;
	t: TFunction;
	timestampFormatter: Intl.DateTimeFormat;
	translationPrefix: AuditLogPageProps["translationPrefix"];
}) {
	const detail = formatAuditDetail(t, item);

	return (
		<AdminTableRow>
			<AdminTableCell>
				<div className="grid gap-1">
					<span className="text-sm text-foreground" title={item.created_at}>
						{formatTimestamp(item.created_at, timestampFormatter)}
					</span>
					<span className={ADMIN_TABLE_MONO_TEXT_CLASS}>#{item.id}</span>
				</div>
			</AdminTableCell>
			{showActor ? (
				<AdminTableCell>
					<div className="grid gap-1">
						<span className="text-sm text-foreground">
							{item.user?.username || t("admin.auditPage.systemActor")}
						</span>
						<span className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
							{t("admin.auditPage.userId", { id: item.user_id })}
						</span>
					</div>
				</AdminTableCell>
			) : null}
			<AdminTableCell>
				<div className="grid gap-1">
					<Badge
						variant="outline"
						className={getAuditActionBadgeClass(item.action)}
					>
						{formatAuditSummary(t, item)}
					</Badge>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="grid gap-1">
					<span className="text-sm text-foreground">
						{formatAuditTargetType(t, item)}
					</span>
					{item.entity_id != null ? (
						<span className={ADMIN_TABLE_MONO_TEXT_CLASS}>
							#{item.entity_id}
						</span>
					) : null}
				</div>
			</AdminTableCell>
			<AdminTableCell className="max-w-0">
				<div className="grid min-w-0 gap-1">
					<span
						className="truncate text-sm text-foreground"
						title={formatAuditTarget(t, item)}
					>
						{formatAuditTarget(t, item)}
					</span>
					{detail ? (
						<span className="truncate text-xs text-muted-foreground">
							{detail}
						</span>
					) : (
						<span className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
							{t(`${translationPrefix}.noDetail`)}
						</span>
					)}
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<span className={ADMIN_TABLE_MONO_TEXT_CLASS}>
					{item.ip_address ?? "---"}
				</span>
			</AdminTableCell>
		</AdminTableRow>
	);
}
