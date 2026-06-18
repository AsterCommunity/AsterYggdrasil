import { type FormEvent, useEffect, useMemo, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";
import { z } from "zod/v4";
import { LoginEntryFooter } from "@/components/auth/LoginEntryFooter";
import { LoginHero } from "@/components/auth/LoginHero";
import { PasswordStrengthMeter } from "@/components/auth/PasswordStrengthMeter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { usePageTitle } from "@/hooks/usePageTitle";
import { cn } from "@/lib/utils";
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

const inviteInputClassName =
	"h-12 rounded-lg border-black/10 bg-white/70 text-[#102118] shadow-[inset_0_1px_0_rgba(255,255,255,0.72)] [caret-color:#102118] [-webkit-text-fill-color:#102118] placeholder:text-slate-500 focus-visible:border-emerald-700/32 focus-visible:bg-white/82 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:border-white/14 dark:bg-neutral-950/42 dark:text-white dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] dark:[caret-color:white] dark:[-webkit-text-fill-color:white] dark:placeholder:text-white/42 dark:focus-visible:border-emerald-300/45 dark:focus-visible:bg-neutral-950/52 dark:focus-visible:ring-emerald-400/20 [&:-webkit-autofill]:border-black/10 [&:-webkit-autofill]:shadow-[0_0_0_1000px_rgba(255,255,255,0.92)_inset] [&:-webkit-autofill]:[-webkit-text-fill-color:#102118] dark:[&:-webkit-autofill]:border-white/14 dark:[&:-webkit-autofill]:shadow-[0_0_0_1000px_rgba(20,28,25,0.98)_inset] dark:[&:-webkit-autofill]:[-webkit-text-fill-color:white] dark:[&:-webkit-autofill:focus]:shadow-[0_0_0_1000px_rgba(20,28,25,0.98)_inset]";

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
	const navigate = useNavigate();
	usePageTitle(t("invite.pageTitle"));

	const brandTitle = branding.title || t("brand.name");
	const expiresAt = useMemo(() => {
		if (!state.invitation) return null;
		return new Intl.DateTimeFormat(undefined, {
			dateStyle: "medium",
			timeStyle: "short",
		}).format(new Date(state.invitation.expires_at));
	}, [state.invitation]);
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
		}).success;
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
			});
			toast.success(t("invite.accepted"));
			navigate(publicPaths.login);
		} catch (error) {
			toast.error(formatUnknownError(error));
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
			<main className="app-route-transition mx-auto grid w-full max-w-[92rem] flex-1 items-center gap-8 px-4 py-8 sm:px-8 lg:px-12 xl:grid-cols-[minmax(560px,1fr)_minmax(430px,520px)]">
				<LoginHero
					isRegister
					headline={t("invite.headline")}
					description={t("invite.heroDescription")}
				/>
				<InviteCard
					state={state}
					expiresAt={expiresAt}
					passwordScore={passwordScore}
					passwordStrengthLabel={t(passwordStrengthKey)}
					submitDisabled={submitDisabled}
					onBackToLogin={() => navigate(publicPaths.login)}
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
	expiresAt,
	onBackToLogin,
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
	expiresAt: string | null;
	onBackToLogin: () => void;
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
		<section className="auth-card-transition relative mx-auto w-full max-w-[520px] rounded-[1.35rem] border border-black/10 bg-white/78 p-6 shadow-[0_24px_90px_rgba(15,35,25,0.18),0_0_0_1px_rgba(255,255,255,0.52),0_0_58px_rgba(22,163,74,0.10)] backdrop-blur-2xl before:pointer-events-none before:absolute before:inset-0 before:rounded-[1.35rem] before:border before:border-emerald-700/8 dark:border-white/11 dark:bg-neutral-950/70 dark:shadow-[0_24px_90px_rgba(0,0,0,0.42),0_0_0_1px_rgba(120,255,190,0.04),0_0_58px_rgba(82,255,170,0.18)] dark:before:border-emerald-300/9 sm:p-9">
			<div>
				<h1 className="text-3xl font-semibold tracking-normal text-[#102118] sm:text-4xl dark:text-white">
					{t("invite.cardTitle")}
				</h1>
				<p className="mt-2 text-sm leading-6 text-slate-600 dark:text-white/72">
					{state.invitation
						? t("invite.cardDescription", {
								email: state.invitation.email,
								expiresAt,
							})
						: t("invite.loading")}
				</p>
			</div>
			{state.error ? (
				<InviteErrorPanel message={state.error} onBackToLogin={onBackToLogin} />
			) : (
				<InviteForm
					state={state}
					passwordScore={passwordScore}
					passwordStrengthLabel={passwordStrengthLabel}
					submitDisabled={submitDisabled}
					onSubmit={onSubmit}
					onUsernameChange={onUsernameChange}
					onPasswordChange={onPasswordChange}
					onConfirmPasswordChange={onConfirmPasswordChange}
					onTogglePassword={onTogglePassword}
				/>
			)}
		</section>
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
			<p className="flex items-start gap-2 rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-sm leading-6 text-red-800 dark:border-red-300/20 dark:bg-red-400/10 dark:text-red-100">
				<Icon name="CircleAlert" className="mt-1 size-4 shrink-0" />
				<span>{message}</span>
			</p>
			<Button
				type="button"
				className="h-13 rounded-lg border-0 bg-emerald-500 text-base font-semibold text-white shadow-lg shadow-emerald-950/25 hover:bg-emerald-400"
				onClick={onBackToLogin}
			>
				<Icon name="ArrowLeft" className="size-4" />
				{t("invite.backToLogin")}
			</Button>
		</div>
	);
}

function InviteForm({
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
			<InviteTextField
				id="username"
				label={t("login.username")}
				value={state.username}
				error={state.errors.username && t(state.errors.username)}
				disabled={disabled}
				onChange={onUsernameChange}
			/>
			<InvitePasswordField
				id="password"
				label={t("login.password")}
				value={state.password}
				error={state.errors.password && t(state.errors.password)}
				autoComplete="new-password"
				placeholder={t("login.passwordPlaceholder")}
				showPassword={state.showPassword}
				disabled={disabled}
				onChange={onPasswordChange}
				onTogglePassword={onTogglePassword}
			/>
			<InvitePasswordField
				id="confirm-password"
				label={t("login.confirmPassword")}
				value={state.confirmPassword}
				error={state.errors.confirmPassword && t(state.errors.confirmPassword)}
				autoComplete="new-password"
				placeholder={t("login.confirmPasswordPlaceholder")}
				showPassword={state.showPassword}
				disabled={disabled}
				onChange={onConfirmPasswordChange}
			/>
			<PasswordStrengthMeter
				label={t("login.passwordStrength")}
				value={passwordStrengthLabel}
				score={passwordScore}
			/>
			<Button
				type="submit"
				className="h-13 rounded-lg border-0 bg-emerald-500 text-base font-semibold text-white shadow-lg shadow-emerald-950/25 hover:bg-emerald-400 disabled:bg-emerald-500/55 disabled:text-white/58"
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

function InviteTextField({
	disabled,
	error,
	id,
	label,
	onChange,
	value,
}: {
	disabled: boolean;
	error?: string;
	id: string;
	label: string;
	onChange: (value: string) => void;
	value: string;
}) {
	const { t } = useTranslation();
	const errorId = `invite-${id}-error`;
	return (
		<div className="grid gap-2">
			<Label htmlFor={id} className="text-slate-700 dark:text-white/88">
				{label}
			</Label>
			<div className="relative">
				<Icon
					name="User"
					className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
				/>
				<Input
					id={id}
					required
					minLength={4}
					maxLength={16}
					autoComplete="username"
					disabled={disabled}
					placeholder={t("login.usernamePlaceholder")}
					value={value}
					className={cn(inviteInputClassName, "pr-4 pl-11")}
					aria-invalid={Boolean(error)}
					aria-describedby={error ? errorId : undefined}
					onChange={(event) => onChange(event.currentTarget.value)}
				/>
			</div>
			<FormFieldError id={errorId} message={error} />
		</div>
	);
}

function InvitePasswordField({
	autoComplete,
	disabled,
	error,
	id,
	label,
	onChange,
	onTogglePassword,
	placeholder,
	showPassword,
	value,
}: {
	autoComplete: string;
	disabled: boolean;
	error?: string;
	id: string;
	label: string;
	onChange: (value: string) => void;
	onTogglePassword?: () => void;
	placeholder: string;
	showPassword: boolean;
	value: string;
}) {
	const { t } = useTranslation();
	const errorId = `invite-${id}-error`;
	return (
		<div className="grid gap-2">
			<Label htmlFor={id} className="text-slate-700 dark:text-white/88">
				{label}
			</Label>
			<div className="relative">
				<Icon
					name="Lock"
					className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
				/>
				<Input
					id={id}
					required
					minLength={8}
					maxLength={128}
					type={showPassword ? "text" : "password"}
					autoComplete={autoComplete}
					disabled={disabled}
					placeholder={placeholder}
					value={value}
					className={cn(
						inviteInputClassName,
						onTogglePassword ? "pr-11 pl-11" : "pr-4 pl-11",
					)}
					aria-invalid={Boolean(error)}
					aria-describedby={error ? errorId : undefined}
					onChange={(event) => onChange(event.currentTarget.value)}
				/>
				{onTogglePassword ? (
					<button
						type="button"
						className="absolute top-1/2 right-3 flex size-6 -translate-y-1/2 items-center justify-center rounded-md bg-transparent text-slate-500 transition-colors outline-none hover:text-slate-800 focus-visible:ring-3 focus-visible:ring-emerald-500/18 disabled:pointer-events-none disabled:opacity-50 dark:text-white/62 dark:hover:text-white dark:focus-visible:ring-emerald-400/20"
						onClick={onTogglePassword}
						disabled={disabled}
						aria-label={
							showPassword ? t("login.hidePassword") : t("login.showPassword")
						}
					>
						<Icon name={showPassword ? "EyeSlash" : "Eye"} className="size-4" />
					</button>
				) : null}
			</div>
			<FormFieldError id={errorId} message={error} />
		</div>
	);
}

function FormFieldError({ id, message }: { id: string; message?: string }) {
	if (!message) return null;
	return (
		<p
			id={id}
			className="flex items-start gap-2 text-xs leading-5 text-red-700 dark:text-red-300"
		>
			<Icon name="CircleAlert" className="mt-0.5 size-3.5 shrink-0" />
			<span>{message}</span>
		</p>
	);
}
