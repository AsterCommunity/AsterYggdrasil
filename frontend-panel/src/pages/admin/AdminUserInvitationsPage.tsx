import type { FormEvent } from "react";
import { useCallback, useMemo, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { AdminOffsetPagination } from "@/components/admin/AdminOffsetPagination";
import { InviteUserDialog } from "@/components/admin/admin-users-page/InviteUserDialog";
import {
	UserInvitationsTableHeader,
	UserInvitationsTableRow,
} from "@/components/admin/admin-users-page/UserInvitationsTable";
import { UsersSectionNav } from "@/components/admin/UsersSectionNav";
import { AdminTableList } from "@/components/common/AdminTableList";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { handleApiError } from "@/hooks/useApiError";
import { useConfirmDialog } from "@/hooks/useConfirmDialog";
import { useCursorList } from "@/hooks/useCursorList";
import {
	managedPageSizeField,
	useManagedListQueryState,
} from "@/hooks/useManagedListQueryState";
import { usePageTitle } from "@/hooks/usePageTitle";
import { usePendingAction } from "@/hooks/usePendingAction";
import { usePendingId } from "@/hooks/usePendingId";
import { parsePageSizeOption } from "@/lib/pagination";
import { cn } from "@/lib/utils";
import { adminUserService } from "@/services/adminService";
import type {
	AdminUserInvitationInfo,
	CreateUserInvitationRequest,
} from "@/types/api";

const INVITATION_PAGE_SIZE_OPTIONS = [10, 20, 50] as const;
const DEFAULT_INVITATION_PAGE_SIZE = 20;

type InvitationPageSize = (typeof INVITATION_PAGE_SIZE_OPTIONS)[number];
type InvitationCursor = { id: number; value: string };

type InviteState = {
	createdInvitation: AdminUserInvitationInfo | null;
	emailError: string | null;
	form: CreateUserInvitationRequest;
	inviteDialogOpen: boolean;
};

type InviteAction =
	| { type: "closeDialog" }
	| { type: "created"; invitation: AdminUserInvitationInfo }
	| { type: "email"; value: string }
	| { type: "emailError"; value: string | null }
	| { type: "openDialog"; value: boolean };

function createInviteForm(): CreateUserInvitationRequest {
	return { email: "" };
}

function inviteReducer(state: InviteState, action: InviteAction): InviteState {
	switch (action.type) {
		case "closeDialog":
			return {
				createdInvitation: null,
				emailError: null,
				form: createInviteForm(),
				inviteDialogOpen: false,
			};
		case "created":
			return {
				...state,
				createdInvitation: action.invitation,
				form: { email: action.invitation.email },
			};
		case "email":
			return {
				...state,
				createdInvitation: null,
				emailError: null,
				form: { email: action.value },
			};
		case "emailError":
			return { ...state, emailError: action.value };
		case "openDialog":
			return { ...state, inviteDialogOpen: action.value };
	}
}

function initInviteState(): InviteState {
	return {
		createdInvitation: null,
		emailError: null,
		form: createInviteForm(),
		inviteDialogOpen: false,
	};
}

function isValidEmail(value: string) {
	return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

export default function AdminUserInvitationsPage() {
	const { t } = useTranslation();
	const [searchParams, setSearchParams] = useSearchParams();
	const [inviteState, dispatchInvite] = useReducer(
		inviteReducer,
		undefined,
		initInviteState,
	);
	const { pending: inviting, runWithPending } = usePendingAction();
	const { pendingId: revokingInvitationId, runWithPending: runRevocation } =
		usePendingId<number>();

	usePageTitle(t("admin.users.invitationsTitle"));

	const { query, setQuery } = useManagedListQueryState<{
		pageSize: InvitationPageSize;
	}>({
		schema: {
			pageSize: managedPageSizeField(
				INVITATION_PAGE_SIZE_OPTIONS,
				DEFAULT_INVITATION_PAGE_SIZE,
			),
		},
		searchParams,
		setSearchParams,
	});
	const { pageSize } = query;

	const loadInvitationsPage = useCallback(
		async ({ cursor }: { cursor: InvitationCursor | null }) => {
			const page = await adminUserService.listInvitations({
				limit: pageSize,
				after_created_at: cursor?.value,
				after_id: cursor?.id,
			});
			return {
				items: page.items,
				nextCursor: page.next_cursor ?? null,
				total: page.total,
			};
		},
		[pageSize],
	);
	const list = useCursorList<AdminUserInvitationInfo, InvitationCursor>({
		deps: [pageSize],
		loadPage: loadInvitationsPage,
		onError: handleApiError,
	});
	const {
		currentPage,
		items: invitations,
		loading,
		setItems,
		total,
		totalPages,
	} = list;

	const revokeDialog = useConfirmDialog<AdminUserInvitationInfo>((target) => {
		void runRevocation(target.id, async () => {
			try {
				const updated = await adminUserService.revokeInvitation(target.id);
				setItems((current) =>
					current.map((item) => (item.id === updated.id ? updated : item)),
				);
				toast.success(t("admin.users.invitationRevoked"));
			} catch (error) {
				handleApiError(error);
			}
		});
	});

	const pageSizeOptions = INVITATION_PAGE_SIZE_OPTIONS.map((size) => ({
		label: t("admin.pagination.pageSizeOption", { count: size }),
		value: String(size),
	}));
	const pagination = useMemo(
		() => (
			<AdminOffsetPagination
				total={total}
				currentPage={currentPage}
				totalPages={totalPages}
				pageSize={String(pageSize)}
				pageSizeOptions={pageSizeOptions}
				prevDisabled={list.prevPageDisabled}
				nextDisabled={list.nextPageDisabled}
				onPrevious={list.goPrevious}
				onNext={list.goNext}
				onPageSizeChange={(value) => {
					const next = parsePageSizeOption(value, INVITATION_PAGE_SIZE_OPTIONS);
					if (next == null) return;
					setQuery({ pageSize: next });
				}}
			/>
		),
		[
			currentPage,
			list.goNext,
			list.goPrevious,
			list.nextPageDisabled,
			list.prevPageDisabled,
			pageSize,
			pageSizeOptions,
			setQuery,
			total,
			totalPages,
		],
	);
	const invitationsEmptyIcon = useMemo(
		() => <Icon name="EnvelopeSimple" className="size-5" />,
		[],
	);
	const invitationsEmptyAction = useMemo(
		() => (
			<Button
				type="button"
				onClick={() => dispatchInvite({ type: "openDialog", value: true })}
			>
				<Icon name="EnvelopeSimple" className="mr-2 size-4" />
				{t("admin.users.inviteUser")}
			</Button>
		),
		[t],
	);
	const invitationsHeaderRow = useMemo(
		() => <UserInvitationsTableHeader />,
		[],
	);

	const copyInvitationLink = useCallback(
		async (value: string) => {
			if (!value) return;
			try {
				await navigator.clipboard.writeText(value);
				toast.success(t("common.copied"));
			} catch (error) {
				handleApiError(error);
			}
		},
		[t],
	);

	async function createInvitation(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const email = inviteState.form.email.trim();
		if (!isValidEmail(email)) {
			dispatchInvite({
				type: "emailError",
				value: t("admin.users.inviteEmailInvalid"),
			});
			return;
		}
		await runWithPending(async () => {
			try {
				const invitation = await adminUserService.createInvitation({ email });
				dispatchInvite({ type: "created", invitation });
				toast.success(t("admin.users.invitationCreated"));
				await list.reloadFirstPage();
			} catch (error) {
				handleApiError(error);
			}
		});
	}

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={t("admin.users.invitationsTitle")}
				description={t("admin.users.invitationsDescription")}
				actions={
					<>
						<UsersSectionNav active="invitations" />
						<Button
							type="button"
							size="sm"
							onClick={() =>
								dispatchInvite({ type: "openDialog", value: true })
							}
						>
							<Icon name="EnvelopeSimple" className="mr-2 size-4" />
							{t("admin.users.inviteUser")}
						</Button>
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={loading}
							onClick={() => void list.reload()}
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
			<AdminTableList
				loading={loading}
				items={invitations}
				columns={6}
				rows={6}
				emptyIcon={invitationsEmptyIcon}
				emptyTitle={t("admin.users.noInvitationsTitle")}
				emptyDescription={t("admin.users.noInvitationsDescription")}
				emptyAction={invitationsEmptyAction}
				headerRow={invitationsHeaderRow}
				pagination={pagination}
				renderRow={(invitation) => (
					<UserInvitationsTableRow
						key={invitation.id}
						invitation={invitation}
						revokingInvitationId={revokingInvitationId}
						onRevokeInvitation={revokeDialog.requestConfirm}
					/>
				)}
			/>
			<InviteUserDialog
				open={inviteState.inviteDialogOpen}
				form={inviteState.form}
				error={inviteState.emailError}
				inviting={inviting}
				createdInvitation={inviteState.createdInvitation}
				onFieldChange={(value) => dispatchInvite({ type: "email", value })}
				onCopyLink={(value) => void copyInvitationLink(value)}
				onSubmit={(event) => void createInvitation(event)}
				onOpenChange={(open) => {
					if (open) {
						dispatchInvite({ type: "openDialog", value: true });
					} else {
						dispatchInvite({ type: "closeDialog" });
					}
				}}
			/>
			<ConfirmDialog
				open={revokeDialog.dialogProps.open}
				onOpenChange={revokeDialog.dialogProps.onOpenChange}
				title={t("admin.users.revokeInvitation")}
				description={t("admin.users.confirmRevokeInvitation", {
					email: revokeDialog.confirmTarget?.email ?? "",
				})}
				cancelLabel={t("common.cancel")}
				confirmLabel={t("admin.users.revokeInvitation")}
				loading={revokingInvitationId != null}
				variant="destructive"
				onConfirm={revokeDialog.dialogProps.onConfirm}
			/>
		</AdminPageShell>
	);
}
