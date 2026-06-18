import type { FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
	externalAuthKindIconPath,
	normalizeExternalAuthIconUrl,
} from "@/lib/externalAuthProviders";
import { cn } from "@/lib/utils";
import { publicPaths } from "@/routes/routePaths";
import type { ExternalAuthPublicProvider } from "@/types/api";
import { PasswordStrengthMeter } from "./PasswordStrengthMeter";

const loginInputClassName =
	"h-12 rounded-lg border-black/10 bg-white/70 text-[#102118] shadow-[inset_0_1px_0_rgba(255,255,255,0.72)] [caret-color:#102118] [-webkit-text-fill-color:#102118] placeholder:text-slate-500 focus-visible:border-emerald-700/32 focus-visible:bg-white/82 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:border-white/14 dark:bg-neutral-950/42 dark:text-white dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] dark:[caret-color:white] dark:[-webkit-text-fill-color:white] dark:placeholder:text-white/42 dark:focus-visible:border-emerald-300/45 dark:focus-visible:bg-neutral-950/52 dark:focus-visible:ring-emerald-400/20 [&:-webkit-autofill]:border-black/10 [&:-webkit-autofill]:shadow-[0_0_0_1000px_rgba(255,255,255,0.92)_inset] [&:-webkit-autofill]:[-webkit-text-fill-color:#102118] dark:[&:-webkit-autofill]:border-white/14 dark:[&:-webkit-autofill]:shadow-[0_0_0_1000px_rgba(20,28,25,0.98)_inset] dark:[&:-webkit-autofill]:[-webkit-text-fill-color:white] dark:[&:-webkit-autofill:focus]:shadow-[0_0_0_1000px_rgba(20,28,25,0.98)_inset]";

type LoginFormField =
	| "identifier"
	| "username"
	| "email"
	| "password"
	| "confirmPassword"
	| "acceptedTerms";

type LoginFormErrors = Partial<Record<LoginFormField, string>>;

export type LoginFormCardProps = {
	isRegister: boolean;
	usesAccountCreationForm: boolean;
	cardTitle: string;
	cardDescription: string;
	identifier: string;
	username: string;
	email: string;
	password: string;
	confirmPassword: string;
	errors: LoginFormErrors;
	showPassword: boolean;
	acceptedTerms: boolean;
	visibleProviders: ExternalAuthPublicProvider[];
	externalLoadingKey: string | null;
	loading: boolean;
	passkeySubmitting: boolean;
	passkeySupported: boolean;
	showPasskeyLogin: boolean;
	submitDisabled: boolean;
	submitLabel: string;
	passwordScore: number;
	passwordStrengthLabel: string;
	allowUserRegistration: boolean;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onIdentifierChange: (value: string) => void;
	onUsernameChange: (value: string) => void;
	onEmailChange: (value: string) => void;
	onPasswordChange: (value: string) => void;
	onConfirmPasswordChange: (value: string) => void;
	onToggleShowPassword: () => void;
	onAcceptedTermsChange: (value: boolean) => void;
	onPasskeyLogin: () => void;
	onExternalLogin: (provider: ExternalAuthPublicProvider) => void;
	onResetAccountOptions: () => void;
};

export function LoginFormCard(props: LoginFormCardProps) {
	const {
		allowUserRegistration,
		cardDescription,
		cardTitle,
		isRegister,
		loading,
		onResetAccountOptions,
		onSubmit,
		submitDisabled,
		submitLabel,
		usesAccountCreationForm,
	} = props;
	return (
		<section
			key={isRegister ? "register" : "login"}
			className="auth-card-transition relative mx-auto w-full max-w-[520px] rounded-[1.35rem] border border-black/10 bg-white/78 p-6 shadow-[0_24px_90px_rgba(15,35,25,0.18),0_0_0_1px_rgba(255,255,255,0.52),0_0_58px_rgba(22,163,74,0.10)] backdrop-blur-2xl before:pointer-events-none before:absolute before:inset-0 before:rounded-[1.35rem] before:border before:border-emerald-700/8 dark:border-white/11 dark:bg-neutral-950/70 dark:shadow-[0_24px_90px_rgba(0,0,0,0.42),0_0_0_1px_rgba(120,255,190,0.04),0_0_58px_rgba(82,255,170,0.18)] dark:before:border-emerald-300/9 sm:p-9"
		>
			<div>
				<h1 className="text-3xl font-semibold tracking-normal text-[#102118] sm:text-4xl dark:text-white">
					{cardTitle}
				</h1>
				<p className="mt-2 text-sm leading-6 text-slate-600 dark:text-white/72">
					{cardDescription}
				</p>
			</div>
			<form className="mt-7 grid gap-4" onSubmit={onSubmit} noValidate>
				{usesAccountCreationForm ? (
					<AccountCreationFields {...props} />
				) : (
					<IdentifierField {...props} />
				)}
				<LoginPasswordField {...props} />
				{usesAccountCreationForm ? <RegistrationFields {...props} /> : null}
				<Button
					type="submit"
					disabled={submitDisabled}
					className="h-13 rounded-lg border-0 bg-emerald-500 text-base font-semibold text-white shadow-lg shadow-emerald-950/25 hover:bg-emerald-400 disabled:bg-emerald-500/55 disabled:text-white/58"
				>
					<Icon
						name={loading ? "Spinner" : "SignIn"}
						className={loading ? "size-4 animate-spin" : "size-4"}
					/>
					{submitLabel}
				</Button>
				<AuthAlternatives {...props} />
				{allowUserRegistration ? (
					<AccountModeLink
						isRegister={isRegister}
						onResetAccountOptions={onResetAccountOptions}
					/>
				) : null}
			</form>
		</section>
	);
}

function AccountCreationFields({
	email,
	errors,
	onEmailChange,
	onUsernameChange,
	username,
}: LoginFormCardProps) {
	const { t } = useTranslation();
	return (
		<>
			<IconTextField
				id="username"
				label={t("login.username")}
				value={username}
				error={errors.username && t(errors.username)}
				icon="User"
				autoComplete="username"
				placeholder={t("login.usernamePlaceholder")}
				minLength={4}
				maxLength={16}
				onChange={onUsernameChange}
			/>
			<IconTextField
				id="email"
				label={t("login.email")}
				value={email}
				error={errors.email && t(errors.email)}
				icon="EnvelopeSimple"
				type="email"
				autoComplete="email"
				placeholder={t("login.emailPlaceholder")}
				onChange={onEmailChange}
			/>
		</>
	);
}

function IdentifierField({
	errors,
	identifier,
	onIdentifierChange,
}: LoginFormCardProps) {
	const { t } = useTranslation();
	return (
		<IconTextField
			id="identifier"
			label={t("login.identifier")}
			value={identifier}
			error={errors.identifier && t(errors.identifier)}
			icon="User"
			autoComplete="username"
			placeholder={t("login.identifierPlaceholder")}
			onChange={onIdentifierChange}
		/>
	);
}

function LoginPasswordField({
	errors,
	onPasswordChange,
	onToggleShowPassword,
	password,
	showPassword,
	usesAccountCreationForm,
}: LoginFormCardProps) {
	const { t } = useTranslation();
	return (
		<PasswordInputField
			id="password"
			label={t("login.password")}
			value={password}
			error={errors.password && t(errors.password)}
			autoComplete={
				usesAccountCreationForm ? "new-password" : "current-password"
			}
			placeholder={t("login.passwordPlaceholder")}
			maxLength={usesAccountCreationForm ? 128 : undefined}
			showPassword={showPassword}
			aside={
				usesAccountCreationForm ? null : (
					<Link
						to={publicPaths.resetPassword}
						className="text-xs font-semibold text-emerald-700 underline-offset-4 hover:text-emerald-600 hover:underline dark:text-emerald-300 dark:hover:text-emerald-200"
					>
						{t("login.forgotPassword")}
					</Link>
				)
			}
			onChange={onPasswordChange}
			onToggleShowPassword={onToggleShowPassword}
		/>
	);
}

function RegistrationFields({
	acceptedTerms,
	confirmPassword,
	errors,
	isRegister,
	onAcceptedTermsChange,
	onConfirmPasswordChange,
	passwordScore,
	passwordStrengthLabel,
	showPassword,
}: LoginFormCardProps) {
	const { t } = useTranslation();
	return (
		<>
			<PasswordInputField
				id="confirm-password"
				label={t("login.confirmPassword")}
				value={confirmPassword}
				error={errors.confirmPassword && t(errors.confirmPassword)}
				autoComplete="new-password"
				placeholder={t("login.confirmPasswordPlaceholder")}
				maxLength={128}
				showPassword={showPassword}
				onChange={onConfirmPasswordChange}
			/>
			<PasswordStrengthMeter
				label={t("login.passwordStrength")}
				value={passwordStrengthLabel}
				score={passwordScore}
			/>
			{isRegister ? (
				<TermsField
					checked={acceptedTerms}
					error={errors.acceptedTerms && t(errors.acceptedTerms)}
					onChange={onAcceptedTermsChange}
				/>
			) : null}
		</>
	);
}

function IconTextField({
	autoComplete,
	error,
	icon,
	id,
	label,
	maxLength,
	minLength,
	onChange,
	placeholder,
	type = "text",
	value,
}: {
	autoComplete: string;
	error?: string;
	icon: "EnvelopeSimple" | "User";
	id: string;
	label: string;
	maxLength?: number;
	minLength?: number;
	onChange: (value: string) => void;
	placeholder: string;
	type?: string;
	value: string;
}) {
	return (
		<div className="grid gap-2">
			<Label htmlFor={id} className="text-slate-700 dark:text-white/88">
				{label}
			</Label>
			<div className="relative">
				<Icon
					name={icon}
					className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
				/>
				<Input
					id={id}
					type={type}
					value={value}
					onChange={(event) => onChange(event.currentTarget.value)}
					autoComplete={autoComplete}
					minLength={minLength}
					maxLength={maxLength}
					placeholder={placeholder}
					className={cn(loginInputClassName, "pr-4 pl-11")}
					aria-invalid={Boolean(error)}
					aria-describedby={error ? `${id}-error` : undefined}
					required
				/>
			</div>
			<FormFieldError id={`${id}-error`} message={error} />
		</div>
	);
}

function PasswordInputField({
	aside,
	autoComplete,
	error,
	id,
	label,
	maxLength,
	onChange,
	onToggleShowPassword,
	placeholder,
	showPassword,
	value,
}: {
	aside?: React.ReactNode;
	autoComplete: string;
	error?: string;
	id: string;
	label: string;
	maxLength?: number;
	onChange: (value: string) => void;
	onToggleShowPassword?: () => void;
	placeholder: string;
	showPassword: boolean;
	value: string;
}) {
	const { t } = useTranslation();
	return (
		<div className="grid gap-2">
			<div className="flex items-center justify-between gap-3">
				<Label htmlFor={id} className="text-slate-700 dark:text-white/88">
					{label}
				</Label>
				{aside}
			</div>
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
					placeholder={placeholder}
					maxLength={maxLength}
					className={cn(
						loginInputClassName,
						onToggleShowPassword ? "pr-11 pl-11" : "pr-4 pl-11",
					)}
					aria-invalid={Boolean(error)}
					aria-describedby={error ? `${id}-error` : undefined}
					required
				/>
				{onToggleShowPassword ? (
					<button
						type="button"
						className="absolute top-1/2 right-3 flex size-6 -translate-y-1/2 items-center justify-center rounded-md bg-transparent text-slate-500 transition-colors outline-none hover:text-slate-800 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:text-white/62 dark:hover:text-white dark:focus-visible:ring-emerald-400/20"
						onClick={onToggleShowPassword}
						aria-label={
							showPassword ? t("login.hidePassword") : t("login.showPassword")
						}
					>
						<Icon name={showPassword ? "EyeSlash" : "Eye"} className="size-4" />
					</button>
				) : null}
			</div>
			<FormFieldError id={`${id}-error`} message={error} />
		</div>
	);
}

function TermsField({
	checked,
	error,
	onChange,
}: {
	checked: boolean;
	error?: string;
	onChange: (value: boolean) => void;
}) {
	const { t } = useTranslation();
	return (
		<>
			<label className="flex items-center gap-3 rounded-lg border border-black/10 bg-white/55 p-3 text-sm leading-5 text-slate-700 dark:border-white/8 dark:bg-white/5 dark:text-white/76">
				<input
					type="checkbox"
					checked={checked}
					onChange={(event) => onChange(event.currentTarget.checked)}
					className="peer sr-only"
					aria-invalid={Boolean(error)}
					aria-describedby={error ? "accepted-terms-error" : undefined}
				/>
				<span className="flex size-5 shrink-0 items-center justify-center rounded-md border border-black/16 bg-white/70 text-transparent transition-colors peer-checked:border-emerald-700/40 peer-checked:bg-emerald-600/12 peer-checked:text-emerald-700 peer-focus-visible:ring-3 peer-focus-visible:ring-emerald-500/20 dark:border-white/16 dark:bg-black/20 dark:peer-checked:border-emerald-300/60 dark:peer-checked:bg-emerald-400/20 dark:peer-checked:text-emerald-300 dark:peer-focus-visible:ring-emerald-400/25">
					<Icon name="Check" className="size-3.5" />
				</span>
				<span>{t("login.acceptTerms")}</span>
			</label>
			<FormFieldError id="accepted-terms-error" message={error} />
		</>
	);
}

function AuthAlternatives({
	externalLoadingKey,
	isRegister,
	loading,
	onExternalLogin,
	onPasskeyLogin,
	passkeySubmitting,
	passkeySupported,
	showPasskeyLogin,
	visibleProviders,
}: LoginFormCardProps) {
	const { t } = useTranslation();
	if (isRegister || (!showPasskeyLogin && visibleProviders.length === 0)) {
		return null;
	}
	return (
		<div className="grid gap-3">
			<div className="flex items-center gap-3 text-xs text-slate-500 dark:text-white/52">
				<span className="h-px flex-1 bg-black/10 dark:bg-white/10" />
				<span>{t("login.or")}</span>
				<span className="h-px flex-1 bg-black/10 dark:bg-white/10" />
			</div>
			<div className="grid gap-2">
				{showPasskeyLogin ? (
					<Button
						type="button"
						variant="outline"
						className="h-12 rounded-lg border-black/10 bg-white/55 text-[#102118] hover:bg-white/78 disabled:text-slate-400 dark:border-white/14 dark:bg-white/3 dark:text-white dark:hover:bg-white/9 dark:disabled:text-white/38"
						onClick={onPasskeyLogin}
						disabled={passkeySubmitting || loading || !passkeySupported}
					>
						<Icon
							name={passkeySubmitting ? "Spinner" : "Key"}
							className={cn("size-4", passkeySubmitting && "animate-spin")}
						/>
						{t("login.passkeyLogin")}
					</Button>
				) : null}
				{visibleProviders.map((provider) => (
					<Button
						key={provider.key}
						type="button"
						variant="outline"
						className="h-12 rounded-lg border-black/10 bg-white/55 text-[#102118] hover:bg-white/78 dark:border-white/14 dark:bg-white/3 dark:text-white dark:hover:bg-white/9"
						onClick={() => onExternalLogin(provider)}
						disabled={externalLoadingKey !== null}
					>
						{externalLoadingKey === provider.key ? (
							<Icon name="Spinner" className="size-4 animate-spin" />
						) : (
							<ExternalProviderButtonIcon provider={provider} />
						)}
						{t("login.externalLogin", {
							provider: provider.display_name,
						})}
					</Button>
				))}
			</div>
		</div>
	);
}

function AccountModeLink({
	isRegister,
	onResetAccountOptions,
}: {
	isRegister: boolean;
	onResetAccountOptions: () => void;
}) {
	const { t } = useTranslation();
	return (
		<p className="text-center text-sm text-slate-700 dark:text-white/78">
			{isRegister ? t("login.hasAccount") : t("login.noAccount")}{" "}
			<Link
				to={isRegister ? publicPaths.login : publicPaths.register}
				className="font-semibold text-emerald-700 underline-offset-4 hover:text-emerald-600 hover:underline dark:text-emerald-300 dark:hover:text-emerald-200"
				onClick={onResetAccountOptions}
			>
				{isRegister ? t("nav.login") : t("login.registerNow")}
			</Link>
		</p>
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

function ExternalProviderButtonIcon({
	provider,
}: {
	provider: ExternalAuthPublicProvider;
}) {
	const configuredIcon = normalizeExternalAuthIconUrl(provider.icon_url);
	const kindIcon = externalAuthKindIconPath(provider.kind);
	const iconPath = configuredIcon || kindIcon;

	return (
		<img
			src={iconPath}
			alt=""
			aria-hidden="true"
			className="size-4 object-contain"
			onError={(event) => {
				if (
					configuredIcon &&
					event.currentTarget.dataset.fallbackTried !== "1"
				) {
					event.currentTarget.dataset.fallbackTried = "1";
					event.currentTarget.src = kindIcon;
				}
			}}
		/>
	);
}
