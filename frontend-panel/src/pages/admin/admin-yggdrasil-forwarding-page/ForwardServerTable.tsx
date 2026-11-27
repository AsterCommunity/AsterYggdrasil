import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
	ADMIN_TABLE_MONO_TEXT_CLASS,
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminTableCell,
	AdminTableHead,
	AdminTableHeader,
	AdminTableRow,
} from "@/components/common/AdminTable";
import { DateTimeText } from "@/components/common/DateTimeText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Switch } from "@/components/ui/switch";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import type {
	AdminYggdrasilSessionForwardServerInfo,
	UpdateYggdrasilSessionForwardServerRequest,
} from "@/types/api";

export function ForwardServerTableHeader() {
	const { t } = useTranslation();
	return (
		<AdminTableHeader>
			<AdminTableRow>
				<AdminTableHead>{t("admin.yggdrasilForwarding.server")}</AdminTableHead>
				<AdminTableHead>{t("admin.yggdrasilForwarding.order")}</AdminTableHead>
				<AdminTableHead>{t("admin.yggdrasilForwarding.state")}</AdminTableHead>
				<AdminTableHead>
					{t("admin.yggdrasilForwarding.lastCheck")}
				</AdminTableHead>
				<AdminTableHead className="w-[1%] text-right">
					{t("admin.yggdrasilForwarding.actions")}
				</AdminTableHead>
			</AdminTableRow>
		</AdminTableHeader>
	);
}

function providerLabelKey(server: AdminYggdrasilSessionForwardServerInfo) {
	return server.provider_kind === "local"
		? "admin.yggdrasilForwarding.provider.local"
		: "admin.yggdrasilForwarding.provider.remote";
}

function isMojangTestingServer(server: AdminYggdrasilSessionForwardServerInfo) {
	const endpointKind = (server as { endpoint_kind?: string }).endpoint_kind;
	return endpointKind === "mojang_session" || server.display_name === "Mojang";
}

export function ForwardServerRow({
	onDelete,
	onEdit,
	onPatch,
	server,
	updating,
}: {
	onDelete: (server: AdminYggdrasilSessionForwardServerInfo) => void;
	onEdit: (server: AdminYggdrasilSessionForwardServerInfo) => void;
	onPatch: (patch: UpdateYggdrasilSessionForwardServerRequest) => void;
	server: AdminYggdrasilSessionForwardServerInfo;
	updating: boolean;
}) {
	const { t } = useTranslation();
	const healthKey = server.last_failure_at
		? "admin.yggdrasilForwarding.health.failed"
		: server.last_success_at
			? "admin.yggdrasilForwarding.health.ok"
			: "admin.yggdrasilForwarding.health.pending";

	return (
		<AdminTableRow>
			<AdminTableCell>
				<div className="min-w-0">
					<div className="flex min-w-0 flex-wrap items-center gap-2">
						<span className="truncate font-medium text-foreground">
							{server.display_name}
						</span>
						<Badge
							variant="outline"
							className={cn(
								"rounded-md",
								server.enabled
									? "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-200"
									: "border-muted-foreground/30 bg-muted/50 text-muted-foreground",
							)}
						>
							{server.enabled
								? t("admin.common.enabled")
								: t("admin.common.disabled")}
						</Badge>
						<Badge variant={server.local ? "secondary" : "outline"}>
							{t(providerLabelKey(server))}
						</Badge>
						{isMojangTestingServer(server) ? (
							<Badge variant="secondary">
								{t("admin.yggdrasilForwarding.testing")}
							</Badge>
						) : null}
					</div>
					<div
						className={cn(
							"mt-1 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1",
							ADMIN_TABLE_MUTED_TEXT_CLASS,
						)}
					>
						<span className={ADMIN_TABLE_MONO_TEXT_CLASS}>#{server.id}</span>
						<span
							className={cn(
								"max-w-[28rem] truncate",
								ADMIN_TABLE_MONO_TEXT_CLASS,
							)}
							title={server.base_url ?? undefined}
						>
							{server.base_url ?? t("admin.yggdrasilForwarding.localBaseUrl")}
						</span>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="flex min-w-[9rem] flex-wrap gap-1.5">
					<Badge
						variant="outline"
						className="rounded-md border-sky-500/30 bg-sky-500/10 font-mono text-sky-700 dark:text-sky-200"
					>
						{t("admin.yggdrasilForwarding.priority")}: {server.priority}
					</Badge>
					<Badge
						variant="outline"
						className="rounded-md border-violet-500/30 bg-violet-500/10 font-mono text-violet-700 dark:text-violet-200"
					>
						{t("admin.yggdrasilForwarding.weight")}: {server.weight}
					</Badge>
					<Badge
						variant="outline"
						className="rounded-md border-amber-500/30 bg-amber-500/10 font-mono text-amber-700 dark:text-amber-200"
					>
						{t("admin.yggdrasilForwarding.timeout")}: {server.timeout_ms} ms
					</Badge>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="flex flex-col gap-2">
					<div className="flex items-center gap-2 text-xs">
						<Switch
							size="sm"
							checked={server.enabled}
							disabled={updating}
							onCheckedChange={(enabled) => onPatch({ enabled })}
							aria-label={t("admin.yggdrasilForwarding.enabled")}
						/>
						<span>
							{server.enabled
								? t("admin.common.enabled")
								: t("admin.common.disabled")}
						</span>
					</div>
					<div className="flex items-center gap-2 text-xs">
						<Switch
							size="sm"
							checked={server.texture_forward_enabled}
							disabled={updating || server.local}
							onCheckedChange={(texture_forward_enabled) =>
								onPatch({ texture_forward_enabled })
							}
							aria-label={t("admin.yggdrasilForwarding.textureForward")}
						/>
						<span>{t("admin.yggdrasilForwarding.textureForward")}</span>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="min-w-[10rem]">
					<div className="text-sm font-medium">{t(healthKey)}</div>
					<DateTimeText
						value={server.last_checked_at}
						className={ADMIN_TABLE_MUTED_TEXT_CLASS}
						fallback={t("admin.yggdrasilForwarding.neverChecked")}
					/>
					{server.last_failure_message ? (
						<div
							className="mt-1 max-w-[16rem] truncate text-xs text-destructive"
							title={server.last_failure_message}
						>
							{server.last_failure_message}
						</div>
					) : null}
				</div>
			</AdminTableCell>
			<AdminTableCell className="text-right">
				<div className="flex justify-end gap-1">
					<ActionTooltipButton
						label={t("admin.yggdrasilForwarding.editAction")}
						onClick={() => onEdit(server)}
					>
						<Icon name="PencilSimple" className="size-4" />
					</ActionTooltipButton>
					<ActionTooltipButton
						label={t("admin.yggdrasilForwarding.deleteAction")}
						disabled={!server.deletable || updating}
						className="text-destructive hover:text-destructive"
						onClick={() => onDelete(server)}
					>
						<Icon
							name={updating ? "Spinner" : "Trash"}
							className={cn("size-4", updating && "animate-spin")}
						/>
					</ActionTooltipButton>
				</div>
			</AdminTableCell>
		</AdminTableRow>
	);
}

export function ActionTooltipButton({
	children,
	className,
	disabled,
	label,
	onClick,
}: {
	children: ReactNode;
	className?: string;
	disabled?: boolean;
	label: string;
	onClick: () => void;
}) {
	return (
		<Tooltip>
			<TooltipTrigger render={<span className="inline-flex size-8" />}>
				<Button
					type="button"
					variant="ghost"
					size="icon"
					aria-label={label}
					disabled={disabled}
					className={cn("text-muted-foreground", className)}
					onClick={onClick}
				>
					{children}
				</Button>
			</TooltipTrigger>
			<TooltipContent>{label}</TooltipContent>
		</Tooltip>
	);
}
