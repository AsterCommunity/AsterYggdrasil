import { toast } from "sonner";
import { i18next } from "@/i18n";
import { ApiError, transportErrorKind } from "@/services/http";

/**
 * Single owner of the user-facing error message chain:
 * `ApiError.code` -> `errors.<code underscored>` i18n key -> error message
 * -> transport classification -> fallback. Transport classification itself
 * lives in services/http.ts (transportErrorKind); there is no second copy.
 */

function errorCodeToMessageKey(code: string): string {
	return `errors.${code.replaceAll(".", "_")}`;
}

export function getApiErrorMessage(error: unknown) {
	if (error instanceof ApiError) {
		const key = errorCodeToMessageKey(error.code);
		if (i18next.exists(key)) {
			return i18next.t(key);
		}
		const message = error.message.trim();
		return (
			message ||
			i18next.t("errors.unexpected_error", {
				defaultValue: "Unexpected error",
			})
		);
	}

	const kind = transportErrorKind(error);
	if (kind === "request_timeout") {
		return i18next.t("errors.request_timeout", {
			defaultValue: "Request timed out",
		});
	}
	if (kind === "network_error") {
		return i18next.t("errors.network_error", {
			defaultValue: "Network error",
		});
	}

	if (error instanceof Error) {
		const message = error.message.trim();
		return (
			message ||
			i18next.t("errors.unexpected_error", {
				defaultValue: "Unexpected error",
			})
		);
	}

	return i18next.t("errors.unexpected_error", {
		defaultValue: "Unexpected error",
	});
}

export function handleApiError(error: unknown) {
	toast.error(getApiErrorMessage(error));
}
