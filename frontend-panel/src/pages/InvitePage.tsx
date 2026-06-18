import { type FormEvent, useEffect, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { z } from "zod/v4";
import {
	AuthFormCard,
	AuthFormFieldError,
	AuthIconTextField,
	AuthPasswordField,
	authEntryMainClassName,
	authPrimaryButtonClassName,
} from "@/components/auth/AuthFormPrimitives";
import { CaptchaField } from "@/components/auth/CaptchaField";
import { LoginEntryFooter } from "@/components/auth/LoginEntryFooter";
import { LoginHero } from "@/components/auth/LoginHero";
import { PasswordStrengthMeter } from "@/components/auth/PasswordStrengthMeter";
import { DateTimeText } from "@/components/common/DateTimeText";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { useCaptchaChallenge } from "@/hooks/useCaptchaChallenge";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	confirmPasswordRequiredSchema,
	passwordSchema,
	usernameSchema,
} from "@/lib/validation";
import { publicPaths } from "@/routes/routePaths";
import { authService } from "@/services/authService";
import { formatUnknownError } from "@/services/http";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";
import type { PublicUserInvitationInfo } from "@/types/api";

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

type InviteState = {
	username: string;
	password: string;
	confirmPassword: string;
	errors: InviteFormErrors;
	showPassword: boolean;
	loading: boolean;
	submitting: boolean;
	invitation: PublicUserInvitationInfo | null;
	error: string | null;
};

type InviteFormField = "username" | "password" | "confirmPassword";
type InviteFormErrors = Partial<Record<InviteFormField, string>>;

type InviteAction =
	| {
			type: "field";
			name: "username" | "password" | "confirmPassword";
			value: string;
	  }
	| { type: "errors"; value: InviteFormErrors }
	| { type: "fieldError"; field: InviteFormField; message: string | null }
	| { type: "togglePassword" }
	| { type: "loading"; value: boolean }
	| { type: "submitting"; value: boolean }
	| { type: "loaded"; invitation: PublicUserInvitationInfo }
	| { type: "error"; message: string };

const initialState: InviteState = {
	username: "",
	password: "",
	confirmPassword: "",
	errors: {},
	showPassword: false,
	loading: true,
	submitting: false,
	invitation: null,
	error: null,
};

const acceptInvitationFormSchema = z
	.object({
		username: usernameSchema,
		password: passwordSchema,
		confirmPassword: confirmPasswordRequiredSchema,
	})
	.refine((value) => value.password === value.confirmPassword, {
		path: ["confirmPassword"],
		message: "login.passwordMismatch",
	});

function omitInviteFormError(
	errors: InviteFormErrors,
	field: InviteFormField,
): InviteFormErrors {
	if (!errors[field]) return errors;
	const nextErrors = { ...errors };
	delete nextErrors[field];
	return nextErrors;
}

function zodErrorToInviteFormErrors(error: z.ZodError): InviteFormErrors {
	const nextErrors: InviteFormErrors = {};
	for (const issue of error.issues) {
		const field = issue.path[0];
		if (
			field === "username" ||
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

function reducer(state: InviteState, action: InviteAction): InviteState {
	switch (action.type) {
		case "field":
			return {
				...state,
				[action.name]: action.value,
				errors: omitInviteFormError(state.errors, action.name),
			};
		case "errors":
			return { ...state, errors: action.value };
		case "fieldError":
			return {
				...state,
				errors:
					action.message === null
						? omitInviteFormError(state.errors, action.field)
						: { ...state.errors, [action.field]: action.message },
			};
		case "togglePassword":
			return { ...state, showPassword: !state.showPassword };
		case "loading":
			return { ...state, loading: action.value };
		case "submitting":
			return { ...state, submitting: action.value };
		case "loaded":
			return {
				...state,
				loading: false,
				invitation: action.invitation,
				error: null,
				errors: {},
			};
		case "error":
			return {
				...state,
				loading: false,
				invitation: null,
				error: action.message,
				errors: {},
			};
	}
}

export default function InvitePage() {
	const { t } = useTranslation();
	const { token = "" } = useParams();
	const [state, dispatch] = useReducer(reducer, initialState);
	const branding = useFrontendConfigStore((store) => store.branding);
	const captchaConfig = useFrontendConfigStore((store) => store.captcha);
	const navigate = useNavigate();
	usePageTitle(t("invite.pageTitle"));
	const captchaRequired =
		captchaConfig.enabled && captchaConfig.invitation_accept_required;
	const captcha = useCaptchaChallenge(captchaRequired);

	const brandTitle = branding.title || t("brand.name");
	const passwordScore = getPasswordScore(state.password);
	const passwordStrengthKey =
		passwordScore <= 1
			? "login.passwordStrengthWeak"
			: passwordScore <= 3
				? "login.passwordStrengthMedium"
				: "login.passwordStrengthStrong";
	const canSubmit =
		Boolean(state.invitation) &&
		acceptInvitationFormSchema.safeParse({
			username: state.username,
			password: state.password,
			confirmPassword: state.confirmPassword,
		}).success &&
		(!captchaRequired ||
			(Boolean(captcha.challengeId) && captcha.answer.trim().length > 0));
	const submitDisabled = state.loading || state.submitting || !canSubmit;

	useEffect(() => {
		if (!token.trim()) {
			dispatch({ type: "error", message: t("invite.invalid") });
			return;
		}
		let active = true;
		dispatch({ type: "loading", value: true });
		authService
			.verifyInvitation(token)
			.then((invitation) => {
				if (active) dispatch({ type: "loaded", invitation });
			})
			.catch((error) => {
				if (active) {
					dispatch({ type: "error", message: formatUnknownError(error) });
				}
			});
		return () => {
			active = false;
		};
	}, [token, t]);

	async function submit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!state.invitation) return;
		const validation = acceptInvitationFormSchema.safeParse({
			username: state.username,
			password: state.password,
			confirmPassword: state.confirmPassword,
		});
		if (!validation.success) {
			dispatch({
				type: "errors",
				value: zodErrorToInviteFormErrors(validation.error),
			});
			toast.error(t("login.validationFailed"));
			return;
		}
		dispatch({ type: "errors", value: {} });
		dispatch({ type: "submitting", value: true });
		try {
			await authService.acceptInvitation(token, {
				username: validation.data.username,
				password: validation.data.password,
				captcha_challenge_id: captchaRequired
					? (captcha.challengeId ?? "")
					: undefined,
				captcha_answer: captchaRequired ? captcha.answer : undefined,
			});
			toast.success(t("invite.accepted"));
			navigate(publicPaths.login);
		} catch (error) {
			toast.error(formatUnknownError(error));
			if (captchaRequired) void captcha.refresh();
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	function setFieldError(field: InviteFormField, message: string | null) {
		dispatch({ type: "fieldError", field, message });
	}

	function validateSingle(
		field: InviteFormField,
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

	function changePassword(value: string) {
		dispatch({ type: "field", name: "password", value });
		validateSingle("password", value, passwordSchema);
		if (state.confirmPassword || state.errors.confirmPassword) {
			validateConfirmPassword(state.confirmPassword, value);
		}
	}

	function changeConfirmPassword(value: string) {
		dispatch({ type: "field", name: "confirmPassword", value });
		validateConfirmPassword(value, state.password);
	}

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={t("brand.tagline")}
			variant="auth"
		>
			<main className={authEntryMainClassName}>
				<LoginHero
					isRegister
					headline={t("invite.headline")}
					description={t("invite.heroDescription")}
				/>
				<InviteCard
					state={state}
					passwordScore={passwordScore}
					passwordStrengthLabel={t(passwordStrengthKey)}
					submitDisabled={submitDisabled}
					captchaAnswer={captcha.answer}
					captchaImageBase64={captcha.imageBase64}
					captchaLoadError={captcha.error}
					captchaLoading={captcha.loading}
					captchaRequired={captchaRequired}
					onBackToLogin={() => navigate(publicPaths.login)}
					onCaptchaAnswerChange={captcha.setAnswer}
					onCaptchaRefresh={() => void captcha.refresh()}
					onSubmit={submit}
					onUsernameChange={changeUsername}
					onPasswordChange={changePassword}
					onConfirmPasswordChange={changeConfirmPassword}
					onTogglePassword={() => dispatch({ type: "togglePassword" })}
				/>
			</main>

			<LoginEntryFooter brandTitle={brandTitle} />
		</PublicEntryShell>
	);
}

function InviteCard({
	captchaAnswer,
	captchaImageBase64,
	captchaLoadError,
	captchaLoading,
	captchaRequired,
	onBackToLogin,
	onCaptchaAnswerChange,
	onCaptchaRefresh,
	onConfirmPasswordChange,
	onPasswordChange,
	onSubmit,
	onTogglePassword,
	onUsernameChange,
	passwordScore,
	passwordStrengthLabel,
	state,
	submitDisabled,
}: {
	captchaAnswer: string;
	captchaImageBase64: string | null;
	captchaLoadError: string | null;
	captchaLoading: boolean;
	captchaRequired: boolean;
	onBackToLogin: () => void;
	onCaptchaAnswerChange: (value: string) => void;
	onCaptchaRefresh: () => void;
	onConfirmPasswordChange: (value: string) => void;
	onPasswordChange: (value: string) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onTogglePassword: () => void;
	onUsernameChange: (value: string) => void;
	passwordScore: number;
	passwordStrengthLabel: string;
	state: InviteState;
	submitDisabled: boolean;
}) {
	const { t } = useTranslation();
	return (
		<AuthFormCard
			title={t("invite.cardTitle")}
			description={
				state.invitation ? (
					<>
						{t("invite.cardDescriptionEmail", {
							email: state.invitation.email,
						})}{" "}
						{t("invite.cardDescriptionExpiresAt")}{" "}
						<DateTimeText value={state.invitation.expires_at} />.
					</>
				) : (
					t("invite.loading")
				)
			}
		>
			{state.error ? (
				<InviteErrorPanel message={state.error} onBackToLogin={onBackToLogin} />
			) : (
				<InviteForm
					state={state}
					passwordScore={passwordScore}
					passwordStrengthLabel={passwordStrengthLabel}
					submitDisabled={submitDisabled}
					captchaAnswer={captchaAnswer}
					captchaImageBase64={captchaImageBase64}
					captchaLoadError={captchaLoadError}
					captchaLoading={captchaLoading}
					captchaRequired={captchaRequired}
					onCaptchaAnswerChange={onCaptchaAnswerChange}
					onCaptchaRefresh={onCaptchaRefresh}
					onSubmit={onSubmit}
					onUsernameChange={onUsernameChange}
					onPasswordChange={onPasswordChange}
					onConfirmPasswordChange={onConfirmPasswordChange}
					onTogglePassword={onTogglePassword}
				/>
			)}
		</AuthFormCard>
	);
}

function InviteErrorPanel({
	message,
	onBackToLogin,
}: {
	message: string;
	onBackToLogin: () => void;
}) {
	const { t } = useTranslation();
	return (
		<div className="mt-7 grid gap-4">
			<AuthFormFieldError
				id="invite-token-error"
				message={message}
				tone="panel"
			/>
			<Button
				type="button"
				className={authPrimaryButtonClassName}
				onClick={onBackToLogin}
			>
				<Icon name="ArrowLeft" className="size-4" />
				{t("invite.backToLogin")}
			</Button>
		</div>
	);
}

function InviteForm({
	captchaAnswer,
	captchaImageBase64,
	captchaLoadError,
	captchaLoading,
	captchaRequired,
	onCaptchaAnswerChange,
	onCaptchaRefresh,
	onConfirmPasswordChange,
	onPasswordChange,
	onSubmit,
	onTogglePassword,
	onUsernameChange,
	passwordScore,
	passwordStrengthLabel,
	state,
	submitDisabled,
}: {
	captchaAnswer: string;
	captchaImageBase64: string | null;
	captchaLoadError: string | null;
	captchaLoading: boolean;
	captchaRequired: boolean;
	onCaptchaAnswerChange: (value: string) => void;
	onCaptchaRefresh: () => void;
	onConfirmPasswordChange: (value: string) => void;
	onPasswordChange: (value: string) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onTogglePassword: () => void;
	onUsernameChange: (value: string) => void;
	passwordScore: number;
	passwordStrengthLabel: string;
	state: InviteState;
	submitDisabled: boolean;
}) {
	const { t } = useTranslation();
	const disabled = state.loading || state.submitting;
	return (
		<form className="mt-7 grid gap-4" onSubmit={onSubmit} noValidate>
			<AuthIconTextField
				id="username"
				label={t("login.username")}
				value={state.username}
				error={state.errors.username && t(state.errors.username)}
				icon="User"
				autoComplete="username"
				placeholder={t("login.usernamePlaceholder")}
				minLength={4}
				maxLength={16}
				disabled={disabled}
				onChange={onUsernameChange}
			/>
			<AuthPasswordField
				id="password"
				label={t("login.password")}
				value={state.password}
				error={state.errors.password && t(state.errors.password)}
				autoComplete="new-password"
				placeholder={t("login.passwordPlaceholder")}
				maxLength={128}
				showPassword={state.showPassword}
				disabled={disabled}
				onChange={onPasswordChange}
				onToggleShowPassword={onTogglePassword}
			/>
			<AuthPasswordField
				id="confirm-password"
				label={t("login.confirmPassword")}
				value={state.confirmPassword}
				error={state.errors.confirmPassword && t(state.errors.confirmPassword)}
				autoComplete="new-password"
				placeholder={t("login.confirmPasswordPlaceholder")}
				maxLength={128}
				showPassword={state.showPassword}
				disabled={disabled}
				onChange={onConfirmPasswordChange}
			/>
			<PasswordStrengthMeter
				label={t("login.passwordStrength")}
				value={passwordStrengthLabel}
				score={passwordScore}
			/>
			{captchaRequired ? (
				<CaptchaField
					answer={captchaAnswer}
					disabled={disabled}
					imageBase64={captchaImageBase64}
					loading={captchaLoading}
					loadError={captchaLoadError}
					onAnswerChange={onCaptchaAnswerChange}
					onRefresh={onCaptchaRefresh}
				/>
			) : null}
			<Button
				type="submit"
				className={authPrimaryButtonClassName}
				disabled={submitDisabled}
			>
				<Icon
					name={state.submitting ? "Spinner" : "SignIn"}
					className={state.submitting ? "size-4 animate-spin" : "size-4"}
				/>
				{state.submitting ? t("invite.accepting") : t("invite.accept")}
			</Button>
			<p className="text-center text-sm text-slate-700 dark:text-white/78">
				{t("login.hasAccount")}{" "}
				<Link
					to={publicPaths.login}
					className="font-semibold text-emerald-700 underline-offset-4 hover:text-emerald-600 hover:underline dark:text-emerald-300 dark:hover:text-emerald-200"
				>
					{t("nav.login")}
				</Link>
			</p>
		</form>
	);
}
