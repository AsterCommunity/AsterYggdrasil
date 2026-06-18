import { useCallback } from "react";
import { AuditLogPage } from "@/components/audit/AuditLogPage";
import { adminAuditService } from "@/services/adminService";
import type { AuditLogSortBy } from "@/types/api";

const ADMIN_AUDIT_SORT_BY_OPTIONS = [
	"id",
	"created_at",
	"user_id",
	"action",
	"entity_type",
	"entity_name",
	"ip_address",
] as const satisfies readonly AuditLogSortBy[];

export default function AdminAuditPage() {
	const listAuditLogs = useCallback(
		(query: Parameters<typeof adminAuditService.list>[0]) =>
			adminAuditService.list(query),
		[],
	);

	return (
		<AuditLogPage
			list={listAuditLogs}
			showActor
			sortOptions={ADMIN_AUDIT_SORT_BY_OPTIONS}
			translationPrefix="admin.auditPage"
		/>
	);
}
