import { useAuthStore } from "@/stores/authStore";
import { createAuthUser } from "@/test/fixtures";
import type { AuthUserInfo } from "@/types/api";

/**
 * Seed the auth store with an authenticated session for tests that render
 * guarded components directly, bypassing the `authService.me` hydration path.
 */
export function setAuthenticatedUser(
	user: AuthUserInfo = createAuthUser(),
	options: { isAdmin?: boolean } = {},
) {
	useAuthStore.setState({
		user,
		checking: false,
		error: null,
		expiresAt: Date.now() + 60_000,
		isAuthStale: false,
		isAuthenticated: true,
		isAdmin: options.isAdmin ?? user.role === "admin",
	});
}
