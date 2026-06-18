export type ContactVerificationRedirectStatus =
	| "email-changed"
	| "expired"
	| "invalid"
	| "missing"
	| "register-activated";

const CONTACT_VERIFICATION_STATUSES =
	new Set<ContactVerificationRedirectStatus>([
		"email-changed",
		"expired",
		"invalid",
		"missing",
		"register-activated",
	]);

export type ContactVerificationRedirectState = {
	email: string | null;
	status: ContactVerificationRedirectStatus;
};

export function getContactVerificationRedirectState(
	search: string,
): ContactVerificationRedirectState | null {
	const params = new URLSearchParams(search);
	const status = params.get("contact_verification")?.trim();
	if (
		!status ||
		!CONTACT_VERIFICATION_STATUSES.has(
			status as ContactVerificationRedirectStatus,
		)
	) {
		return null;
	}

	return {
		email: params.get("email")?.trim() || null,
		status: status as ContactVerificationRedirectStatus,
	};
}

export function clearContactVerificationRedirectSearch(search: string) {
	const params = new URLSearchParams(search);
	params.delete("contact_verification");
	params.delete("email");
	const next = params.toString();
	return next ? `?${next}` : "";
}
