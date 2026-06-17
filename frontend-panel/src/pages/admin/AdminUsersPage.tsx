import { useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { AdminUserFiltersToolbar } from "@/components/admin/admin-users-page/AdminUserFiltersToolbar";
import { UserDialog } from "@/components/admin/admin-users-page/UserDialog";
import {
	UsersTableHeader,
	UsersTableRow,
} from "@/components/admin/admin-users-page/UsersTable";
import {
	DEFAULT_SORT_BY,
	DEFAULT_SORT_ORDER,
	DEFAULT_USER_PAGE_SIZE,
	USER_PAGE_SIZE_OPTIONS,
	type UserFilterValue,
	useAdminUsersPageState,
} from "@/components/admin/admin-users-page/useAdminUsersPageState";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import { parsePageSizeOption } from "@/lib/pagination";
import { cn } from "@/lib/utils";
import { adminUserPath } from "@/routes/routePaths";
import { adminUserService } from "@/services/adminService";
import type { CreateAdminUserRequest, UserRole, UserStatus } from "@/types/api";

export default function AdminUsersPage() {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const [searchParams, setSearchParams] = useSearchParams();

	usePageTitle(t("admin.users.title"));

	const [state, dispatch] = useAdminUsersPageState(searchParams);
	const {
		createDialogOpen,
		debouncedKeyword,
		items,
		keyword,
		loading,
		offset,
		pageSize,
		revokingId,
		revokingUser,
		role,
		sortBy,
		sortOrder,
		status,
		submitting,
		total,
	} = state;
	const currentPage = Math.floor(offset / pageSize) + 1;
	const totalPages = Math.max(1, Math.ceil(total / pageSize));
	const roleOptions = useMemo(
		() => [
			{ label: t("admin.users.allRoles"), value: "__all__" },
			{ label: t("admin.users.role.admin"), value: "admin" },
			{ label: t("admin.users.role.user"), value: "user" },
		],
		[t],
	);
	const statusOptions = useMemo(
		() => [
			{ label: t("admin.users.allStatuses"), value: "__all__" },
			{ label: t("admin.users.status.active"), value: "active" },
			{ label: t("admin.users.status.disabled"), value: "disabled" },
		],
		[t],
	);
	const pageSizeOptions = USER_PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));
	const filtered =
		Boolean(debouncedKeyword.trim()) ||
		role !== "__all__" ||
		status !== "__all__";
	const activeFilterCount =
		(keyword.trim() ? 1 : 0) +
		(role !== "__all__" ? 1 : 0) +
		(status !== "__all__" ? 1 : 0);
	const toolbar = useMemo(
		() => (
			<AdminUserFiltersToolbar
				activeFilterCount={activeFilterCount}
				keyword={keyword}
				role={role}
				roleOptions={roleOptions}
				status={status}
				statusOptions={statusOptions}
				onKeywordChange={(value) => {
					dispatch({ type: "keyword", value });
				}}
				onRoleChange={(value) => {
					dispatch({
						type: "role",
						value: (value as UserFilterValue<UserRole> | null) ?? "__all__",
					});
				}}
				onStatusChange={(value) => {
					dispatch({
						type: "status",
						value: (value as UserFilterValue<UserStatus> | null) ?? "__all__",
					});
				}}
				onResetFilters={() => dispatch({ type: "resetFilters" })}
			/>
		),
		[
			activeFilterCount,
			dispatch,
			keyword,
			role,
			roleOptions,
			status,
			statusOptions,
		],
	);
	const emptyIcon = useMemo(() => <Icon name="User" className="size-5" />, []);
	const headerRow = useMemo(
		() => (
			<UsersTableHeader
				sortBy={sortBy}
				sortOrder={sortOrder}
				onSortChange={(nextSortBy, nextSortOrder) => {
					dispatch({
						type: "sort",
						sortBy: nextSortBy,
						sortOrder: nextSortOrder,
					});
				}}
			/>
		),
		[dispatch, sortBy, sortOrder],
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
					dispatch({
						type: "offset",
						value: (current) => Math.max(0, current - pageSize),
					})
				}
				onNext={() =>
					dispatch({ type: "offset", value: (current) => current + pageSize })
				}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(value, USER_PAGE_SIZE_OPTIONS);
					if (next == null) return;
					dispatch({ type: "pageSize", value: next });
				}}
			/>
		),
		[
			dispatch,
			currentPage,
			offset,
			pageSize,
			pageSizeOptions,
			total,
			totalPages,
		],
	);

	useEffect(() => {
		const timer = window.setTimeout(() => {
			dispatch({ type: "debouncedKeyword", value: keyword });
		}, 250);
		return () => window.clearTimeout(timer);
	}, [dispatch, keyword]);

	useEffect(() => {
		const next = new URLSearchParams(searchParams);
		setOrDelete(next, "keyword", debouncedKeyword.trim());
		setOrDelete(next, "role", role === "__all__" ? "" : role);
		setOrDelete(next, "status", status === "__all__" ? "" : status);
		setOrDelete(next, "offset", offset > 0 ? String(offset) : "");
		setOrDelete(
			next,
			"pageSize",
			pageSize !== DEFAULT_USER_PAGE_SIZE ? String(pageSize) : "",
		);
		setOrDelete(next, "sortBy", sortBy !== DEFAULT_SORT_BY ? sortBy : "");
		setOrDelete(
			next,
			"sortOrder",
			sortOrder !== DEFAULT_SORT_ORDER ? sortOrder : "",
		);
		if (next.toString() !== searchParams.toString()) {
			setSearchParams(next, { replace: true });
		}
	}, [
		debouncedKeyword,
		offset,
		pageSize,
		role,
		searchParams,
		setSearchParams,
		sortBy,
		sortOrder,
		status,
	]);

	const loadUsers = useCallback(async () => {
		try {
			dispatch({ type: "loadStart" });
			const page = await adminUserService.list({
				keyword: debouncedKeyword,
				limit: pageSize,
				offset,
				role: role === "__all__" ? undefined : role,
				sort_by: sortBy,
				sort_order: sortOrder,
				status: status === "__all__" ? undefined : status,
			});
			if (page.items.length === 0 && page.total > 0 && offset > 0) {
				dispatch({
					type: "offset",
					value: Math.max(
						0,
						Math.floor((page.total - 1) / pageSize) * pageSize,
					),
				});
				return;
			}
			dispatch({ type: "loadSuccess", items: page.items, total: page.total });
		} catch (error) {
			handleApiError(error);
			dispatch({ type: "loading", value: false });
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}, [
		debouncedKeyword,
		dispatch,
		offset,
		pageSize,
		role,
		sortBy,
		sortOrder,
		status,
	]);

	useEffect(() => {
		void loadUsers();
	}, [loadUsers]);

	async function createUser(data: CreateAdminUserRequest) {
		try {
			dispatch({ type: "submitting", value: true });
			await adminUserService.create(data);
			toast.success(t("admin.users.created"));
			dispatch({ type: "createDialogOpen", value: false });
			await loadUsers();
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	async function revokeUserSessions(id: number) {
		const result = await adminUserService.revokeSessions(id);
		await loadUsers();
		return result.removed;
	}

	async function revokeSessions() {
		if (!revokingUser) return;
		try {
			dispatch({ type: "revokingId", value: revokingUser.id });
			const removed = await revokeUserSessions(revokingUser.id);
			toast.success(t("admin.users.sessionsRevoked", { count: removed }));
			dispatch({ type: "revokingUser", value: null });
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "revokingId", value: null });
		}
	}

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.users.title")}
				description={t("admin.users.description")}
				actions={
					<>
						<Button
							type="button"
							size="sm"
							onClick={() => {
								dispatch({ type: "createDialogOpen", value: true });
							}}
						>
							<Icon name="Plus" className="mr-2 size-4" />
							{t("admin.users.create")}
						</Button>
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={loading}
							onClick={() => void loadUsers()}
						>
							<Icon
								name={loading ? "Spinner" : "ArrowsClockwise"}
								className={cn("mr-2 size-4", loading && "animate-spin")}
							/>
							{t("common.refresh")}
						</Button>
					</>
				}
				toolbar={toolbar}
			/>
			<AdminTableList
				loading={loading}
				items={items}
				columns={6}
				rows={6}
				filtered={filtered}
				emptyIcon={emptyIcon}
				emptyTitle={t("admin.users.emptyTitle")}
				emptyDescription={t("admin.users.emptyDescription")}
				filteredEmptyTitle={t("admin.users.filteredEmptyTitle")}
				filteredEmptyDescription={t("admin.users.filteredEmptyDescription")}
				headerRow={headerRow}
				pagination={pagination}
				renderRow={(user) => (
					<UsersTableRow
						key={user.id}
						user={user}
						revokingId={revokingId}
						onEdit={(item) => {
							void navigate(adminUserPath(item.id));
						}}
						onRevokeSessions={(user) =>
							dispatch({ type: "revokingUser", value: user })
						}
					/>
				)}
			/>
			<UserDialog
				open={createDialogOpen}
				submitting={submitting}
				onOpenChange={(open) => {
					dispatch({ type: "createDialogOpen", value: open });
				}}
				onSubmit={(data) => void createUser(data)}
			/>
			<ConfirmDialog
				open={Boolean(revokingUser)}
				onOpenChange={(open) => {
					if (!open) dispatch({ type: "revokingUser", value: null });
				}}
				title={t("admin.users.revokeSessionsTitle", {
					name: revokingUser?.username ?? "",
				})}
				description={t("admin.users.revokeSessionsDescription")}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("admin.users.revokeSessions")}
				loading={revokingId != null}
				onConfirm={() => void revokeSessions()}
			/>
		</AdminPageShell>
	);
}

function setOrDelete(params: URLSearchParams, key: string, value: string) {
	if (value) params.set(key, value);
	else params.delete(key);
}
