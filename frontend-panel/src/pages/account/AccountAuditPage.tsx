import { useCallback } from "react";
import { AuditLogPage } from "@/components/audit/AuditLogPage";
import { accountService } from "@/services/accountService";
import type { AuditLogSortBy } from "@/types/api";

const ACCOUNT_AUDIT_SORT_BY_OPTIONS = [
	"id",
	"created_at",
	"action",
	"entity_type",
	"entity_name",
	"ip_address",
] as const satisfies readonly AuditLogSortBy[];

export default function AccountAuditPage() {
	const listAuditLogs = useCallback(
		(query: Parameters<typeof accountService.listAuditLogs>[0]) =>
			accountService.listAuditLogs(query),
		[],
	);

	return (
		<AuditLogPage
			list={listAuditLogs}
			showActor={false}
			sortOptions={ACCOUNT_AUDIT_SORT_BY_OPTIONS}
			translationPrefix="account.auditPage"
		/>
	);
}
