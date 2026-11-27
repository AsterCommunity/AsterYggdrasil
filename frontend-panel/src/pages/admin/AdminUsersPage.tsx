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
	DEFAULT_USER_PAGE_SIZE,
	USER_PAGE_SIZE_OPTIONS,
	type UserFilterValue,
} from "@/components/admin/admin-users-page/userPageFilters";
import { UsersSectionNav } from "@/components/admin/UsersSectionNav";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { RefreshButton } from "@/components/common/RefreshButton";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { handleApiError } from "@/hooks/useApiError";
import { useClipboard } from "@/hooks/useClipboard";
import { useConfirmDialog } from "@/hooks/useConfirmDialog";
import { useCursorList } from "@/hooks/useCursorList";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import {
	managedEnumField,
	managedPageSizeField,
	managedStringField,
	useManagedListQueryState,
} from "@/hooks/useManagedListQueryState";
import { usePageTitle } from "@/hooks/usePageTitle";
import { usePendingAction } from "@/hooks/usePendingAction";
import { usePendingId } from "@/hooks/usePendingId";
import { parsePageSizeOption } from "@/lib/pagination";
import { inviteUserSchema } from "@/lib/validation";
import { adminUserPath } from "@/routes/routePaths";
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
	open: boolean;
};

type InviteAction =
	| { type: "open" }
	| { type: "close" }
	| { type: "field"; value: string }
	| { type: "error"; value: string | null }
	| { type: "created"; value: AdminUserInvitationInfo };

type GeneratedPasswordState = {
	password: string | null;
	username: string;
};

type ManagedUsersQuery = {
	keyword: string;
	pageSize: (typeof USER_PAGE_SIZE_OPTIONS)[number];
	role: UserFilterValue<UserRole>;
	status: UserFilterValue<UserStatus>;
};

type UserCursor = { id: number; value: string };

function createInviteForm(): CreateUserInvitationRequest {
	return { email: "" };
}

function initialInviteState(): InviteState {
	return {
		createdInvitation: null,
		error: null,
		form: createInviteForm(),
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
	const [createDialogOpen, setCreateDialogOpen] = useState(false);
	const [keywordDraft, setKeywordDraft] = useState("");

	usePageTitle(t("admin.users.title"));

	const { query, setQuery } = useManagedListQueryState<ManagedUsersQuery>({
		schema: {
			keyword: managedStringField("keyword"),
			pageSize: managedPageSizeField(
				USER_PAGE_SIZE_OPTIONS,
				DEFAULT_USER_PAGE_SIZE,
			),
			role: managedEnumField("role", ["admin", "operator", "user"], "__all__"),
			status: managedEnumField("status", ["active", "disabled"], "__all__"),
		},
		searchParams,
		setSearchParams,
	});
	const { keyword: committedKeyword, pageSize, role, status } = query;
	// The input keeps intermediate drafts; the URL commits after a debounce.
	const debouncedKeyword = useDebouncedValue(keywordDraft, 250);
	useEffect(() => {
		if (debouncedKeyword.trim() === committedKeyword) return;
		setQuery({ keyword: debouncedKeyword });
	}, [committedKeyword, debouncedKeyword, setQuery]);

	const { pending: submitting, runWithPending } = usePendingAction();
	const { pendingId: deletingId, runWithPending: runDeletion } =
		usePendingId<number>();
	const { pendingId: revokingId, runWithPending: runRevocation } =
		usePendingId<number>();

	const loadUserPage = useCallback(
		async ({ cursor }: { cursor: UserCursor | null }) => {
			const page = await adminUserService.list({
				keyword: committedKeyword,
				limit: pageSize,
				after_created_at: cursor?.value,
				after_id: cursor?.id,
				role: role === "__all__" ? undefined : role,
				status: status === "__all__" ? undefined : status,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[committedKeyword, pageSize, role, status],
	);
	const list = useCursorList<AdminUserInfo, UserCursor>({
		deps: [committedKeyword, pageSize, role, status],
		loadPage: loadUserPage,
		onError: handleApiError,
	});

	const deleteDialog = useConfirmDialog<AdminUserInfo>((target) => {
		void runDeletion(target.id, async () => {
			try {
				if (target.id !== 1) {
					await adminUserService.delete(target.id);
					toast.success(t("admin.users.deleted"));
				}
				await list.reload();
			} catch (error) {
				handleApiError(error);
			}
		});
	});
	const revokeDialog = useConfirmDialog<AdminUserInfo>((target) => {
		void runRevocation(target.id, async () => {
			try {
				const result = await adminUserService.revokeSessions(target.id);
				toast.success(
					t("admin.users.sessionsRevoked", { count: result.removed }),
				);
			} catch (error) {
				handleApiError(error);
			}
		});
	});

	const { pending: inviting, runWithPending: runInvite } = usePendingAction();
	const { copy } = useClipboard();

	const roleOptions = useMemo(
		() => [
			{ label: t("admin.users.allRoles"), value: "__all__" },
			{ label: t("admin.users.role.admin"), value: "admin" },
			{ label: t("admin.users.role.operator"), value: "operator" },
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
		Boolean(committedKeyword) || role !== "__all__" || status !== "__all__";
	const activeFilterCount =
		(keywordDraft.trim() ? 1 : 0) +
		(role !== "__all__" ? 1 : 0) +
		(status !== "__all__" ? 1 : 0);
	const toolbar = useMemo(
		() => (
			<AdminUserFiltersToolbar
				activeFilterCount={activeFilterCount}
				keyword={keywordDraft}
				role={role}
				roleOptions={roleOptions}
				status={status}
				statusOptions={statusOptions}
				onKeywordChange={setKeywordDraft}
				onRoleChange={(value) => {
					setQuery({
						role: (value as UserFilterValue<UserRole> | null) ?? "__all__",
					});
				}}
				onStatusChange={(value) => {
					setQuery({
						status: (value as UserFilterValue<UserStatus> | null) ?? "__all__",
					});
				}}
				onResetFilters={() => {
					setKeywordDraft("");
					setQuery({
						keyword: "",
						role: "__all__",
						status: "__all__",
					});
				}}
			/>
		),
		[
			activeFilterCount,
			keywordDraft,
			role,
			roleOptions,
			setQuery,
			status,
			statusOptions,
		],
	);
	const emptyIcon = useMemo(() => <Icon name="User" className="size-5" />, []);
	const headerRow = useMemo(() => <UsersTableHeader />, []);
	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				total={list.total}
				currentPage={list.currentPage}
				totalPages={list.totalPages}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={list.prevPageDisabled}
				nextDisabled={list.nextPageDisabled}
				onPrevious={list.goPrevious}
				onNext={list.goNext}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(value, USER_PAGE_SIZE_OPTIONS);
					if (next == null) return;
					setQuery({ pageSize: next });
				}}
			/>
		),
		[
			list.goNext,
			list.goPrevious,
			list.nextPageDisabled,
			list.prevPageDisabled,
			list.currentPage,
			list.totalPages,
			list.total,
			pageSize,
			pageSizeOptions,
			setQuery,
		],
	);

	async function createUser(data: CreateAdminUserRequest) {
		await runWithPending(async () => {
			try {
				const result = await adminUserService.create(data);
				toast.success(t("admin.users.created"));
				setCreateDialogOpen(false);
				if (result.generated_password) {
					setGeneratedPassword({
						password: result.generated_password,
						username: result.user.username,
					});
				}
				await list.reloadFirstPage();
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	function closeInviteDialog() {
		if (inviting) return;
		dispatchInvite({ type: "close" });
	}

	async function copyInvitationLink(value: string) {
		if (!value) return;
		if (await copy(value)) {
			toast.success(t("common.copied"));
		} else {
			toast.error(t("errors.unexpected_error"));
		}
	}

	async function copyGeneratedPassword() {
		if (!generatedPassword?.password) return;
		if (await copy(generatedPassword.password)) {
			toast.success(t("common.copied"));
		} else {
			toast.error(t("errors.unexpected_error"));
		}
	}

	async function createInvitation(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const parsed = inviteUserSchema.safeParse({
			email: inviteState.form.email.trim(),
		});
		if (!parsed.success) {
			dispatchInvite({
				type: "error",
				value: t(parsed.error.issues[0]?.message ?? ""),
			});
			return;
		}
		await runInvite(async () => {
			try {
				const invitation = await adminUserService.createInvitation({
					email: parsed.data.email,
				});
				dispatchInvite({ type: "created", value: invitation });
				toast.success(t("admin.users.invitationCreated"));
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	return {
		createDialogOpen,
		deleteDialog,
		revokeDialog,
		emptyIcon,
		filtered,
		generatedPassword,
		headerRow,
		inviting,
		inviteState,
		keywordDraft,
		list,
		pagination,
		revokingId,
		deletingId,
		dispatchInvite,
		setCreateDialogOpen,
		setGeneratedPassword,
		setKeywordDraft,
		submitting,
		toolbar,
		actions: {
			closeInviteDialog,
			copyInvitationLink,
			copyGeneratedPassword,
			createInvitation,
			createUser,
			reload: () => list.reload(),
			navigateToUser: (user: AdminUserInfo) => navigate(adminUserPath(user.id)),
			openInviteDialog: () => dispatchInvite({ type: "open" }),
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
	const { actions, list, toolbar } = controller;

	return (
		<AdminPageHeader
			title={t("admin.users.title")}
			description={t("admin.users.description")}
			actions={
				<AdminUsersPageActions
					loading={list.loading}
					onCreate={() => controller.setCreateDialogOpen(true)}
					onInvite={actions.openInviteDialog}
					onRefresh={actions.reload}
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
	onRefresh,
}: {
	loading: boolean;
	onCreate: () => void;
	onInvite: () => void;
	onRefresh: () => void;
}) {
	const { t } = useTranslation();

	return (
		<>
			<UsersSectionNav active="users" />
			<Button type="button" variant="outline" size="sm" onClick={onInvite}>
				<Icon name="EnvelopeSimple" className="mr-2 size-4" />
				{t("admin.users.inviteUser")}
			</Button>
			<Button type="button" size="sm" onClick={onCreate}>
				<Icon name="Plus" className="mr-2 size-4" />
				{t("admin.users.create")}
			</Button>
			<RefreshButton loading={loading} onClick={onRefresh} />
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
		deleteDialog,
		deletingId,
		emptyIcon,
		filtered,
		headerRow,
		list,
		pagination,
		revokingId,
		revokeDialog,
	} = controller;

	return (
		<AdminTableList
			loading={list.loading}
			items={list.items}
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
					deletingId={deletingId}
					revokingId={revokingId}
					onEdit={(item) => {
						void actions.navigateToUser(item);
					}}
					onDelete={deleteDialog.requestConfirm}
					onRevokeSessions={revokeDialog.requestConfirm}
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
		createDialogOpen,
		deleteDialog,
		deletingId,
		dispatchInvite,
		generatedPassword,
		inviteState,
		inviting,
		revokingId,
		revokeDialog,
		setCreateDialogOpen,
		setGeneratedPassword,
		submitting,
	} = controller;

	return (
		<>
			<UserDialog
				open={createDialogOpen}
				submitting={submitting}
				onOpenChange={setCreateDialogOpen}
				onSubmit={(data) => void actions.createUser(data)}
			/>
			<InviteUserDialog
				open={inviteState.open}
				form={inviteState.form}
				error={inviteState.error}
				inviting={inviting}
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
					if (!open) setGeneratedPassword(null);
				}}
			/>
			<ConfirmDialog
				open={revokeDialog.dialogProps.open}
				onOpenChange={revokeDialog.dialogProps.onOpenChange}
				title={t("admin.users.revokeSessionsTitle", {
					name: revokeDialog.confirmTarget?.username ?? "",
				})}
				description={t("admin.users.revokeSessionsDescription")}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("admin.users.revokeSessions")}
				loading={revokingId != null}
				onConfirm={revokeDialog.dialogProps.onConfirm}
			/>
			<ConfirmDialog
				open={deleteDialog.dialogProps.open}
				onOpenChange={deleteDialog.dialogProps.onOpenChange}
				title={t("admin.users.deleteTitle", {
					name: deleteDialog.confirmTarget?.username ?? "",
				})}
				description={t("admin.users.deleteDescription")}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("admin.users.delete")}
				loading={deletingId != null}
				variant="destructive"
				onConfirm={deleteDialog.dialogProps.onConfirm}
			/>
		</>
	);
}
