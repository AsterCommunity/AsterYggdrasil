import { useTranslation } from "react-i18next";
import {
	ADMIN_TABLE_MONO_TEXT_CLASS,
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminTableCell as TableCell,
	AdminTableHead as TableHead,
	AdminTableHeader as TableHeader,
	AdminTableRow as TableRow,
} from "@/components/common/AdminTable";
import { DateTimeText } from "@/components/common/DateTimeText";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";
import type {
	AdminExternalAuthProviderInfo,
	ExternalAuthProviderKindInfo,
} from "@/types/api";
import {
	callbackUrl,
	ExternalAuthProviderIcon,
	kindDisplayName,
	primaryEndpoint,
} from "./shared";

export function ExternalAuthProvidersTableHeader() {
	const { t } = useTranslation();

	return (
		<TableHeader>
			<TableRow>
				<TableHead className="min-w-[240px]">
					{t("admin.externalAuth.table.provider")}
				</TableHead>
				<TableHead className="min-w-[280px]">
					{t("admin.externalAuth.table.endpoint")}
				</TableHead>
				<TableHead className="w-40">
					{t("admin.externalAuth.table.status")}
				</TableHead>
				<TableHead className="w-32 text-right">
					{t("admin.externalAuth.table.actions")}
				</TableHead>
			</TableRow>
		</TableHeader>
	);
}

export function ExternalAuthProvidersTableRow({
	deletingId,
	onCopyCallbackUrl,
	onEdit,
	onRequestDelete,
	onTestProvider,
	provider,
	providerKinds,
	testingId,
}: {
	deletingId: number | null;
	onCopyCallbackUrl: (value: string) => void;
	onEdit: (provider: AdminExternalAuthProviderInfo) => void;
	onRequestDelete: (provider: AdminExternalAuthProviderInfo) => void;
	onTestProvider: (provider: AdminExternalAuthProviderInfo) => void;
	provider: AdminExternalAuthProviderInfo;
	providerKinds: ExternalAuthProviderKindInfo[];
	testingId: number | null;
}) {
	const { t } = useTranslation();
	const deleting = deletingId === provider.id;
	const testing = testingId === provider.id;
	const endpoint = primaryEndpoint(provider);

	return (
		<TableRow
			className="cursor-pointer"
			tabIndex={0}
			onClick={() => {
				if (!deleting) onEdit(provider);
			}}
			onKeyDown={(event) => {
				if (event.key === "Enter" || event.key === " ") {
					event.preventDefault();
					if (!deleting) onEdit(provider);
				}
			}}
		>
			<TableCell>
				<div className="flex min-w-0 items-center gap-3">
					<div className="grid size-10 shrink-0 place-items-center rounded-lg border border-border/70 bg-background shadow-xs">
						<ExternalAuthProviderIcon
							iconUrl={provider.icon_url}
							kind={provider.provider_kind}
							className="max-h-6 max-w-6"
						/>
					</div>
					<div className="min-w-0">
						<div className="flex min-w-0 items-center gap-2">
							<span className="truncate font-medium">
								{provider.display_name}
							</span>
							<Badge variant="outline" className="rounded-md">
								{kindDisplayName(t, provider.provider_kind, providerKinds)}
							</Badge>
						</div>
						<div className={cn("mt-1 truncate", ADMIN_TABLE_MUTED_TEXT_CLASS)}>
							{provider.key}
						</div>
					</div>
				</div>
			</TableCell>
			<TableCell>
				<div className="min-w-0">
					<div
						className={cn("truncate", endpoint && ADMIN_TABLE_MONO_TEXT_CLASS)}
						title={endpoint || undefined}
					>
						{endpoint || t("admin.externalAuth.table.noEndpoint")}
					</div>
					<div
						className={cn(
							"mt-1 flex min-w-0 flex-wrap items-center gap-1",
							ADMIN_TABLE_MUTED_TEXT_CLASS,
						)}
					>
						<span>{t("admin.externalAuth.table.updatedAtPrefix")}</span>
						<DateTimeText value={provider.updated_at} />
					</div>
				</div>
			</TableCell>
			<TableCell>
				<div className="flex flex-wrap gap-1.5">
					<Badge
						variant="outline"
						className={cn(
							"rounded-md",
							provider.enabled
								? "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-200"
								: "border-muted-foreground/30 bg-muted/50 text-muted-foreground",
						)}
					>
						{provider.enabled
							? t("admin.externalAuth.enabled")
							: t("admin.externalAuth.disabled")}
					</Badge>
					<Badge variant="outline" className="rounded-md">
						{provider.protocol}
					</Badge>
				</div>
			</TableCell>
			<TableCell
				onClick={(event) => event.stopPropagation()}
				onKeyDown={(event) => event.stopPropagation()}
			>
				<div className="flex justify-end gap-1">
					<Button
						type="button"
						variant="ghost"
						size="icon"
						disabled={deleting || testing}
						aria-label={t("admin.externalAuth.copyCallback")}
						title={t("admin.externalAuth.copyCallback")}
						onClick={() => onCopyCallbackUrl(callbackUrl(provider))}
					>
						<Icon name="Copy" className="size-4" />
					</Button>
					<Button
						type="button"
						variant="ghost"
						size="icon"
						disabled={deleting || testing}
						aria-label={t("admin.externalAuth.test")}
						title={t("admin.externalAuth.test")}
						onClick={() => onTestProvider(provider)}
					>
						<Icon
							name={testing ? "Spinner" : "WifiHigh"}
							className={cn("size-4", testing && "animate-spin")}
						/>
					</Button>
					<Button
						type="button"
						variant="ghost"
						size="icon"
						disabled={deleting || testing}
						className="text-destructive hover:text-destructive"
						aria-label={t("admin.externalAuth.delete")}
						title={t("admin.externalAuth.delete")}
						onClick={() => onRequestDelete(provider)}
					>
						<Icon
							name={deleting ? "Spinner" : "Trash"}
							className={cn("size-4", deleting && "animate-spin")}
						/>
					</Button>
				</div>
			</TableCell>
		</TableRow>
	);
}
