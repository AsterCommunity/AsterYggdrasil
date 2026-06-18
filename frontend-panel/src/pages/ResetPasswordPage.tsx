import { type FormEvent, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { z } from "zod/v4";
import { LoginEntryFooter } from "@/components/auth/LoginEntryFooter";
import { PublicEntryShell } from "@/components/layout/PublicEntryShell";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { usePageTitle } from "@/hooks/usePageTitle";
import {
	confirmPasswordRequiredSchema,
	emailSchema,
	passwordSchema,
} from "@/lib/validation";
import { publicPaths } from "@/routes/routePaths";
import { authService } from "@/services/authService";
import { ApiError, formatUnknownError } from "@/services/http";
import { useFrontendConfigStore } from "@/stores/frontendConfigStore";

type ResetStatus = "form" | "invalid" | "expired";

type ResetPasswordState = {
	confirmPassword: string;
	confirmPasswordError: string | null;
	email: string;
	emailError: string | null;
	password: string;
	passwordError: string | null;
	showPassword: boolean;
	status: ResetStatus;
	submitting: boolean;
};

type ResetPasswordAction =
	| { type: "email"; value: string; error: string | null }
	| {
			type: "password";
			value: string;
			passwordError: string | null;
			confirmPasswordError: string | null;
	  }
	| { type: "confirmPassword"; value: string; error: string | null }
	| { type: "requestError"; error: string | null }
	| {
			type: "confirmErrors";
			password: string | null;
			confirmPassword: string | null;
	  }
	| { type: "togglePassword" }
	| { type: "submitting"; value: boolean }
	| { type: "status"; value: ResetStatus };

const initialState: ResetPasswordState = {
	confirmPassword: "",
	confirmPasswordError: null,
	email: "",
	emailError: null,
	password: "",
	passwordError: null,
	showPassword: false,
	status: "form",
	submitting: false,
};

const resetPasswordSchema = z
	.object({
		password: passwordSchema,
		confirmPassword: confirmPasswordRequiredSchema,
	})
	.refine((value) => value.password === value.confirmPassword, {
		path: ["confirmPassword"],
		message: "login.passwordMismatch",
	});

function readToken(search: string) {
	return new URLSearchParams(search).get("token")?.trim() ?? "";
}

function firstIssueMessage(result: z.ZodSafeParseResult<unknown>) {
	return result.success ? null : (result.error.issues[0]?.message ?? "");
}

function reducer(
	state: ResetPasswordState,
	action: ResetPasswordAction,
): ResetPasswordState {
	switch (action.type) {
		case "email":
			return { ...state, email: action.value, emailError: action.error };
		case "password":
			return {
				...state,
				password: action.value,
				passwordError: action.passwordError,
				confirmPasswordError: action.confirmPasswordError,
			};
		case "confirmPassword":
			return {
				...state,
				confirmPassword: action.value,
				confirmPasswordError: action.error,
			};
		case "requestError":
			return { ...state, emailError: action.error };
		case "confirmErrors":
			return {
				...state,
				passwordError: action.password,
				confirmPasswordError: action.confirmPassword,
			};
		case "togglePassword":
			return { ...state, showPassword: !state.showPassword };
		case "submitting":
			return { ...state, submitting: action.value };
		case "status":
			return { ...state, status: action.value };
	}
}

export default function ResetPasswordPage() {
	const { t } = useTranslation();
	const { search } = useLocation();
	const navigate = useNavigate();
	const branding = useFrontendConfigStore((state) => state.branding);
	const token = readToken(search);
	const [state, dispatch] = useReducer(reducer, initialState);
	const {
		confirmPassword,
		confirmPasswordError,
		email,
		emailError,
		password,
		passwordError,
		showPassword,
		status,
		submitting,
	} = state;
	const isConfirmMode = token.length > 0;
	const brandTitle = branding.title || t("brand.name");

	usePageTitle(
		isConfirmMode ? t("login.resetPasswordTitle") : t("login.forgotPassword"),
	);

	async function submitRequest(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const result = emailSchema.safeParse(email);
		if (!result.success) {
			dispatch({
				type: "requestError",
				error: result.error.issues[0]?.message ?? "",
			});
			toast.error(t("login.validationFailed"));
			return;
		}

		dispatch({ type: "requestError", error: null });
		dispatch({ type: "submitting", value: true });
		try {
			await authService.requestPasswordReset({ email: result.data });
			toast.success(t("login.passwordResetRequested"));
			navigate(publicPaths.login, { replace: true });
		} catch (error) {
			toast.error(formatUnknownError(error));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	async function submitConfirm(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (status !== "form") return;

		const result = resetPasswordSchema.safeParse({
			password,
			confirmPassword,
		});
		if (!result.success) {
			dispatch({
				type: "confirmErrors",
				password: firstIssueMessage(passwordSchema.safeParse(password)),
				confirmPassword:
					firstIssueMessage(
						confirmPasswordRequiredSchema.safeParse(confirmPassword),
					) ?? (password === confirmPassword ? null : "login.passwordMismatch"),
			});
			toast.error(t("login.validationFailed"));
			return;
		}

		dispatch({
			type: "confirmErrors",
			password: null,
			confirmPassword: null,
		});
		dispatch({ type: "submitting", value: true });
		try {
			await authService.confirmPasswordReset({
				token,
				new_password: result.data.password,
			});
			navigate(`${publicPaths.login}?password_reset=success`, {
				replace: true,
			});
		} catch (error) {
			if (error instanceof ApiError) {
				if (error.code === "auth.contact_verification_invalid") {
					dispatch({ type: "status", value: "invalid" });
					return;
				}
				if (error.code === "auth.contact_verification_expired") {
					dispatch({ type: "status", value: "expired" });
					return;
				}
			}
			toast.error(formatUnknownError(error));
		} finally {
			dispatch({ type: "submitting", value: false });
		}
	}

	const title = isConfirmMode
		? status === "invalid"
			? t("login.resetPasswordInvalidTitle")
			: status === "expired"
				? t("login.resetPasswordExpiredTitle")
				: t("login.resetPasswordTitle")
		: t("login.forgotPassword");
	const description = isConfirmMode
		? status === "invalid"
			? t("login.resetPasswordInvalidDescription")
			: status === "expired"
				? t("login.resetPasswordExpiredDescription")
				: t("login.resetPasswordDescription")
		: t("login.passwordResetRequestDescription");

	return (
		<PublicEntryShell
			branding={branding}
			title={brandTitle}
			tagline={t("brand.tagline")}
			variant="auth"
			hideLanguageOnMobile
		>
			<main className="app-route-transition mx-auto flex w-full max-w-[36rem] flex-1 items-center px-4 py-8 sm:px-8">
				<section className="w-full rounded-[1.35rem] border border-black/10 bg-white/78 p-6 shadow-[0_24px_90px_rgba(15,35,25,0.18),0_0_0_1px_rgba(255,255,255,0.52)] backdrop-blur-2xl dark:border-white/11 dark:bg-neutral-950/70 dark:shadow-[0_24px_90px_rgba(0,0,0,0.42)] sm:p-9">
					<div>
						<h1 className="text-3xl font-semibold tracking-normal text-[#102118] dark:text-white">
							{title}
						</h1>
						<p className="mt-2 text-sm leading-6 text-slate-600 dark:text-white/72">
							{description}
						</p>
					</div>

					{isConfirmMode && status !== "form" ? (
						<div className="mt-7 grid gap-3">
							<Button
								type="button"
								className="h-12 rounded-lg"
								onClick={() => navigate(publicPaths.login)}
							>
								<Icon name="SignIn" className="size-4" />
								{t("login.backToLogin")}
							</Button>
							<Button
								type="button"
								variant="outline"
								className="h-12 rounded-lg"
								onClick={() => navigate(publicPaths.resetPassword)}
							>
								<Icon name="EnvelopeSimple" className="size-4" />
								{t("login.requestNewResetLink")}
							</Button>
						</div>
					) : isConfirmMode ? (
						<form className="mt-7 grid gap-4" onSubmit={submitConfirm}>
							<PasswordField
								id="reset-password"
								label={t("login.password")}
								value={password}
								error={passwordError}
								showPassword={showPassword}
								autoComplete="new-password"
								onChange={(value) => {
									dispatch({
										type: "password",
										value,
										passwordError: firstIssueMessage(
											passwordSchema.safeParse(value),
										),
										confirmPasswordError: confirmPassword
											? value === confirmPassword
												? null
												: "login.passwordMismatch"
											: confirmPasswordError,
									});
								}}
								onToggleShowPassword={() =>
									dispatch({ type: "togglePassword" })
								}
							/>
							<PasswordField
								id="reset-confirm-password"
								label={t("login.confirmPassword")}
								value={confirmPassword}
								error={confirmPasswordError}
								showPassword={showPassword}
								autoComplete="new-password"
								onChange={(value) => {
									dispatch({
										type: "confirmPassword",
										value,
										error:
											firstIssueMessage(
												confirmPasswordRequiredSchema.safeParse(value),
											) ??
											(value === password ? null : "login.passwordMismatch"),
									});
								}}
								onToggleShowPassword={() =>
									dispatch({ type: "togglePassword" })
								}
							/>
							<Button
								type="submit"
								disabled={
									submitting ||
									!resetPasswordSchema.safeParse({
										password,
										confirmPassword,
									}).success
								}
								className="h-12 rounded-lg bg-emerald-500 text-base font-semibold text-white shadow-lg shadow-emerald-950/25 hover:bg-emerald-400 disabled:bg-emerald-500/55"
							>
								<Icon
									name={submitting ? "Spinner" : "Key"}
									className={submitting ? "size-4 animate-spin" : "size-4"}
								/>
								{submitting
									? t("login.resetPasswordSubmitting")
									: t("login.resetPasswordSubmit")}
							</Button>
						</form>
					) : (
						<form className="mt-7 grid gap-4" onSubmit={submitRequest}>
							<div className="grid gap-2">
								<Label
									htmlFor="reset-email"
									className="text-slate-700 dark:text-white/88"
								>
									{t("login.email")}
								</Label>
								<div className="relative">
									<Icon
										name="EnvelopeSimple"
										className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
									/>
									<Input
										id="reset-email"
										type="email"
										value={email}
										onChange={(event) => {
											const nextEmail = event.currentTarget.value;
											dispatch({
												type: "email",
												value: nextEmail,
												error: emailError
													? firstIssueMessage(emailSchema.safeParse(nextEmail))
													: emailError,
											});
										}}
										autoComplete="email"
										placeholder={t("login.emailPlaceholder")}
										className="h-12 rounded-lg border-black/10 bg-white/70 pr-4 pl-11 text-[#102118] shadow-[inset_0_1px_0_rgba(255,255,255,0.72)] placeholder:text-slate-500 focus-visible:border-emerald-700/32 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:border-white/14 dark:bg-neutral-950/42 dark:text-white dark:placeholder:text-white/42"
										aria-invalid={Boolean(emailError)}
										aria-describedby={
											emailError ? "reset-email-error" : undefined
										}
									/>
								</div>
								<FormFieldError
									id="reset-email-error"
									message={emailError && t(emailError)}
								/>
							</div>
							<Button
								type="submit"
								disabled={submitting || !emailSchema.safeParse(email).success}
								className="h-12 rounded-lg bg-emerald-500 text-base font-semibold text-white shadow-lg shadow-emerald-950/25 hover:bg-emerald-400 disabled:bg-emerald-500/55"
							>
								<Icon
									name={submitting ? "Spinner" : "EnvelopeSimple"}
									className={submitting ? "size-4 animate-spin" : "size-4"}
								/>
								{submitting
									? t("login.passwordResetRequestSubmitting")
									: t("login.passwordResetRequestSubmit")}
							</Button>
							<p className="text-center text-sm text-slate-700 dark:text-white/78">
								<Link
									to={publicPaths.login}
									className="font-semibold text-emerald-700 underline-offset-4 hover:text-emerald-600 hover:underline dark:text-emerald-300 dark:hover:text-emerald-200"
								>
									{t("login.backToLogin")}
								</Link>
							</p>
						</form>
					)}
				</section>
			</main>
			<LoginEntryFooter brandTitle={brandTitle} />
		</PublicEntryShell>
	);
}

function PasswordField({
	id,
	label,
	value,
	error,
	showPassword,
	autoComplete,
	onChange,
	onToggleShowPassword,
}: {
	id: string;
	label: string;
	value: string;
	error: string | null;
	showPassword: boolean;
	autoComplete: string;
	onChange: (value: string) => void;
	onToggleShowPassword: () => void;
}) {
	const { t } = useTranslation();
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
					type={showPassword ? "text" : "password"}
					value={value}
					onChange={(event) => onChange(event.currentTarget.value)}
					autoComplete={autoComplete}
					maxLength={128}
					className="h-12 rounded-lg border-black/10 bg-white/70 pr-11 pl-11 text-[#102118] shadow-[inset_0_1px_0_rgba(255,255,255,0.72)] placeholder:text-slate-500 focus-visible:border-emerald-700/32 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:border-white/14 dark:bg-neutral-950/42 dark:text-white dark:placeholder:text-white/42"
					aria-invalid={Boolean(error)}
					aria-describedby={error ? `${id}-error` : undefined}
				/>
				<button
					type="button"
					className="absolute top-1/2 right-3 flex size-6 -translate-y-1/2 items-center justify-center rounded-md bg-transparent text-slate-500 transition-colors outline-none hover:text-slate-800 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:text-white/62 dark:hover:text-white"
					onClick={onToggleShowPassword}
					aria-label={
						showPassword ? t("login.hidePassword") : t("login.showPassword")
					}
				>
					<Icon name={showPassword ? "EyeSlash" : "Eye"} className="size-4" />
				</button>
			</div>
			<FormFieldError id={`${id}-error`} message={error && t(error)} />
		</div>
	);
}

function FormFieldError({
	id,
	message,
}: {
	id: string;
	message?: string | null;
}) {
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
