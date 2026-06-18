import { type FormEvent, useMemo, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { z } from "zod/v4";
import { InitEntryFooter } from "@/components/auth/InitEntryFooter";
import {
	InitFormCard,
	type InitFormErrors,
	type InitFormField,
	type PublicUrlStatus,
} from "@/components/auth/InitFormCard";
import { InitHero } from "@/components/auth/InitHero";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	confirmPasswordRequiredSchema,
	emailSchema,
	passwordSchema,
	usernameSchema,
} from "@/lib/validation";
import { accountPaths } from "@/routes/routePaths";
import { formatUnknownError } from "@/services/http";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import { useInitStatusStore } from "@/stores/initStatusStore";

function detectedPublicUrl() {
	if (typeof window === "undefined") return "https://example.com";
	return window.location.origin;
}

function isLocalHost(hostname: string) {
	return (
		hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1"
	);
}

function validatePublicSiteUrl(value: string): PublicUrlStatus {
	const trimmed = value.trim();
	if (!trimmed) {
		return { valid: false, messageKey: "init.publicSiteUrlRequired" };
	}
	try {
		const url = new URL(trimmed);
		if (url.protocol !== "http:" && url.protocol !== "https:") {
			return { valid: false, messageKey: "init.publicSiteUrlProtocol" };
		}
		if (url.pathname !== "/" || url.search || url.hash) {
			return { valid: false, messageKey: "init.publicSiteUrlOriginOnly" };
		}
		return {
			valid: true,
			normalized: url.origin,
			insecure: url.protocol === "http:" && !isLocalHost(url.hostname),
		};
	} catch {
		return { valid: false, messageKey: "init.publicSiteUrlInvalid" };
	}
}

function getPasswordScore(password: string) {
	if (!password) return 0;
	let score = 0;
	if (password.length >= 8) score += 1;
	if (password.length >= 12) score += 1;
	if (/[a-z]/.test(password) && /[A-Z]/.test(password)) score += 1;
	if (/\d/.test(password)) score += 1;
	if (/[^A-Za-z0-9]/.test(password)) score += 1;
	return Math.min(score, 4);
}

type InitFormState = {
	username: string;
	email: string;
	password: string;
	confirmPassword: string;
	publicSiteUrl: string;
	errors: InitFormErrors;
	showPassword: boolean;
	loading: boolean;
};

type InitFormAction =
	| {
			type: "field";
			name:
				| "username"
				| "email"
				| "password"
				| "confirmPassword"
				| "publicSiteUrl";
			value: string;
	  }
	| { type: "errors"; value: InitFormErrors }
	| { type: "fieldError"; field: InitFormField; message: string | null }
	| { type: "togglePassword" }
	| { type: "loading"; value: boolean };

const setupFormSchema = z
	.object({
		username: usernameSchema,
		email: emailSchema,
		password: passwordSchema,
		confirmPassword: confirmPasswordRequiredSchema,
	})
	.refine((value) => value.password === value.confirmPassword, {
		path: ["confirmPassword"],
		message: "login.passwordMismatch",
	});

function omitFormError(
	errors: InitFormErrors,
	field: InitFormField,
): InitFormErrors {
	if (!errors[field]) return errors;
	const nextErrors = { ...errors };
	delete nextErrors[field];
	return nextErrors;
}

function zodErrorToFormErrors(error: z.ZodError): InitFormErrors {
	const nextErrors: InitFormErrors = {};
	for (const issue of error.issues) {
		const field = issue.path[0];
		if (
			field === "username" ||
			field === "email" ||
			field === "password" ||
			field === "confirmPassword"
		) {
			nextErrors[field] = issue.message;
		}
	}
	return nextErrors;
}

function firstZodIssueMessage(result: z.ZodSafeParseResult<unknown>) {
	return result.success ? null : (result.error.issues[0]?.message ?? "");
}

function initFormReducer(
	state: InitFormState,
	action: InitFormAction,
): InitFormState {
	switch (action.type) {
		case "field":
			return {
				...state,
				[action.name]: action.value,
				errors:
					action.name === "publicSiteUrl"
						? state.errors
						: omitFormError(state.errors, action.name),
			};
		case "errors":
			return { ...state, errors: action.value };
		case "fieldError":
			return {
				...state,
				errors:
					action.message === null
						? omitFormError(state.errors, action.field)
						: { ...state.errors, [action.field]: action.message },
			};
		case "togglePassword":
			return { ...state, showPassword: !state.showPassword };
		case "loading":
			return { ...state, loading: action.value };
	}
}

export default function InitPage() {
	const { t } = useTranslation();
	const setup = useAuthStore((state) => state.setup);
	const branding = useFrontendConfigStore((state) => state.branding);
	const markInitialized = useInitStatusStore((state) => state.markInitialized);
	const navigate = useNavigate();
	const [form, dispatch] = useReducer(
		initFormReducer,
		undefined,
		(): InitFormState => ({
			username: "",
			email: "",
			password: "",
			confirmPassword: "",
			publicSiteUrl: detectedPublicUrl(),
			errors: {},
			showPassword: false,
			loading: false,
		}),
	);

	const publicUrlStatus = useMemo(
		() => validatePublicSiteUrl(form.publicSiteUrl),
		[form.publicSiteUrl],
	);
	const passwordScore = getPasswordScore(form.password);
	const passwordStrengthKey =
		passwordScore <= 1
			? "login.passwordStrengthWeak"
			: passwordScore <= 3
				? "login.passwordStrengthMedium"
				: "login.passwordStrengthStrong";
	const canSubmit =
		publicUrlStatus.valid &&
		setupFormSchema.safeParse({
			username: form.username,
			email: form.email,
			password: form.password,
			confirmPassword: form.confirmPassword,
		}).success;
	const submitDisabled = form.loading || !canSubmit;
	const brandTitle = branding.title || t("brand.name");

	usePageTitle(t("init.title"));

	async function submit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const validation = setupFormSchema.safeParse({
			username: form.username,
			email: form.email,
			password: form.password,
			confirmPassword: form.confirmPassword,
		});
		if (!validation.success) {
			dispatch({
				type: "errors",
				value: zodErrorToFormErrors(validation.error),
			});
			toast.error(t("login.validationFailed"));
			return;
		}
		if (!publicUrlStatus.valid) {
			toast.error(t(publicUrlStatus.messageKey));
			return;
		}

		dispatch({ type: "errors", value: {} });
		dispatch({ type: "loading", value: true });
		try {
			await setup(
				validation.data.username,
				validation.data.email,
				validation.data.password,
				publicUrlStatus.normalized,
			);
			markInitialized();
			toast.success(t("init.setupComplete"));
			navigate(accountPaths.home, { replace: true });
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	function setFieldError(field: InitFormField, message: string | null) {
		dispatch({ type: "fieldError", field, message });
	}

	function validateSingle(
		field: InitFormField,
		value: unknown,
		schema: z.ZodType,
	) {
		setFieldError(field, firstZodIssueMessage(schema.safeParse(value)));
	}

	function validateConfirmPassword(
		nextConfirmPassword: string,
		nextPassword: string,
	) {
		const requiredResult =
			confirmPasswordRequiredSchema.safeParse(nextConfirmPassword);
		if (!requiredResult.success) {
			setFieldError(
				"confirmPassword",
				requiredResult.error.issues[0]?.message ?? "",
			);
			return;
		}
		setFieldError(
			"confirmPassword",
			nextConfirmPassword === nextPassword ? null : "login.passwordMismatch",
		);
	}

	function changeUsername(value: string) {
		dispatch({ type: "field", name: "username", value });
		validateSingle("username", value, usernameSchema);
	}

	function changeEmail(value: string) {
		dispatch({ type: "field", name: "email", value });
		validateSingle("email", value, emailSchema);
	}

	function changePassword(value: string) {
		dispatch({ type: "field", name: "password", value });
		validateSingle("password", value, passwordSchema);
		if (form.confirmPassword || form.errors.confirmPassword) {
			validateConfirmPassword(form.confirmPassword, value);
		}
	}

	function changeConfirmPassword(value: string) {
		dispatch({ type: "field", name: "confirmPassword", value });
		validateConfirmPassword(value, form.password);
	}

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={t("brand.tagline")}
			variant="auth"
		>
			<main className="app-route-transition mx-auto grid w-full max-w-[92rem] flex-1 items-center gap-8 px-4 py-8 sm:px-8 lg:px-12 xl:grid-cols-[minmax(560px,1fr)_minmax(430px,520px)]">
				<InitHero />
				<InitFormCard
					username={form.username}
					email={form.email}
					password={form.password}
					confirmPassword={form.confirmPassword}
					publicSiteUrl={form.publicSiteUrl}
					errors={form.errors}
					showPassword={form.showPassword}
					loading={form.loading}
					submitDisabled={submitDisabled}
					passwordScore={passwordScore}
					passwordStrengthLabel={t(passwordStrengthKey)}
					publicUrlStatus={publicUrlStatus}
					onSubmit={submit}
					onUsernameChange={changeUsername}
					onEmailChange={changeEmail}
					onPasswordChange={changePassword}
					onConfirmPasswordChange={changeConfirmPassword}
					onPublicSiteUrlChange={(value) =>
						dispatch({ type: "field", name: "publicSiteUrl", value })
					}
					onToggleShowPassword={() => dispatch({ type: "togglePassword" })}
				/>
			</main>

			<InitEntryFooter brandTitle={brandTitle} />
		</PublicEntryShell>
	);
}
