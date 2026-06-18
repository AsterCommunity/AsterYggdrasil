import type { FormEvent } from "react";
import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { AdminUserFiltersToolbar } from "@/components/admin/admin-users-page/AdminUserFiltersToolbar";
import { GeneratedPasswordDialog } from "@/components/admin/admin-users-page/GeneratedPasswordDialog";
import { InviteUserDialog } from "@/components/admin/admin-users-page/InviteUserDialog";
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
import { adminPaths, adminUserPath } from "@/routes/routePaths";
import { adminUserService } from "@/services/adminService";
import type {
	AdminUserInfo,
	AdminUserInvitationInfo,
	CreateAdminUserRequest,
	CreateUserInvitationRequest,
	UserRole,
	UserStatus,
} from "@/types/api";

type InviteState = {
	createdInvitation: AdminUserInvitationInfo | null;
	error: string | null;
	form: CreateUserInvitationRequest;
	inviting: boolean;
	open: boolean;
};

type InviteAction =
	| { type: "open" }
	| { type: "close" }
	| { type: "field"; value: string }
	| { type: "error"; value: string | null }
	| { type: "inviting"; value: boolean }
	| { type: "created"; value: AdminUserInvitationInfo };

type GeneratedPasswordState = {
	password: string | null;
	username: string;
};

function isValidEmail(value: string) {
	return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

function createInviteForm(): CreateUserInvitationRequest {
	return { email: "" };
}

function initialInviteState(): InviteState {
	return {
		createdInvitation: null,
		error: null,
		form: createInviteForm(),
		inviting: false,
		open: false,
	};
}

function inviteReducer(state: InviteState, action: InviteAction): InviteState {
	switch (action.type) {
		case "open":
			return { ...state, open: true };
		case "close":
			return initialInviteState();
		case "field":
			return {
				...state,
				createdInvitation: null,
				error: null,
				form: { email: action.value },
			};
		case "error":
			return { ...state, error: action.value };
		case "inviting":
			return { ...state, inviting: action.value };
		case "created":
			return {
				...state,
				createdInvitation: action.value,
				form: { email: action.value.email },
			};
	}
}

export default function AdminUsersPage() {
	const controller = useAdminUsersPageController();
	return <AdminUsersPageLayout controller={controller} />;
}

function useAdminUsersPageController() {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const [searchParams, setSearchParams] = useSearchParams();
	const [inviteState, dispatchInvite] = useReducer(
		inviteReducer,
		undefined,
		initialInviteState,
	);
	const [generatedPassword, setGeneratedPassword] =
		useState<GeneratedPasswordState | null>(null);

	usePageTitle(t("admin.users.title"));

	const [state, dispatch] = useAdminUsersPageState(searchParams);
	const {
		debouncedKeyword,
		keyword,
		offset,
		pageSize,
		role,
		sortBy,
		sortOrder,
		status,
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
			const result = await adminUserService.create(data);
			toast.success(t("admin.users.created"));
			dispatch({ type: "createDialogOpen", value: false });
			if (result.generated_password) {
				setGeneratedPassword({
					password: result.generated_password,
					username: result.user.username,
				});
			}
			await loadUsers();
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	function closeInviteDialog() {
		if (inviteState.inviting) return;
		dispatchInvite({ type: "close" });
	}

	async function copyInvitationLink(value: string) {
		if (!value) return;
		try {
			await navigator.clipboard.writeText(value);
			toast.success(t("common.copied"));
		} catch (error) {
			handleApiError(error);
		}
	}

	async function copyGeneratedPassword() {
		if (!generatedPassword?.password) return;
		try {
			await navigator.clipboard.writeText(generatedPassword.password);
			toast.success(t("common.copied"));
		} catch (error) {
			handleApiError(error);
		}
	}

	async function createInvitation(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const email = inviteState.form.email.trim();
		if (!isValidEmail(email)) {
			dispatchInvite({
				type: "error",
				value: t("admin.users.inviteEmailInvalid"),
			});
			return;
		}
		try {
			dispatchInvite({ type: "inviting", value: true });
			const invitation = await adminUserService.createInvitation({ email });
			dispatchInvite({ type: "created", value: invitation });
			toast.success(t("admin.users.invitationCreated"));
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatchInvite({ type: "inviting", value: false });
		}
	}

	async function revokeUserSessions(id: number) {
		const result = await adminUserService.revokeSessions(id);
		await loadUsers();
		return result.removed;
	}

	async function revokeSessions() {
		if (!state.revokingUser) return;
		try {
			dispatch({ type: "revokingId", value: state.revokingUser.id });
			const removed = await revokeUserSessions(state.revokingUser.id);
			toast.success(t("admin.users.sessionsRevoked", { count: removed }));
			dispatch({ type: "revokingUser", value: null });
		} catch (error) {
			handleApiError(error);
		} finally {
			dispatch({ type: "revokingId", value: null });
		}
	}

	return {
		dispatch,
		dispatchInvite,
		emptyIcon,
		filtered,
		generatedPassword,
		headerRow,
		inviteState,
		pagination,
		state,
		toolbar,
		actions: {
			closeInviteDialog,
			copyInvitationLink,
			copyGeneratedPassword,
			createInvitation,
			createUser,
			loadUsers,
			navigateToInvitations: () => navigate(adminPaths.userInvitations),
			navigateToUser: (user: AdminUserInfo) => navigate(adminUserPath(user.id)),
			openCreateDialog: () =>
				dispatch({ type: "createDialogOpen", value: true }),
			openInviteDialog: () => dispatchInvite({ type: "open" }),
			setGeneratedPassword,
			revokeSessions,
		},
	};
}

type AdminUsersController = ReturnType<typeof useAdminUsersPageController>;

function AdminUsersPageLayout({
	controller,
}: {
	controller: AdminUsersController;
}) {
	return (
		<AdminPageShell>
			<AdminUsersPageHeader controller={controller} />
			<AdminUsersTableSection controller={controller} />
			<AdminUsersDialogs controller={controller} />
		</AdminPageShell>
	);
}

function AdminUsersPageHeader({
	controller,
}: {
	controller: AdminUsersController;
}) {
	const { t } = useTranslation();
	const { actions, state, toolbar } = controller;

	return (
		<AdminPageHeader
			title={t("admin.users.title")}
			description={t("admin.users.description")}
			actions={
				<AdminUsersPageActions
					loading={state.loading}
					onCreate={actions.openCreateDialog}
					onInvite={actions.openInviteDialog}
					onOpenInvitations={actions.navigateToInvitations}
					onRefresh={() => void actions.loadUsers()}
				/>
			}
			toolbar={toolbar}
		/>
	);
}

function AdminUsersPageActions({
	loading,
	onCreate,
	onInvite,
	onOpenInvitations,
	onRefresh,
}: {
	loading: boolean;
	onCreate: () => void;
	onInvite: () => void;
	onOpenInvitations: () => void;
	onRefresh: () => void;
}) {
	const { t } = useTranslation();

	return (
		<>
			<Button type="button" variant="outline" size="sm" onClick={onInvite}>
				<Icon name="EnvelopeSimple" className="mr-2 size-4" />
				{t("admin.users.inviteUser")}
			</Button>
			<Button
				type="button"
				variant="outline"
				size="sm"
				onClick={onOpenInvitations}
			>
				<Icon name="ListBullets" className="mr-2 size-4" />
				{t("admin.users.invitationRecords")}
			</Button>
			<Button type="button" size="sm" onClick={onCreate}>
				<Icon name="Plus" className="mr-2 size-4" />
				{t("admin.users.create")}
			</Button>
			<Button
				type="button"
				variant="outline"
				size="sm"
				disabled={loading}
				onClick={onRefresh}
			>
				<Icon
					name={loading ? "Spinner" : "ArrowsClockwise"}
					className={cn("mr-2 size-4", loading && "animate-spin")}
				/>
				{t("common.refresh")}
			</Button>
		</>
	);
}

function AdminUsersTableSection({
	controller,
}: {
	controller: AdminUsersController;
}) {
	const { t } = useTranslation();
	const {
		actions,
		dispatch,
		emptyIcon,
		filtered,
		headerRow,
		pagination,
		state,
	} = controller;

	return (
		<AdminTableList
			loading={state.loading}
			items={state.items}
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
					revokingId={state.revokingId}
					onEdit={(item) => {
						void actions.navigateToUser(item);
					}}
					onRevokeSessions={(item) =>
						dispatch({ type: "revokingUser", value: item })
					}
				/>
			)}
		/>
	);
}

function AdminUsersDialogs({
	controller,
}: {
	controller: AdminUsersController;
}) {
	const { t } = useTranslation();
	const {
		actions,
		dispatch,
		dispatchInvite,
		generatedPassword,
		inviteState,
		state,
	} = controller;

	return (
		<>
			<UserDialog
				open={state.createDialogOpen}
				submitting={state.submitting}
				onOpenChange={(open) => {
					dispatch({ type: "createDialogOpen", value: open });
				}}
				onSubmit={(data) => void actions.createUser(data)}
			/>
			<InviteUserDialog
				open={inviteState.open}
				form={inviteState.form}
				error={inviteState.error}
				inviting={inviteState.inviting}
				createdInvitation={inviteState.createdInvitation}
				onFieldChange={(value) => dispatchInvite({ type: "field", value })}
				onCopyLink={(value) => void actions.copyInvitationLink(value)}
				onSubmit={(event) => void actions.createInvitation(event)}
				onOpenChange={(open) => {
					if (open) {
						dispatchInvite({ type: "open" });
					} else {
						actions.closeInviteDialog();
					}
				}}
			/>
			<GeneratedPasswordDialog
				open={Boolean(generatedPassword)}
				password={generatedPassword?.password ?? null}
				username={generatedPassword?.username ?? ""}
				onCopy={() => void actions.copyGeneratedPassword()}
				onOpenChange={(open) => {
					if (!open) actions.setGeneratedPassword(null);
				}}
			/>
			<ConfirmDialog
				open={Boolean(state.revokingUser)}
				onOpenChange={(open) => {
					if (!open) dispatch({ type: "revokingUser", value: null });
				}}
				title={t("admin.users.revokeSessionsTitle", {
					name: state.revokingUser?.username ?? "",
				})}
				description={t("admin.users.revokeSessionsDescription")}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("admin.users.revokeSessions")}
				loading={state.revokingId != null}
				onConfirm={() => void actions.revokeSessions()}
			/>
		</>
	);
}

function setOrDelete(params: URLSearchParams, key: string, value: string) {
	if (value) params.set(key, value);
	else params.delete(key);
}
