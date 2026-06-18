export type PasswordResetRedirectStatus = "success";

const PASSWORD_RESET_STATUSES = new Set<PasswordResetRedirectStatus>([
	"success",
]);

export type PasswordResetRedirectState = {
	status: PasswordResetRedirectStatus;
};

export function getPasswordResetRedirectState(
	search: string,
): PasswordResetRedirectState | null {
	const params = new URLSearchParams(search);
	const status = params.get("password_reset")?.trim();
	if (
		!status ||
		!PASSWORD_RESET_STATUSES.has(status as PasswordResetRedirectStatus)
	) {
		return null;
	}

	return { status: status as PasswordResetRedirectStatus };
}

export function clearPasswordResetRedirectSearch(search: string) {
	const params = new URLSearchParams(search);
	params.delete("password_reset");
	const next = params.toString();
	return next ? `?${next}` : "";
}
