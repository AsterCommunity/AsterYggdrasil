import { useTranslation } from "react-i18next";
import {
	ADMIN_TABLE_MONO_TEXT_CLASS,
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminTableCell as TableCell,
	AdminTableHead as TableHead,
	AdminTableHeader as TableHeader,
	AdminTableRow as TableRow,
} from "@/components/common/AdminTable";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { formatDateTime } from "@/lib/dateTime";
import { cn } from "@/lib/utils";
import type {
	AdminUserInvitationInfo,
	UserInvitationStatus,
} from "@/types/api";

interface UserInvitationsTableRowProps {
	invitation: AdminUserInvitationInfo;
	revokingInvitationId: number | null;
	onRevokeInvitation: (invitation: AdminUserInvitationInfo) => void;
}

function invitationStatusClass(status: UserInvitationStatus) {
	if (status === "pending") {
		return "border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-200";
	}
	if (status === "accepted") {
		return "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-200";
	}
	if (status === "revoked") {
		return "border-muted-foreground/30 bg-muted/40 text-muted-foreground";
	}
	return "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-200";
}

export function UserInvitationsTableHeader() {
	const { t } = useTranslation();

	return (
		<TableHeader>
			<TableRow>
				<TableHead className="w-16">{t("admin.users.table.id")}</TableHead>
				<TableHead className="min-w-[260px]">
					{t("admin.users.table.email")}
				</TableHead>
				<TableHead className="w-32">{t("admin.users.table.status")}</TableHead>
				<TableHead className="w-44">
					{t("admin.users.invitationExpiresAt")}
				</TableHead>
				<TableHead className="w-44">
					{t("admin.users.invitationCreatedAt")}
				</TableHead>
				<TableHead className="w-28 text-right">
					{t("admin.users.table.actions")}
				</TableHead>
			</TableRow>
		</TableHeader>
	);
}

export function UserInvitationsTableRow({
	invitation,
	onRevokeInvitation,
	revokingInvitationId,
}: UserInvitationsTableRowProps) {
	const { t, i18n } = useTranslation();
	const locale = i18n.language?.startsWith("zh") ? "zh-CN" : "en-US";
	const isPending = invitation.status === "pending";
	const isRevoking = revokingInvitationId === invitation.id;

	return (
		<TableRow>
			<TableCell>
				<span className={ADMIN_TABLE_MONO_TEXT_CLASS}>{invitation.id}</span>
			</TableCell>
			<TableCell>
				<div className="min-w-0">
					<div className="space-y-1">
						<div className="truncate font-medium text-foreground">
							{invitation.email}
						</div>
						{invitation.accepted_user_id ? (
							<div className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
								{t("admin.users.invitationAcceptedUser", {
									id: invitation.accepted_user_id,
								})}
							</div>
						) : null}
					</div>
				</div>
			</TableCell>
			<TableCell>
				<div className="flex items-center">
					<Badge
						variant="outline"
						className={cn(
							"rounded-md",
							invitationStatusClass(invitation.status),
						)}
					>
						{t(`admin.users.invitationStatus.${invitation.status}`)}
					</Badge>
				</div>
			</TableCell>
			<TableCell>
				<span
					className="text-sm text-muted-foreground"
					title={new Date(invitation.expires_at).toISOString()}
				>
					{formatDateTime(invitation.expires_at, locale)}
				</span>
			</TableCell>
			<TableCell>
				<span
					className="text-sm text-muted-foreground"
					title={new Date(invitation.created_at).toISOString()}
				>
					{formatDateTime(invitation.created_at, locale)}
				</span>
			</TableCell>
			<TableCell>
				<div className="flex justify-end gap-1">
					<Button
						type="button"
						variant="ghost"
						size="icon"
						onClick={() => onRevokeInvitation(invitation)}
						aria-label={t("admin.users.revokeInvitation")}
						title={t("admin.users.revokeInvitation")}
						disabled={!isPending || isRevoking}
					>
						<Icon
							name={isRevoking ? "Spinner" : "X"}
							className={cn("size-4", isRevoking && "animate-spin")}
						/>
					</Button>
				</div>
			</TableCell>
		</TableRow>
	);
}
