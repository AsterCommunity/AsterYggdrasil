import type { TFunction } from "i18next";
import type { ReactNode } from "react";
import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { AdminFilterToolbar } from "@/components/common/AdminFilterToolbar";
import {
	ADMIN_TABLE_MONO_TEXT_CLASS,
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminTableCell,
	AdminTableHead,
	AdminTableHeader,
	AdminTableRow,
} from "@/components/common/AdminTable";
import { AdminTableList } from "@/components/common/AdminTableList";
import { DateTimeText } from "@/components/common/DateTimeText";
import { EmptyState } from "@/components/common/EmptyState";
import { RefreshButton } from "@/components/common/RefreshButton";
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
import { useCursorList } from "@/hooks/useCursorList";
import {
	managedEnumField,
	managedPageSizeField,
	managedStringField,
	useManagedListQueryState,
} from "@/hooks/useManagedListQueryState";
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
import { parsePageSizeOption } from "@/lib/pagination";
import type {
	AuditEntityType,
	AuditLogEntry,
	AuditLogPage as AuditLogPageData,
} from "@/types/api";

const AUDIT_PAGE_SIZE_OPTIONS = [10, 20, 50] as const;
const DEFAULT_AUDIT_PAGE_SIZE = 20 as const;

type AuditEntityTypeFilter = "__all__" | AuditEntityType;
type AuditCursor = { id: number; value: string };

type AuditLogPageProps = {
	list: (query: {
		action?: string;
		after_created_at?: string;
		after_id?: number;
		entity_type?: AuditEntityType;
		limit: number;
	}) => Promise<AuditLogPageData>;
	showActor: boolean;
	translationPrefix: "account.auditPage" | "admin.auditPage";
};

export function AuditLogPage({
	list,
	showActor,
	translationPrefix,
}: AuditLogPageProps) {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t(`${translationPrefix}.title`));

	const { query, setQuery } = useManagedListQueryState<{
		action: string;
		entityType: AuditEntityTypeFilter;
		pageSize: (typeof AUDIT_PAGE_SIZE_OPTIONS)[number];
	}>({
		schema: {
			action: managedStringField("action"),
			entityType: managedEnumField(
				"entityType",
				AUDIT_ENTITY_TYPE_FILTER_VALUES,
				"__all__",
			),
			pageSize: managedPageSizeField(
				AUDIT_PAGE_SIZE_OPTIONS,
				DEFAULT_AUDIT_PAGE_SIZE,
			),
		},
		searchParams,
		setSearchParams,
	});
	const {
		action: actionFilter,
		entityType: entityTypeFilter,
		pageSize,
	} = query;

	const loadAuditPage = useCallback(
		async ({ cursor }: { cursor: AuditCursor | null }) => {
			const page = await list({
				action: actionFilter || undefined,
				after_created_at: cursor?.value,
				after_id: cursor?.id,
				entity_type:
					entityTypeFilter === "__all__" ? undefined : entityTypeFilter,
				limit: pageSize,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[actionFilter, entityTypeFilter, list, pageSize],
	);
	const auditList = useCursorList<AuditLogEntry, AuditCursor>({
		deps: [actionFilter, entityTypeFilter, pageSize],
		loadPage: loadAuditPage,
	});
	const { currentPage, error, totalPages } = auditList;

	const activeFilterCount =
		(actionFilter.length > 0 ? 1 : 0) +
		(entityTypeFilter !== "__all__" ? 1 : 0);
	const hasServerFilters = activeFilterCount > 0;
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
		setQuery({ action: "", entityType: "__all__" });
	}, [setQuery]);

	const handlePageSizeChange = useCallback(
		(value: string | null) => {
			const next = parsePageSizeOption(value, AUDIT_PAGE_SIZE_OPTIONS);
			if (next == null) {
				return;
			}
			setQuery({ pageSize: next });
		},
		[setQuery],
	);

	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				currentPage={currentPage}
				nextDisabled={auditList.nextPageDisabled}
				onNext={auditList.goNext}
				onPageSizeChange={handlePageSizeChange}
				onPrevious={auditList.goPrevious}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={auditList.prevPageDisabled}
				total={auditList.total}
				totalPages={totalPages}
			/>
		),
		[
			currentPage,
			handlePageSizeChange,
			auditList.goNext,
			auditList.goPrevious,
			auditList.nextPageDisabled,
			auditList.prevPageDisabled,
			pageSize,
			pageSizeOptions,
			auditList.total,
			totalPages,
		],
	);

	const toolbar = (
		<AuditFilterToolbar
			actionFilter={actionFilter}
			activeFilterCount={activeFilterCount}
			entityTypeFilter={entityTypeFilter}
			entityTypeOptions={entityTypeOptions}
			translationPrefix={translationPrefix}
			onActionFilterChange={(value) => setQuery({ action: value })}
			onEntityTypeFilterChange={(value) => {
				if (!value) return;
				setQuery({
					entityType: isAuditEntityType(value) ? value : "__all__",
				});
			}}
			onResetFilters={resetFilters}
		/>
	);

	const headerRow = useMemo(
		() => (
			<AuditTableHeader
				showActor={showActor}
				translationPrefix={translationPrefix}
			/>
		),
		[showActor, translationPrefix],
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
					<RefreshButton
						loading={auditList.loading}
						onClick={() => void auditList.reload()}
					/>
				}
				toolbar={toolbar}
			/>

			<AuditLogTable
				emptyIcon={emptyIcon}
				error={error}
				filteredEmptyAction={filteredEmptyAction}
				hasServerFilters={hasServerFilters}
				headerRow={headerRow}
				items={auditList.items}
				loading={auditList.loading}
				pagination={pagination}
				reload={() => auditList.reload()}
				showActor={showActor}
				t={t}
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
	showActor,
	translationPrefix,
}: {
	showActor: boolean;
	translationPrefix: AuditLogPageProps["translationPrefix"];
}) {
	const { t } = useTranslation();

	return (
		<AdminTableHeader>
			<AdminTableRow>
				<AdminTableHead className="w-[12rem]">
					{t(`${translationPrefix}.created`)}
				</AdminTableHead>
				{showActor ? (
					<AdminTableHead className="w-[11rem]">
						{t("admin.auditPage.user")}
					</AdminTableHead>
				) : null}
				<AdminTableHead className="w-[15rem]">
					{t(`${translationPrefix}.action`)}
				</AdminTableHead>
				<AdminTableHead className="w-[10rem]">
					{t(`${translationPrefix}.entity`)}
				</AdminTableHead>
				<AdminTableHead>{t("common.name")}</AdminTableHead>
				<AdminTableHead className="w-[10rem]">IP</AdminTableHead>
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
	translationPrefix,
}: {
	item: AuditLogEntry;
	showActor: boolean;
	t: TFunction;
	translationPrefix: AuditLogPageProps["translationPrefix"];
}) {
	const detail = formatAuditDetail(t, item);

	return (
		<AdminTableRow>
			<AdminTableCell>
				<div className="grid gap-1">
					<DateTimeText
						value={item.created_at}
						className="text-sm text-foreground"
					/>
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
