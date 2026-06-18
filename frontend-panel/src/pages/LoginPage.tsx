import { type FormEvent, useEffect, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { z } from "zod/v4";
import { authEntryMainClassName } from "@/components/auth/AuthFormPrimitives";
import { LoginEntryFooter } from "@/components/auth/LoginEntryFooter";
import {
	LoginFormCard,
	type LoginFormCardProps,
} from "@/components/auth/LoginFormCard";
import { LoginHero } from "@/components/auth/LoginHero";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { useCaptchaChallenge } from "@/hooks/useCaptchaChallenge";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	clearContactVerificationRedirectSearch,
	getContactVerificationRedirectState,
} from "@/lib/contactVerificationRedirect";
import {
	clearPasswordResetRedirectSearch,
	getPasswordResetRedirectState,
} from "@/lib/passwordResetRedirect";
import {
	confirmPasswordRequiredSchema,
	emailSchema,
	existingPasswordSchema,
	passwordSchema,
	usernameSchema,
} from "@/lib/validation";
import {
	getPasskeyCredential,
	isWebAuthnSupported,
	WebAuthnCancelledError,
	WebAuthnUnsupportedError,
} from "@/lib/webauthn";
import { accountPaths, publicPaths } from "@/routes/routePaths";
import { authService } from "@/services/authService";
import { externalAuthService } from "@/services/externalAuthService";
import { formatUnknownError } from "@/services/http";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { ExternalAuthPublicProvider } from "@/types/api";

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

type LoginFormState = {
	identifier: string;
	username: string;
	email: string;
	password: string;
	confirmPassword: string;
	errors: LoginFormErrors;
	showPassword: boolean;
	acceptedTerms: boolean;
	providers: ExternalAuthPublicProvider[];
	externalLoadingKey: string | null;
	loading: boolean;
	passkeySubmitting: boolean;
};

type LoginFormField =
	| "identifier"
	| "username"
	| "email"
	| "password"
	| "confirmPassword"
	| "acceptedTerms";

type LoginFormErrors = Partial<Record<LoginFormField, string>>;

type LoginFormAction =
	| {
			type: "field";
			name:
				| "identifier"
				| "username"
				| "email"
				| "password"
				| "confirmPassword";
			value: string;
	  }
	| { type: "togglePassword" }
	| { type: "acceptedTerms"; value: boolean }
	| { type: "errors"; value: LoginFormErrors }
	| { type: "fieldError"; field: LoginFormField; message: string | null }
	| { type: "providers"; value: ExternalAuthPublicProvider[] }
	| { type: "externalLoadingKey"; value: string | null }
	| { type: "loading"; value: boolean }
	| { type: "passkeySubmitting"; value: boolean }
	| { type: "resetAccountOptions" };

function loginFormReducer(
	state: LoginFormState,
	action: LoginFormAction,
): LoginFormState {
	switch (action.type) {
		case "field":
			return {
				...state,
				[action.name]: action.value,
				errors: omitFormError(state.errors, action.name),
			};
		case "togglePassword":
			return { ...state, showPassword: !state.showPassword };
		case "acceptedTerms":
			return {
				...state,
				acceptedTerms: action.value,
				errors: action.value
					? omitFormError(state.errors, "acceptedTerms")
					: state.errors,
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
		case "providers":
			return { ...state, providers: action.value };
		case "externalLoadingKey":
			return { ...state, externalLoadingKey: action.value };
		case "loading":
			return { ...state, loading: action.value };
		case "passkeySubmitting":
			return { ...state, passkeySubmitting: action.value };
		case "resetAccountOptions":
			return {
				...state,
				confirmPassword: "",
				acceptedTerms: false,
				errors: {},
			};
	}
}

const initialLoginFormState: LoginFormState = {
	identifier: "",
	username: "",
	email: "",
	password: "",
	confirmPassword: "",
	errors: {},
	showPassword: false,
	acceptedTerms: false,
	providers: [],
	externalLoadingKey: null,
	loading: false,
	passkeySubmitting: false,
};

const loginFormSchema = z.object({
	identifier: z.string().trim().min(1, "login.validationIdentifierRequired"),
	password: z.string().min(1, "login.validationPasswordRequired"),
});

const loginIdentifierSchema = loginFormSchema.shape.identifier;

const registerFormSchema = z
	.object({
		username: usernameSchema,
		email: emailSchema,
		password: passwordSchema,
		confirmPassword: confirmPasswordRequiredSchema,
		acceptedTerms: z.literal(true, {
			error: "login.validationAcceptTerms",
		}),
	})
	.refine((value) => value.password === value.confirmPassword, {
		path: ["confirmPassword"],
		message: "login.passwordMismatch",
	});

function omitFormError(
	errors: LoginFormErrors,
	field: LoginFormField,
): LoginFormErrors {
	if (!errors[field]) return errors;
	const nextErrors = { ...errors };
	delete nextErrors[field];
	return nextErrors;
}

function zodErrorToFormErrors(error: z.ZodError): LoginFormErrors {
	const nextErrors: LoginFormErrors = {};
	for (const issue of error.issues) {
		const field = issue.path[0];
		if (
			field === "identifier" ||
			field === "username" ||
			field === "email" ||
			field === "password" ||
			field === "confirmPassword" ||
			field === "acceptedTerms"
		) {
			nextErrors[field] = issue.message;
		}
	}
	return nextErrors;
}

function firstZodIssueMessage(result: z.ZodSafeParseResult<unknown>) {
	return result.success ? null : (result.error.issues[0]?.message ?? "");
}

function useLoginPageController() {
	const { t } = useTranslation();
	const location = useLocation();
	const locationSearch = location.search;
	const locationPathname = location.pathname;
	const [form, dispatch] = useReducer(loginFormReducer, initialLoginFormState);
	const {
		identifier,
		username,
		email,
		password,
		confirmPassword,
		errors,
		showPassword,
		acceptedTerms,
		providers,
		externalLoadingKey,
		loading,
		passkeySubmitting,
	} = form;
	const passkeySupported = isWebAuthnSupported();
	const login = useAuthStore((state) => state.login);
	const loginWithPasskey = useAuthStore((state) => state.loginWithPasskey);
	const register = useAuthStore((state) => state.register);
	const branding = useFrontendConfigStore((state) => state.branding);
	const allowUserRegistration = useFrontendConfigStore(
		(state) => state.allowUserRegistration,
	);
	const passkeyLoginEnabled = useFrontendConfigStore(
		(state) => state.passkeyLoginEnabled,
	);
	const captchaConfig = useFrontendConfigStore((state) => state.captcha);
	const navigate = useNavigate();
	const isRegister = locationPathname === publicPaths.register;
	const captchaRequired =
		captchaConfig.enabled &&
		(isRegister
			? captchaConfig.register_required
			: captchaConfig.login_required);
	const captcha = useCaptchaChallenge(captchaRequired);

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

	useEffect(() => {
		const contactVerification =
			getContactVerificationRedirectState(locationSearch);
		const passwordReset = getPasswordResetRedirectState(locationSearch);
		if (!contactVerification && !passwordReset) return;

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

		const nextContactSearch =
			clearContactVerificationRedirectSearch(locationSearch);
		const nextSearch = clearPasswordResetRedirectSearch(nextContactSearch);
		navigate(
			{
				pathname: locationPathname,
				search: nextSearch,
			},
			{ replace: true },
		);
	}, [locationPathname, locationSearch, navigate, t]);

	function setFieldError(field: LoginFormField, message: string | null) {
		dispatch({ type: "fieldError", field, message });
	}

	function validateSingle(
		field: LoginFormField,
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

	function changeIdentifier(value: string) {
		dispatch({ type: "field", name: "identifier", value });
		if (value.trim() || errors.identifier) {
			validateSingle("identifier", value, loginIdentifierSchema);
		}
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
		if (isRegister) {
			validateSingle("password", value, passwordSchema);
			if (confirmPassword || errors.confirmPassword) {
				validateConfirmPassword(confirmPassword, value);
			}
			return;
		}
		if (errors.password) {
			validateSingle("password", value, existingPasswordSchema);
		}
	}

	function changeConfirmPassword(value: string) {
		dispatch({ type: "field", name: "confirmPassword", value });
		validateConfirmPassword(value, password);
	}

	function changeAcceptedTerms(value: boolean) {
		dispatch({ type: "acceptedTerms", value });
		setFieldError(
			"acceptedTerms",
			value ? null : "login.validationAcceptTerms",
		);
	}

	async function submit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (isRegister) {
			const validation = registerFormSchema.safeParse({
				username,
				email,
				password,
				confirmPassword,
				acceptedTerms,
			});
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
				const response = await register(
					validation.data.username,
					validation.data.email,
					validation.data.password,
					captchaRequired
						? {
								answer: captcha.answer,
								challengeId: captcha.challengeId ?? "",
							}
						: undefined,
				);
				if (response.requires_activation) {
					toast.success(t("login.registerActivationSent"));
					navigate(publicPaths.login);
					return;
				}
				toast.success(t("login.registerSuccess"));
				navigate(accountPaths.home);
			} catch (nextError) {
				toast.error(formatUnknownError(nextError));
				if (captchaRequired) void captcha.refresh();
			} finally {
				dispatch({ type: "loading", value: false });
			}
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

	const usesAccountCreationForm = isRegister;
	const passwordScore = getPasswordScore(password);
	const passwordStrengthKey =
		passwordScore <= 1
			? "login.passwordStrengthWeak"
			: passwordScore <= 3
				? "login.passwordStrengthMedium"
				: "login.passwordStrengthStrong";
	const headline = isRegister
		? t("login.registerHeadline")
		: t("login.welcomeTitle");
	const description = isRegister
		? t("login.registerHeroDescription")
		: t("login.welcomeDescription");
	const cardTitle = isRegister ? t("login.registerTitle") : t("login.title");
	const cardDescription = isRegister
		? t("login.registerDescription")
		: t("login.cardDescription");
	const submitLabel = isRegister ? t("login.registerNow") : t("nav.login");
	const canSubmit = isRegister
		? registerFormSchema.safeParse({
				username,
				email,
				password,
				confirmPassword,
				acceptedTerms,
			}).success
		: loginFormSchema.safeParse({ identifier, password }).success;
	const captchaReady =
		!captchaRequired ||
		(Boolean(captcha.challengeId) && captcha.answer.trim().length > 0);
	const submitDisabled = loading || !canSubmit || !captchaReady;
	const brandTitle = branding.title || t("brand.name");
	const visibleProviders = providers.slice(0, 3);
	const showPasskeyLogin = !usesAccountCreationForm && passkeyLoginEnabled;

	usePageTitle(cardTitle);

	const formCardProps: LoginFormCardProps = {
		isRegister,
		usesAccountCreationForm,
		cardTitle,
		cardDescription,
		identifier,
		username,
		email,
		password,
		confirmPassword,
		errors,
		showPassword,
		acceptedTerms,
		visibleProviders,
		externalLoadingKey,
		loading,
		passkeySubmitting,
		passkeySupported,
		showPasskeyLogin,
		submitDisabled,
		submitLabel,
		passwordScore,
		passwordStrengthLabel: t(passwordStrengthKey),
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
		onUsernameChange: changeUsername,
		onEmailChange: changeEmail,
		onPasswordChange: changePassword,
		onConfirmPasswordChange: changeConfirmPassword,
		onToggleShowPassword: () => dispatch({ type: "togglePassword" }),
		onAcceptedTermsChange: changeAcceptedTerms,
		onPasskeyLogin: () => void startPasskeyLogin(),
		onExternalLogin: (provider) => void startExternalLogin(provider),
		onResetAccountOptions: () => dispatch({ type: "resetAccountOptions" }),
	};

	return {
		branding,
		brandTitle,
		description,
		formCardProps,
		headline,
		isRegister,
		tagline: t("brand.tagline"),
	};
}

export default function LoginPage() {
	const {
		branding,
		brandTitle,
		description,
		formCardProps,
		headline,
		isRegister,
		tagline,
	} = useLoginPageController();

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={tagline}
			variant="auth"
		>
			<main className={authEntryMainClassName}>
				<LoginHero
					isRegister={isRegister}
					headline={headline}
					description={description}
				/>
				<LoginFormCard {...formCardProps} />
			</main>

			<LoginEntryFooter brandTitle={brandTitle} />
		</PublicEntryShell>
	);
}
