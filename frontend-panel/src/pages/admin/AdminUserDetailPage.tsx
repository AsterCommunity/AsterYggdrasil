import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { UserDetailPanel } from "@/components/admin/admin-users-page/UserDetailPanel";
import { EmptyState } from "@/components/common/EmptyState";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminPageShell } from "@/components/layout/AdminPageShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { handleApiError } from "@/hooks/useApiError";
import { usePageTitle } from "@/hooks/usePageTitle";
import { adminPaths } from "@/routes/routePaths";
import { adminUserService } from "@/services/adminService";
import { useAuthStore } from "@/stores/authStore";
import type { AdminUserInfo, UpdateAdminUserRequest } from "@/types/api";

function parseUserId(value: string | undefined) {
	if (!value) return null;
	const id = Number(value);
	if (!Number.isSafeInteger(id) || id <= 0) return null;
	return id;
}

export default function AdminUserDetailPage() {
	const { t } = useTranslation();
	const navigate = useNavigate();
	const params = useParams();
	const userId = parseUserId(params.id);
	const syncCurrentUserFromAdminUser = useAuthStore(
		(state) => state.syncCurrentUserFromAdminUser,
	);
	const [user, setUser] = useState<AdminUserInfo | null>(null);
	const [loading, setLoading] = useState(true);

	usePageTitle(user?.username ?? t("admin.users.detailTitle"));

	const backToUsers = useCallback(() => {
		void navigate(adminPaths.users);
	}, [navigate]);

	const loadUser = useCallback(async () => {
		if (userId == null) {
			setUser(null);
			setLoading(false);
			return;
		}
		try {
			setLoading(true);
			setUser(await adminUserService.get(userId));
		} catch (error) {
			handleApiError(error);
			setUser(null);
		} finally {
			setLoading(false);
		}
	}, [userId]);

	useEffect(() => {
		void loadUser();
	}, [loadUser]);

	async function updateUser(id: number, data: UpdateAdminUserRequest) {
		const updated = await adminUserService.update(id, data);
		setUser(updated);
		syncCurrentUserFromAdminUser(updated);
	}

	async function revokeUserSessions(id: number) {
		const result = await adminUserService.revokeSessions(id);
		await loadUser();
		return result.removed;
	}

	return (
		<AdminPageShell>
			<AdminPageHeader
				title={user?.username ?? t("admin.users.detailTitle")}
				description={t("admin.users.detailPageDescription")}
				actions={
					<>
						<Button
							type="button"
							variant="outline"
							size="sm"
							onClick={backToUsers}
						>
							<Icon name="ArrowLeft" className="mr-2 size-4" />
							{t("admin.users.backToUsers")}
						</Button>
						<Button
							type="button"
							variant="outline"
							size="sm"
							disabled={loading || userId == null}
							onClick={() => void loadUser()}
						>
							<Icon
								name={loading ? "Spinner" : "ArrowsClockwise"}
								className={loading ? "mr-2 size-4 animate-spin" : "mr-2 size-4"}
							/>
							{t("common.refresh")}
						</Button>
					</>
				}
			/>

			{loading ? (
				<div className="rounded-lg border border-border/70 bg-card p-8 text-sm text-muted-foreground dark:border-white/10 dark:bg-card/90">
					{t("common.loading")}
				</div>
			) : user ? (
				<UserDetailPanel
					user={user}
					onBack={backToUsers}
					onRevokeSessions={revokeUserSessions}
					onUpdate={updateUser}
				/>
			) : (
				<div className="rounded-lg border border-border/70 bg-card dark:border-white/10 dark:bg-card/90">
					<EmptyState
						icon={<Icon name="User" className="size-5" />}
						title={t("admin.users.detailNotFoundTitle")}
						description={t("admin.users.detailNotFoundDescription")}
						action={
							<Button type="button" variant="outline" onClick={backToUsers}>
								<Icon name="ArrowLeft" className="mr-2 size-4" />
								{t("admin.users.backToUsers")}
							</Button>
						}
					/>
				</div>
			)}
		</AdminPageShell>
	);
}
