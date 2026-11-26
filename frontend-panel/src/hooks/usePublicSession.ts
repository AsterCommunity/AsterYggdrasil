import { useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useAuthStore } from "@/stores/authStore";

export function usePublicSession() {
	const checkPublicSession = useAuthStore((state) => state.checkPublicSession);
	const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
	const logout = useAuthStore((state) => state.logout);
	const user = useAuthStore((state) => state.user);
	const { t } = useTranslation();

	const handleLogout = useCallback(async () => {
		try {
			await logout();
		} catch {
			// Store 已清理本地会话；服务端撤销未确认时如实提示。
			toast.warning(t("shell.logoutIncomplete"));
		}
	}, [logout, t]);

	useEffect(() => {
		void checkPublicSession();
	}, [checkPublicSession]);

	return {
		isAuthenticated: isAuthenticated && user !== null,
		logout: handleLogout,
		user,
	};
}
