import { type FormEvent, useEffect, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { z } from "zod/v4";
import type { LoginFormCardProps } from "@/components/auth/LoginFormCard";
import { useExternalAuthRecovery } from "@/components/auth/useExternalAuthRecovery";
import { useCaptchaChallenge } from "@/hooks/useCaptchaChallenge";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	confirmPasswordRequiredSchema,
	emailSchema,
	passwordSchema,
	usernameSchema,
} from "@/lib/validation";
import { accountPaths, publicPaths } from "@/routes/routePaths";
import { formatUnknownError } from "@/services/http";
import { useAuthStore } from "@/stores/authStore";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

type RegisterField =
	| "acceptedTerms"
	| "confirmPassword"
	| "email"
	| "password"
	| "username";

type RegisterFieldErrors = Partial<Record<RegisterField, string>>;

type RegisterPageState = {
	acceptedTerms: boolean;
	confirmPassword: string;
	email: string;
	errors: RegisterFieldErrors;
	loading: boolean;
	password: string;
	showPassword: boolean;
	username: string;
};

type RegisterPageAction =
	| {
			type: "field";
			name: "username" | "email" | "password" | "confirmPassword";
			value: string;
	  }
	| { type: "togglePassword" }
	| { type: "acceptedTerms"; value: boolean }
	| { type: "errors"; value: RegisterFieldErrors }
	| { type: "fieldError"; field: RegisterField; message: string | null }
	| { type: "loading"; value: boolean };

function omitFormError(
	errors: RegisterFieldErrors,
	field: RegisterField,
): RegisterFieldErrors {
	if (!errors[field]) return errors;
	const nextErrors = { ...errors };
	delete nextErrors[field];
	return nextErrors;
}

function registerPageReducer(
	state: RegisterPageState,
	action: RegisterPageAction,
): RegisterPageState {
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
		case "loading":
			return { ...state, loading: action.value };
	}
}

const initialRegisterPageState: RegisterPageState = {
	acceptedTerms: false,
	confirmPassword: "",
	email: "",
	errors: {},
	loading: false,
	password: "",
	showPassword: false,
	username: "",
};

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

function zodErrorToFormErrors(error: z.ZodError): RegisterFieldErrors {
	const nextErrors: RegisterFieldErrors = {};
	for (const issue of error.issues) {
		const field = issue.path[0];
		if (
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

export function useRegisterPageController() {
	const { t } = useTranslation();
	const [form, dispatch] = useReducer(
		registerPageReducer,
		initialRegisterPageState,
	);
	const {
		acceptedTerms,
		confirmPassword,
		email,
		errors,
		loading,
		password,
		showPassword,
		username,
	} = form;
	const register = useAuthStore((state) => state.register);
	const branding = useFrontendConfigStore((state) => state.branding);
	const allowUserRegistration = useFrontendConfigStore(
		(state) => state.allowUserRegistration,
	);
	const captchaConfig = useFrontendConfigStore((state) => state.captcha);
	const navigate = useNavigate();
	const captchaRequired =
		captchaConfig.enabled && captchaConfig.register_required;
	const captcha = useCaptchaChallenge(captchaRequired);
	const {
		externalAuthRecovery,
		submitRecovery,
		clearRecovery,
		changeRecoveryEmail,
		changeRecoveryMode,
		changeRecoveryPassword,
		changeRecoveryPasswordIdentifier,
	} = useExternalAuthRecovery({ email, identifier: "" });

	useEffect(() => {
		void import("@/lib/pwaWarmup").then(({ warmupLoginSuccessPath }) => {
			warmupLoginSuccessPath();
		});
	}, []);

	function setFieldError(field: RegisterField, message: string | null) {
		dispatch({ type: "fieldError", field, message });
	}

	function validateSingle(
		field: RegisterField,
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
		if (confirmPassword || errors.confirmPassword) {
			validateConfirmPassword(confirmPassword, value);
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
		if (externalAuthRecovery) {
			await submitRecovery();
			return;
		}

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
	}

	const passwordScore = getPasswordScore(password);
	const passwordStrengthKey =
		passwordScore <= 1
			? "login.passwordStrengthWeak"
			: passwordScore <= 3
				? "login.passwordStrengthMedium"
				: "login.passwordStrengthStrong";
	const canSubmit = registerFormSchema.safeParse({
		username,
		email,
		password,
		confirmPassword,
		acceptedTerms,
	}).success;
	const captchaReady =
		!captchaRequired ||
		(Boolean(captcha.challengeId) && captcha.answer.trim().length > 0);
	const submitDisabled = loading || !canSubmit || !captchaReady;
	const brandTitle = branding.title || t("brand.name");

	usePageTitle(t("login.registerTitle"));

	const formCardProps: LoginFormCardProps = {
		isRegister: true,
		usesAccountCreationForm: true,
		cardTitle: t("login.registerTitle"),
		cardDescription: t("login.registerDescription"),
		identifier: "",
		username,
		email,
		password,
		confirmPassword,
		errors,
		showPassword,
		acceptedTerms,
		visibleProviders: [],
		externalLoadingKey: null,
		loading,
		passkeySubmitting: false,
		passkeySupported: false,
		showPasskeyLogin: false,
		externalAuthRecovery,
		submitDisabled,
		submitLabel: t("login.registerNow"),
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
		onIdentifierChange: () => {},
		onUsernameChange: changeUsername,
		onEmailChange: changeEmail,
		onPasswordChange: changePassword,
		onConfirmPasswordChange: changeConfirmPassword,
		onToggleShowPassword: () => dispatch({ type: "togglePassword" }),
		onAcceptedTermsChange: changeAcceptedTerms,
		onPasskeyLogin: () => {},
		onExternalLogin: () => {},
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
		description: t("login.registerHeroDescription"),
		formCardProps,
		headline: t("login.registerHeadline"),
		tagline: t("brand.tagline"),
	};
}
