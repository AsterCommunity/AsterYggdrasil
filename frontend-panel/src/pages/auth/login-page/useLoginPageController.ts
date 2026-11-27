import { type FormEvent, useEffect, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { z } from "zod/v4";
import type { LoginFormCardProps } from "@/components/auth/LoginFormCard";
import { useExternalAuthRecovery } from "@/components/auth/useExternalAuthRecovery";
import { useCaptchaChallenge } from "@/hooks/useCaptchaChallenge";
import { usePageTitle } from "@/hooks/usePageTitle";
import { existingPasswordSchema } from "@/lib/validation";
import {
	getPasskeyCredential,
	isWebAuthnSupported,
	WebAuthnCancelledError,
	WebAuthnUnsupportedError,
} from "@/lib/webauthn";
import { accountPaths } from "@/routes/routePaths";
import { authService } from "@/services/authService";
import { externalAuthService } from "@/services/externalAuthService";
import { formatUnknownError } from "@/services/http";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { ExternalAuthPublicProvider } from "@/types/api";

type LoginField = "identifier" | "password";

type LoginFieldErrors = Partial<Record<LoginField, string>>;

type LoginPageState = {
	errors: LoginFieldErrors;
	externalLoadingKey: string | null;
	identifier: string;
	loading: boolean;
	passkeySubmitting: boolean;
	password: string;
	providers: ExternalAuthPublicProvider[];
	showPassword: boolean;
};

type LoginPageAction =
	| { type: "field"; name: "identifier" | "password"; value: string }
	| { type: "togglePassword" }
	| { type: "errors"; value: LoginFieldErrors }
	| { type: "fieldError"; field: LoginField; message: string | null }
	| { type: "providers"; value: ExternalAuthPublicProvider[] }
	| { type: "externalLoadingKey"; value: string | null }
	| { type: "loading"; value: boolean }
	| { type: "passkeySubmitting"; value: boolean };

function omitFormError(
	errors: LoginFieldErrors,
	field: LoginField,
): LoginFieldErrors {
	if (!errors[field]) return errors;
	const nextErrors = { ...errors };
	delete nextErrors[field];
	return nextErrors;
}

function loginPageReducer(
	state: LoginPageState,
	action: LoginPageAction,
): LoginPageState {
	switch (action.type) {
		case "field":
			return {
				...state,
				[action.name]: action.value,
				errors: omitFormError(state.errors, action.name),
			};
		case "togglePassword":
			return { ...state, showPassword: !state.showPassword };
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
		case "providers":
			return { ...state, providers: action.value };
		case "externalLoadingKey":
			return { ...state, externalLoadingKey: action.value };
		case "loading":
			return { ...state, loading: action.value };
		case "passkeySubmitting":
			return { ...state, passkeySubmitting: action.value };
	}
}

const initialLoginPageState: LoginPageState = {
	errors: {},
	externalLoadingKey: null,
	identifier: "",
	loading: false,
	passkeySubmitting: false,
	password: "",
	providers: [],
	showPassword: false,
};

const loginFormSchema = z.object({
	identifier: z.string().trim().min(1, "login.validationIdentifierRequired"),
	password: z.string().min(1, "login.validationPasswordRequired"),
});

const loginIdentifierSchema = loginFormSchema.shape.identifier;

function zodErrorToFormErrors(error: z.ZodError): LoginFieldErrors {
	const nextErrors: LoginFieldErrors = {};
	for (const issue of error.issues) {
		const field = issue.path[0];
		if (field === "identifier" || field === "password") {
			nextErrors[field] = issue.message;
		}
	}
	return nextErrors;
}

function firstZodIssueMessage(result: z.ZodSafeParseResult<unknown>) {
	return result.success ? null : (result.error.issues[0]?.message ?? "");
}

export function useLoginPageController() {
	const { t } = useTranslation();
	const [form, dispatch] = useReducer(loginPageReducer, initialLoginPageState);
	const {
		errors,
		externalLoadingKey,
		identifier,
		loading,
		passkeySubmitting,
		password,
		providers,
		showPassword,
	} = form;
	const passkeySupported = isWebAuthnSupported();
	const login = useAuthStore((state) => state.login);
	const loginWithPasskey = useAuthStore((state) => state.loginWithPasskey);
	const branding = useFrontendConfigStore((state) => state.branding);
	const allowUserRegistration = useFrontendConfigStore(
		(state) => state.allowUserRegistration,
	);
	const passkeyLoginEnabled = useFrontendConfigStore(
		(state) => state.passkeyLoginEnabled,
	);
	const captchaConfig = useFrontendConfigStore((state) => state.captcha);
	const navigate = useNavigate();
	const captchaRequired = captchaConfig.enabled && captchaConfig.login_required;
	const captcha = useCaptchaChallenge(captchaRequired);
	const {
		externalAuthRecovery,
		submitRecovery,
		clearRecovery,
		changeRecoveryEmail,
		changeRecoveryMode,
		changeRecoveryPassword,
		changeRecoveryPasswordIdentifier,
	} = useExternalAuthRecovery({ email: "", identifier });

	useEffect(() => {
		void import("@/lib/pwaWarmup").then(({ warmupLoginSuccessPath }) => {
			warmupLoginSuccessPath();
		});
	}, []);

	useEffect(() => {
		const controller = new AbortController();
		externalAuthService
			.listPublic(controller.signal)
			.then((nextProviders) =>
				dispatch({ type: "providers", value: nextProviders }),
			)
			.catch(() => undefined);
		return () => controller.abort();
	}, []);

	function setFieldError(field: LoginField, message: string | null) {
		dispatch({ type: "fieldError", field, message });
	}

	function validateSingle(
		field: LoginField,
		value: unknown,
		schema: z.ZodType,
	) {
		setFieldError(field, firstZodIssueMessage(schema.safeParse(value)));
	}

	function changeIdentifier(value: string) {
		dispatch({ type: "field", name: "identifier", value });
		if (value.trim() || errors.identifier) {
			validateSingle("identifier", value, loginIdentifierSchema);
		}
	}

	function changePassword(value: string) {
		dispatch({ type: "field", name: "password", value });
		if (errors.password) {
			validateSingle("password", value, existingPasswordSchema);
		}
	}

	async function submit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (externalAuthRecovery) {
			await submitRecovery();
			return;
		}

		const validation = loginFormSchema.safeParse({ identifier, password });
		if (!validation.success) {
			dispatch({
				type: "errors",
				value: zodErrorToFormErrors(validation.error),
			});
			toast.error(t("login.validationFailed"));
			return;
		}

		dispatch({ type: "errors", value: {} });
		dispatch({ type: "loading", value: true });
		try {
			await login(
				validation.data.identifier,
				validation.data.password,
				captchaRequired
					? {
							answer: captcha.answer,
							challengeId: captcha.challengeId ?? "",
						}
					: undefined,
			);
			toast.success(t("login.loginSuccess"));
			navigate(accountPaths.home);
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
			if (captchaRequired) void captcha.refresh();
		} finally {
			dispatch({ type: "loading", value: false });
		}
	}

	async function startExternalLogin(provider: ExternalAuthPublicProvider) {
		dispatch({ type: "externalLoadingKey", value: provider.key });
		try {
			const response = await externalAuthService.startAuthAlias(
				provider.kind,
				provider.key,
				{
					return_path: accountPaths.home,
				},
			);
			window.location.assign(response.authorization_url);
		} catch (nextError) {
			toast.error(formatUnknownError(nextError));
			dispatch({ type: "externalLoadingKey", value: null });
		}
	}

	async function startPasskeyLogin() {
		if (!passkeySupported) {
			toast.error(t("login.passkeyUnsupported"));
			return;
		}
		dispatch({ type: "passkeySubmitting", value: true });
		try {
			const start = await authService.startPasskeyLogin({
				identifier: identifier.trim() || undefined,
			});
			const credential = await getPasskeyCredential(start.public_key);
			await loginWithPasskey(start.flow_id, credential);
			toast.success(t("login.loginSuccess"));
			navigate(accountPaths.home);
		} catch (nextError) {
			if (nextError instanceof WebAuthnUnsupportedError) {
				toast.error(t("login.passkeyUnsupported"));
				return;
			}
			if (nextError instanceof WebAuthnCancelledError) {
				toast.error(t("login.passkeyCancelled"));
				return;
			}
			toast.error(formatUnknownError(nextError));
		} finally {
			dispatch({ type: "passkeySubmitting", value: false });
		}
	}

	const canSubmit = loginFormSchema.safeParse({ identifier, password }).success;
	const captchaReady =
		!captchaRequired ||
		(Boolean(captcha.challengeId) && captcha.answer.trim().length > 0);
	const submitDisabled = loading || !canSubmit || !captchaReady;
	const brandTitle = branding.title || t("brand.name");
	const visibleProviders = providers.slice(0, 3);

	usePageTitle(t("login.title"));

	const formCardProps: LoginFormCardProps = {
		isRegister: false,
		usesAccountCreationForm: false,
		cardTitle: t("login.title"),
		cardDescription: t("login.cardDescription"),
		identifier,
		username: "",
		email: "",
		password,
		confirmPassword: "",
		errors,
		showPassword,
		acceptedTerms: false,
		visibleProviders,
		externalLoadingKey,
		loading,
		passkeySubmitting,
		passkeySupported,
		showPasskeyLogin: passkeyLoginEnabled,
		externalAuthRecovery,
		submitDisabled,
		submitLabel: t("nav.login"),
		passwordScore: 0,
		passwordStrengthLabel: "",
		allowUserRegistration,
		captchaAnswer: captcha.answer,
		captchaImageBase64: captcha.imageBase64,
		captchaLoadError: captcha.error,
		captchaLoading: captcha.loading,
		captchaRequired,
		onSubmit: submit,
		onCaptchaAnswerChange: captcha.setAnswer,
		onCaptchaRefresh: () => void captcha.refresh(),
		onIdentifierChange: changeIdentifier,
		onUsernameChange: () => {},
		onEmailChange: () => {},
		onPasswordChange: changePassword,
		onConfirmPasswordChange: () => {},
		onToggleShowPassword: () => dispatch({ type: "togglePassword" }),
		onAcceptedTermsChange: () => {},
		onPasskeyLogin: () => void startPasskeyLogin(),
		onExternalLogin: (provider) => void startExternalLogin(provider),
		onExternalAuthRecoveryBack: clearRecovery,
		onExternalAuthRecoveryEmailChange: changeRecoveryEmail,
		onExternalAuthRecoveryIdentifierChange: changeRecoveryPasswordIdentifier,
		onExternalAuthRecoveryModeChange: changeRecoveryMode,
		onExternalAuthRecoveryPasswordChange: changeRecoveryPassword,
		onResetAccountOptions: () => {},
	};

	return {
		branding,
		brandTitle,
		description: t("login.welcomeDescription"),
		formCardProps,
		headline: t("login.welcomeTitle"),
		tagline: t("brand.tagline"),
	};
}
