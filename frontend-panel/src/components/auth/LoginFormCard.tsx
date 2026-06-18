import type { FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import {
	AuthFormCard,
	AuthFormFieldError,
	AuthIconTextField,
	AuthPasswordField,
	authPrimaryButtonClassName,
	authSecondaryButtonClassName,
} from "@/components/auth/AuthFormPrimitives";
import { CaptchaField } from "@/components/auth/CaptchaField";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import {
	externalAuthKindIconPath,
	normalizeExternalAuthIconUrl,
} from "@/lib/externalAuthProviders";
import { cn } from "@/lib/utils";
import { publicPaths } from "@/routes/routePaths";
import type { ExternalAuthPublicProvider } from "@/types/api";
import { PasswordStrengthMeter } from "./PasswordStrengthMeter";

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
	captchaAnswer: string;
	captchaImageBase64: string | null;
	captchaLoadError: string | null;
	captchaLoading: boolean;
	captchaRequired: boolean;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onCaptchaAnswerChange: (value: string) => void;
	onCaptchaRefresh: () => void;
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
		<AuthFormCard
			key={isRegister ? "register" : "login"}
			title={cardTitle}
			description={cardDescription}
		>
			<form className="mt-7 grid gap-4" onSubmit={onSubmit} noValidate>
				{usesAccountCreationForm ? (
					<AccountCreationFields {...props} />
				) : (
					<IdentifierField {...props} />
				)}
				<LoginPasswordField {...props} />
				{usesAccountCreationForm ? <RegistrationFields {...props} /> : null}
				<CaptchaPanel {...props} />
				<Button
					type="submit"
					disabled={submitDisabled}
					className={authPrimaryButtonClassName}
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
		</AuthFormCard>
	);
}

function CaptchaPanel({
	captchaAnswer,
	captchaImageBase64,
	captchaLoadError,
	captchaLoading,
	captchaRequired,
	loading,
	onCaptchaAnswerChange,
	onCaptchaRefresh,
}: LoginFormCardProps) {
	if (!captchaRequired) return null;
	return (
		<CaptchaField
			answer={captchaAnswer}
			disabled={loading}
			imageBase64={captchaImageBase64}
			loading={captchaLoading}
			loadError={captchaLoadError}
			onAnswerChange={onCaptchaAnswerChange}
			onRefresh={onCaptchaRefresh}
		/>
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
			<AuthIconTextField
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
			<AuthIconTextField
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
		<AuthIconTextField
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
		<AuthPasswordField
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
			<AuthPasswordField
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
			<AuthFormFieldError id="accepted-terms-error" message={error} />
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
						className={authSecondaryButtonClassName}
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
						className={authSecondaryButtonClassName}
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
