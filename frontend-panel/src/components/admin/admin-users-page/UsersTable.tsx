import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminSortableTableHead,
	AdminTableCell as TableCell,
	AdminTableHead as TableHead,
	AdminTableHeader as TableHeader,
	AdminTableRow as TableRow,
} from "@/components/common/AdminTable";
import { DateTimeText } from "@/components/common/DateTimeText";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { getNormalizedDisplayName, getUserDisplayName } from "@/lib/user";
import { cn } from "@/lib/utils";
import type { AdminUserInfo, AdminUserSortBy } from "@/types/api";

type SortOrder = "asc" | "desc";

export function UsersTableHeader({
	onSortChange,
	sortBy,
	sortOrder,
}: {
	onSortChange: (sortBy: AdminUserSortBy, sortOrder: SortOrder) => void;
	sortBy: AdminUserSortBy;
	sortOrder: SortOrder;
}) {
	const { t } = useTranslation();

	return (
		<TableHeader>
			<TableRow>
				<AdminSortableTableHead
					sortKey="id"
					sortBy={sortBy}
					sortOrder={sortOrder}
					onSortChange={onSortChange}
					className="w-20 text-center"
				>
					{t("admin.users.table.id")}
				</AdminSortableTableHead>
				<AdminSortableTableHead
					sortKey="username"
					sortBy={sortBy}
					sortOrder={sortOrder}
					onSortChange={onSortChange}
					className="min-w-[260px]"
				>
					{t("admin.users.table.account")}
				</AdminSortableTableHead>
				<AdminSortableTableHead
					sortKey="role"
					sortBy={sortBy}
					sortOrder={sortOrder}
					onSortChange={onSortChange}
					className="w-32"
				>
					{t("admin.users.table.role")}
				</AdminSortableTableHead>
				<AdminSortableTableHead
					sortKey="status"
					sortBy={sortBy}
					sortOrder={sortOrder}
					onSortChange={onSortChange}
					className="w-32"
				>
					{t("admin.users.table.status")}
				</AdminSortableTableHead>
				<TableHead className="w-44">
					{t("admin.users.table.activity")}
				</TableHead>
				<AdminSortableTableHead
					sortKey="updated_at"
					sortBy={sortBy}
					sortOrder={sortOrder}
					onSortChange={onSortChange}
					className="w-44"
				>
					{t("admin.users.table.updated")}
				</AdminSortableTableHead>
				<TableHead className="w-32 text-right">
					{t("admin.users.table.actions")}
				</TableHead>
			</TableRow>
		</TableHeader>
	);
}

export function UsersTableRow({
	deletingId,
	onDelete,
	onEdit,
	onRevokeSessions,
	revokingId,
	user,
}: {
	deletingId: number | null;
	onDelete: (user: AdminUserInfo) => void;
	onEdit: (user: AdminUserInfo) => void;
	onRevokeSessions: (user: AdminUserInfo) => void;
	revokingId: number | null;
	user: AdminUserInfo;
}) {
	const { t } = useTranslation();
	const revoking = revokingId === user.id;
	const deleting = deletingId === user.id;
	const displayName = getUserDisplayName(user);
	const showUsernameSecondary =
		getNormalizedDisplayName(user.profile.display_name) != null &&
		displayName !== user.username;

	return (
		<TableRow
			className="cursor-pointer"
			tabIndex={0}
			onClick={() => onEdit(user)}
			onKeyDown={(event) => {
				if (event.key === "Enter" || event.key === " ") {
					event.preventDefault();
					onEdit(user);
				}
			}}
		>
			<TableCell className="text-center">
				<span className="font-mono text-muted-foreground text-sm tabular-nums">
					{user.id}
				</span>
			</TableCell>
			<TableCell>
				<div className="flex min-w-0 items-center gap-3">
					<UserAvatarImage
						avatar={user.profile.avatar}
						name={displayName}
						alt=""
						size="sm"
						className="rounded-lg"
					/>
					<div className="min-w-0">
						<div className="truncate font-medium">{displayName}</div>
						<div className={cn("mt-1 truncate", ADMIN_TABLE_MUTED_TEXT_CLASS)}>
							{showUsernameSecondary
								? `@${user.username} · ${user.email}`
								: user.email}
						</div>
					</div>
				</div>
			</TableCell>
			<TableCell>
				<RoleBadge userRole={user.role} />
			</TableCell>
			<TableCell>
				<StatusBadge status={user.status} />
			</TableCell>
			<TableCell>
				<div className="space-y-1 text-sm">
					<div>
						{t("admin.users.profileCount", { count: user.profile_count })}
					</div>
					<div className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
						{t("admin.users.activeSessionCount", {
							count: user.active_session_count,
						})}
					</div>
				</div>
			</TableCell>
			<TableCell>
				<div className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
					<DateTimeText value={user.updated_at} />
				</div>
			</TableCell>
			<TableCell
				onClick={(event) => event.stopPropagation()}
				onKeyDown={(event) => event.stopPropagation()}
			>
				<div className="flex justify-end gap-1">
					<TooltipProvider>
						<ActionTooltip label={t("admin.users.edit")}>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								onClick={() => onEdit(user)}
								aria-label={t("admin.users.edit")}
							>
								<Icon name="PencilSimple" className="size-4" />
							</Button>
						</ActionTooltip>
						<ActionTooltip label={t("admin.users.revokeSessions")}>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								disabled={revoking || user.active_session_count === 0}
								onClick={() => onRevokeSessions(user)}
								aria-label={t("admin.users.revokeSessions")}
							>
								<Icon
									name={revoking ? "Spinner" : "Key"}
									className={cn("size-4", revoking && "animate-spin")}
								/>
							</Button>
						</ActionTooltip>
						<ActionTooltip
							label={
								user.id === 1
									? t("admin.users.initialAdminDeleteBlocked")
									: t("admin.users.delete")
							}
						>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								disabled={user.id === 1 || deleting}
								onClick={() => onDelete(user)}
								aria-label={t("admin.users.delete")}
								className="text-destructive hover:text-destructive"
							>
								<Icon
									name={deleting ? "Spinner" : "Trash"}
									className={cn("size-4", deleting && "animate-spin")}
								/>
							</Button>
						</ActionTooltip>
					</TooltipProvider>
				</div>
			</TableCell>
		</TableRow>
	);
}

function ActionTooltip({
	children,
	label,
}: {
	children: ReactNode;
	label: string;
}) {
	return (
		<Tooltip>
			<TooltipTrigger render={<span className="inline-flex size-8" />}>
				{children}
			</TooltipTrigger>
			<TooltipContent>{label}</TooltipContent>
		</Tooltip>
	);
}

export function RoleBadge({ userRole }: { userRole: AdminUserInfo["role"] }) {
	const { t } = useTranslation();
	return (
		<Badge
			variant="outline"
			className={cn(
				"rounded-md",
				userRole === "admin"
					? "border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-200"
					: userRole === "operator"
						? "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-200"
						: "border-muted-foreground/30 bg-muted/40 text-muted-foreground",
			)}
		>
			{userRole === "admin"
				? t("admin.users.role.admin")
				: userRole === "operator"
					? t("admin.users.role.operator")
					: t("admin.users.role.user")}
		</Badge>
	);
}

export function StatusBadge({ status }: { status: AdminUserInfo["status"] }) {
	const { t } = useTranslation();
	return (
		<Badge
			variant="outline"
			className={cn(
				"rounded-md",
				status === "active"
					? "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-200"
					: "border-destructive/30 bg-destructive/10 text-destructive",
			)}
		>
			{status === "active"
				? t("admin.users.status.active")
				: t("admin.users.status.disabled")}
		</Badge>
	);
}
