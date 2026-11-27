import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import {
	AUTH_REDIRECT_STATUS,
	appendAuthRedirectStatus,
} from "@/lib/authRedirectToast";
import {
	clearContactVerificationRedirectSearch,
	getContactVerificationRedirectState,
} from "@/lib/contactVerificationRedirect";
import {
	clearPasswordResetRedirectSearch,
	getPasswordResetRedirectState,
} from "@/lib/passwordResetRedirect";
import { emailSchema, existingPasswordSchema } from "@/lib/validation";
import { accountPaths } from "@/routes/routePaths";
import { authService } from "@/services/authService";
import { formatUnknownError } from "@/services/http";

export type ExternalAuthRecoveryMode = "password" | "email";

export type ExternalAuthRecoveryState = {
	email: string;
	emailError: string;
	emailSubmitting: boolean;
	flowToken: string;
	mode: ExternalAuthRecoveryMode;
	password: string;
	passwordError: string;
	passwordIdentifier: string;
	passwordIdentifierError: string;
	passwordSubmitting: boolean;
	returnPath: string;
	sent: boolean;
};

/**
 * Shared auth-entry callback handling: redirect toasts (contact
 * verification, password reset), external-auth callback errors, and the
 * external-auth recovery state machine used by both the login and the
 * register pages.
 */
export function useExternalAuthRecovery({
	email,
	identifier,
}: {
	email: string;
	identifier: string;
}) {
	const { t } = useTranslation();
	const location = useLocation();
	const locationSearch = location.search;
	const locationPathname = location.pathname;
	const navigate = useNavigate();
	const [recovery, setRecovery] = useState<ExternalAuthRecoveryState | null>(
		null,
	);

	useEffect(() => {
		const contactVerification =
			getContactVerificationRedirectState(locationSearch);
		const passwordReset = getPasswordResetRedirectState(locationSearch);
		const searchParams = new URLSearchParams(locationSearch);
		const externalAuthStatus = searchParams.get("external_auth");
		const externalAuthFlow = searchParams.get("flow");
		const externalAuthMessage = searchParams.get("message");
		const externalAuthReturnPath =
			searchParams.get("return_path") || accountPaths.home;
		if (!contactVerification && !passwordReset && !externalAuthStatus) {
			return;
		}

		if (contactVerification?.status === "register-activated") {
			toast.success(t("login.activationSuccess"));
		} else if (contactVerification?.status === "invalid") {
			toast.error(t("login.contactVerificationInvalid"));
		} else if (contactVerification?.status === "expired") {
			toast.error(t("login.contactVerificationExpired"));
		} else if (contactVerification?.status === "missing") {
			toast.error(t("login.contactVerificationMissing"));
		}

		if (passwordReset?.status === "success") {
			toast.success(t("login.passwordResetSuccess"));
		}

		if (externalAuthStatus === "email_required" && externalAuthFlow) {
			setRecovery({
				email,
				emailError: "",
				emailSubmitting: false,
				flowToken: externalAuthFlow,
				mode: "password",
				password: "",
				passwordError: "",
				passwordIdentifier: identifier.trim(),
				passwordIdentifierError: "",
				passwordSubmitting: false,
				returnPath: externalAuthReturnPath,
				sent: false,
			});
		} else if (externalAuthStatus === "email_verification_missing") {
			toast.error(t("login.externalAuthEmailMissing"));
		} else if (externalAuthStatus === "email_verification_invalid") {
			toast.error(t("login.externalAuthEmailInvalid"));
		} else if (externalAuthStatus === "email_verification_expired") {
			toast.error(t("login.externalAuthEmailExpired"));
		} else if (externalAuthStatus === "error") {
			toast.error(externalAuthMessage || t("login.externalAuthFailed"));
		}

		const nextContactSearch =
			clearContactVerificationRedirectSearch(locationSearch);
		const cleanedParams = new URLSearchParams(
			clearPasswordResetRedirectSearch(nextContactSearch),
		);
		cleanedParams.delete("external_auth");
		cleanedParams.delete("code");
		cleanedParams.delete("message");
		cleanedParams.delete("flow");
		cleanedParams.delete("return_path");
		const cleanedSearch = cleanedParams.toString();
		navigate(
			{
				pathname: locationPathname,
				search: cleanedSearch ? `?${cleanedSearch}` : "",
			},
			{ replace: true },
		);
	}, [email, identifier, locationPathname, locationSearch, navigate, t]);

	function changeRecoveryEmail(value: string) {
		const validation = emailSchema.safeParse(value);
		setRecovery((current) =>
			current
				? {
						...current,
						email: value,
						emailError: validation.success
							? ""
							: t(validation.error.issues[0]?.message ?? ""),
					}
				: current,
		);
	}

	function changeRecoveryEmailSubmitting(value: boolean) {
		setRecovery((current) =>
			current ? { ...current, emailSubmitting: value } : current,
		);
	}

	function changeRecoveryMode(value: ExternalAuthRecoveryMode) {
		setRecovery((current) => (current ? { ...current, mode: value } : current));
	}

	function changeRecoveryPassword(value: string) {
		const validation = existingPasswordSchema.safeParse(value);
		setRecovery((current) =>
			current
				? {
						...current,
						password: value,
						passwordError: validation.success
							? ""
							: t(validation.error.issues[0]?.message ?? ""),
					}
				: current,
		);
	}

	function changeRecoveryPasswordIdentifier(value: string) {
		setRecovery((current) =>
			current
				? {
						...current,
						passwordIdentifier: value,
						passwordIdentifierError: value.trim()
							? ""
							: current.passwordIdentifierError,
					}
				: current,
		);
	}

	function clearRecovery() {
		setRecovery(null);
	}

	async function requestRecoveryEmailVerification() {
		if (!recovery) return;
		const nextEmail = recovery.email.trim();
		const validation = emailSchema.safeParse(nextEmail);
		if (!validation.success) {
			setRecovery((current) =>
				current
					? {
							...current,
							email: nextEmail,
							emailError: t(validation.error.issues[0]?.message ?? ""),
						}
					: current,
			);
			return;
		}

		changeRecoveryEmailSubmitting(true);
		try {
			await authService.startExternalAuthEmailVerification({
				email: nextEmail,
				flow_token: recovery.flowToken,
			});
			setRecovery((current) =>
				current
					? {
							...current,
							emailError: "",
							emailSubmitting: false,
							sent: true,
						}
					: current,
			);
			toast.success(t("login.externalAuthEmailSentToast"));
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			changeRecoveryEmailSubmitting(false);
		}
	}

	async function linkRecoveryWithPassword() {
		if (!recovery) return;
		const nextIdentifier = recovery.passwordIdentifier.trim();
		const nextPassword = recovery.password;
		const passwordValidation = existingPasswordSchema.safeParse(nextPassword);
		const identifierError = nextIdentifier
			? ""
			: t("login.validationIdentifierRequired");
		const passwordError = passwordValidation.success
			? ""
			: t(passwordValidation.error.issues[0]?.message ?? "");

		setRecovery((current) =>
			current
				? {
						...current,
						passwordError,
						passwordIdentifierError: identifierError,
					}
				: current,
		);
		if (identifierError || passwordError) return;

		setRecovery((current) =>
			current ? { ...current, passwordSubmitting: true } : current,
		);
		try {
			const result = await authService.linkExternalAuthWithPassword({
				flow_token: recovery.flowToken,
				identifier: nextIdentifier,
				password: nextPassword,
			});
			const nextPath =
				result.status === "password_change_required"
					? accountPaths.forcePasswordChange
					: recovery.returnPath || accountPaths.home;
			navigate(
				appendAuthRedirectStatus(
					nextPath,
					AUTH_REDIRECT_STATUS.externalAuthLinked,
				),
			);
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			setRecovery((current) =>
				current ? { ...current, passwordSubmitting: false } : current,
			);
		}
	}

	async function submitRecovery() {
		if (!recovery) return;
		if (recovery.mode === "email") {
			await requestRecoveryEmailVerification();
		} else {
			await linkRecoveryWithPassword();
		}
	}

	return {
		externalAuthRecovery: recovery,
		submitRecovery,
		clearRecovery,
		changeRecoveryEmail,
		changeRecoveryMode,
		changeRecoveryPassword,
		changeRecoveryPasswordIdentifier,
	};
}
