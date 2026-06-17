import type { SetStateAction } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import { adminAuditService } from "@/services/adminService";
import type { AuditEntityType, AuditLogSortBy } from "@/types/api";

const AUDIT_PAGE_SIZE_OPTIONS = [10, 20, 50] as const;
const DEFAULT_AUDIT_PAGE_SIZE = 20 as const;
const AUDIT_SORT_BY_OPTIONS = [
	"id",
	"created_at",
	"user_id",
	"action",
	"entity_type",
	"entity_name",
	"ip_address",
] as const satisfies readonly AuditLogSortBy[];
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

function getManagedAuditSearchString(searchParams: URLSearchParams) {
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
			AUDIT_SORT_BY_OPTIONS,
			DEFAULT_AUDIT_SORT_BY,
		),
		sortOrder: parseSortOrderSearchParam(
			searchParams.get("sortOrder"),
			DEFAULT_AUDIT_SORT_ORDER,
		),
	}).toString();
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

function formatTimestamp(value: string, locale: string) {
	const date = new Date(value);
	if (Number.isNaN(date.getTime())) {
		return value;
	}
	return new Intl.DateTimeFormat(locale, {
		dateStyle: "medium",
		timeStyle: "short",
	}).format(date);
}

export default function AdminAuditPage() {
	const { t, i18n } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t("admin.auditPage.title"));

	const [offset, setOffsetState] = useState(
		normalizeOffset(parseOffsetSearchParam(searchParams.get("offset"))),
	);
	const [pageSize, setPageSize] = useState<
		(typeof AUDIT_PAGE_SIZE_OPTIONS)[number]
	>(
		parsePageSizeSearchParam(
			searchParams.get("pageSize"),
			AUDIT_PAGE_SIZE_OPTIONS,
			DEFAULT_AUDIT_PAGE_SIZE,
		),
	);
	const [actionFilter, setActionFilter] = useState(
		searchParams.get("action") ?? "",
	);
	const [entityTypeFilter, setEntityTypeFilter] =
		useState<AuditEntityTypeFilter>(
			parseEntityTypeSearchParam(searchParams.get("entityType")),
		);
	const [sortBy, setSortBy] = useState<AuditLogSortBy>(
		parseSortSearchParam(
			searchParams.get("sortBy"),
			AUDIT_SORT_BY_OPTIONS,
			DEFAULT_AUDIT_SORT_BY,
		),
	);
	const [sortOrder, setSortOrder] = useState<SortOrder>(
		parseSortOrderSearchParam(
			searchParams.get("sortOrder"),
			DEFAULT_AUDIT_SORT_ORDER,
		),
	);
	const lastWrittenSearchRef = useRef<string | null>(null);

	const setOffset = useCallback((value: SetStateAction<number>) => {
		setOffsetState((current) =>
			normalizeOffset(typeof value === "function" ? value(current) : value),
		);
	}, []);

	useEffect(() => {
		const managedSearch = getManagedAuditSearchString(searchParams);
		if (managedSearch === lastWrittenSearchRef.current) {
			return;
		}

		const nextOffset = normalizeOffset(
			parseOffsetSearchParam(searchParams.get("offset")),
		);
		const nextPageSize = parsePageSizeSearchParam(
			searchParams.get("pageSize"),
			AUDIT_PAGE_SIZE_OPTIONS,
			DEFAULT_AUDIT_PAGE_SIZE,
		);
		const nextAction = searchParams.get("action") ?? "";
		const nextEntityType = parseEntityTypeSearchParam(
			searchParams.get("entityType"),
		);
		const nextSortBy = parseSortSearchParam(
			searchParams.get("sortBy"),
			AUDIT_SORT_BY_OPTIONS,
			DEFAULT_AUDIT_SORT_BY,
		);
		const nextSortOrder = parseSortOrderSearchParam(
			searchParams.get("sortOrder"),
			DEFAULT_AUDIT_SORT_ORDER,
		);

		setOffsetState((prev) => (prev === nextOffset ? prev : nextOffset));
		setPageSize((prev) => (prev === nextPageSize ? prev : nextPageSize));
		setActionFilter((prev) => (prev === nextAction ? prev : nextAction));
		setEntityTypeFilter((prev) =>
			prev === nextEntityType ? prev : nextEntityType,
		);
		setSortBy((prev) => (prev === nextSortBy ? prev : nextSortBy));
		setSortOrder((prev) => (prev === nextSortOrder ? prev : nextSortOrder));
	}, [searchParams]);

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
		const currentSearch = getManagedAuditSearchString(searchParams);

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
		sortOrder,
	]);

	const { error, items, loading, reload, total } = useApiList(
		() =>
			adminAuditService.list({
				action: actionFilter.trim() || undefined,
				entity_type:
					entityTypeFilter === "__all__" ? undefined : entityTypeFilter,
				limit: pageSize,
				offset,
				sort_by: sortBy,
				sort_order: sortOrder,
			}),
		[actionFilter, entityTypeFilter, offset, pageSize, sortBy, sortOrder],
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
			label: t("admin.auditPage.filters.allTypes"),
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
		setActionFilter("");
		setEntityTypeFilter("__all__");
		setOffset(0);
	}, [setOffset]);

	const handlePageSizeChange = useCallback(
		(value: string | null) => {
			const next = parsePageSizeOption(value, AUDIT_PAGE_SIZE_OPTIONS);
			if (next == null) {
				return;
			}
			setPageSize(next);
			setOffset(0);
		},
		[setOffset],
	);

	const handleActionFilterChange = (value: string) => {
		setActionFilter(value);
		setOffset(0);
	};

	const handleEntityTypeFilterChange = (value: string | null) => {
		if (!value) {
			return;
		}
		setEntityTypeFilter(isAuditEntityType(value) ? value : "__all__");
		setOffset(0);
	};

	const handleSortChange = useCallback(
		(nextSortBy: AuditLogSortBy, nextOrder: SortOrder) => {
			setSortBy(nextSortBy);
			setSortOrder(nextOrder);
			setOffset(0);
		},
		[setOffset],
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
		<AdminFilterToolbar
			activeFilterCount={activeFilterCount}
			inline
			onResetFilters={resetFilters}
		>
			<div className="relative min-w-[14rem] flex-1 md:max-w-sm">
				<Icon
					name="MagnifyingGlass"
					className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
				/>
				<Input
					value={actionFilter}
					onChange={(event) => handleActionFilterChange(event.target.value)}
					placeholder={t("admin.auditPage.filters.actionPlaceholder")}
					className="pl-9"
				/>
			</div>
			<Select
				items={entityTypeOptions}
				value={entityTypeFilter}
				onValueChange={handleEntityTypeFilterChange}
			>
				<SelectTrigger width="compact" aria-label={t("admin.auditPage.entity")}>
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

	const headerRow = useMemo(
		() => (
			<AdminTableHeader>
				<AdminTableRow>
					<AdminSortableTableHead
						className="w-[12rem]"
						onSortChange={handleSortChange}
						sortBy={sortBy}
						sortKey="created_at"
						sortOrder={sortOrder}
					>
						{t("admin.auditPage.created")}
					</AdminSortableTableHead>
					<AdminSortableTableHead
						className="w-[11rem]"
						onSortChange={handleSortChange}
						sortBy={sortBy}
						sortKey="user_id"
						sortOrder={sortOrder}
					>
						{t("admin.auditPage.user")}
					</AdminSortableTableHead>
					<AdminSortableTableHead
						className="w-[15rem]"
						onSortChange={handleSortChange}
						sortBy={sortBy}
						sortKey="action"
						sortOrder={sortOrder}
					>
						{t("admin.auditPage.action")}
					</AdminSortableTableHead>
					<AdminSortableTableHead
						className="w-[10rem]"
						onSortChange={handleSortChange}
						sortBy={sortBy}
						sortKey="entity_type"
						sortOrder={sortOrder}
					>
						{t("admin.auditPage.entity")}
					</AdminSortableTableHead>
					<AdminSortableTableHead
						onSortChange={handleSortChange}
						sortBy={sortBy}
						sortKey="entity_name"
						sortOrder={sortOrder}
					>
						{t("common.name")}
					</AdminSortableTableHead>
					<AdminSortableTableHead
						className="w-[10rem]"
						onSortChange={handleSortChange}
						sortBy={sortBy}
						sortKey="ip_address"
						sortOrder={sortOrder}
					>
						IP
					</AdminSortableTableHead>
				</AdminTableRow>
			</AdminTableHeader>
		),
		[handleSortChange, sortBy, sortOrder, t],
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
				title={t("admin.auditPage.title")}
				description={t("admin.auditPage.description")}
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

			{error && items.length === 0 ? (
				<AdminSurface padded={false}>
					<EmptyState
						icon={<Icon name="CircleAlert" className="size-5" />}
						title={t("admin.auditPage.loadErrorTitle")}
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
					columns={6}
					emptyDescription={t("admin.auditPage.emptyDescription")}
					emptyIcon={emptyIcon}
					emptyTitle={t("admin.auditPage.emptyTitle")}
					filtered={hasServerFilters}
					filteredEmptyAction={filteredEmptyAction}
					filteredEmptyDescription={t(
						"admin.auditPage.filteredEmptyDescription",
					)}
					filteredEmptyTitle={t("admin.auditPage.filteredEmptyTitle")}
					headerRow={headerRow}
					items={items}
					loading={loading}
					pagination={pagination}
					renderRow={(item) => {
						const detail = formatAuditDetail(t, item);
						const locale = i18n.language?.startsWith("zh") ? "zh-CN" : "en-US";
						return (
							<AdminTableRow key={item.id}>
								<AdminTableCell>
									<div className="grid gap-1">
										<span
											className="text-sm text-foreground"
											title={item.created_at}
										>
											{formatTimestamp(item.created_at, locale)}
										</span>
										<span className={ADMIN_TABLE_MONO_TEXT_CLASS}>
											#{item.id}
										</span>
									</div>
								</AdminTableCell>
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
												{t("admin.auditPage.noDetail")}
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
					}}
					rows={6}
				/>
			)}
		</AdminPageShell>
	);
}
