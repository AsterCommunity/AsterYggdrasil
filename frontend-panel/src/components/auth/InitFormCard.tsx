import type { FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { PasswordStrengthMeter } from "./PasswordStrengthMeter";

const initInputClassName =
	"h-12 rounded-lg border-black/10 bg-white/70 text-[#102118] shadow-[inset_0_1px_0_rgba(255,255,255,0.72)] [caret-color:#102118] [-webkit-text-fill-color:#102118] placeholder:text-slate-500 focus-visible:border-emerald-700/32 focus-visible:bg-white/82 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:border-white/14 dark:bg-neutral-950/42 dark:text-white dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] dark:[caret-color:white] dark:[-webkit-text-fill-color:white] dark:placeholder:text-white/42 dark:focus-visible:border-emerald-300/45 dark:focus-visible:bg-neutral-950/52 dark:focus-visible:ring-emerald-400/20 [&:-webkit-autofill]:border-black/10 [&:-webkit-autofill]:shadow-[0_0_0_1000px_rgba(255,255,255,0.92)_inset] [&:-webkit-autofill]:[-webkit-text-fill-color:#102118] dark:[&:-webkit-autofill]:border-white/14 dark:[&:-webkit-autofill]:shadow-[0_0_0_1000px_rgba(20,28,25,0.98)_inset] dark:[&:-webkit-autofill]:[-webkit-text-fill-color:white] dark:[&:-webkit-autofill:focus]:shadow-[0_0_0_1000px_rgba(20,28,25,0.98)_inset]";

export type PublicUrlStatus =
	| { valid: true; normalized: string; insecure: boolean }
	| { valid: false; messageKey: string };

export type InitFormField =
	| "username"
	| "email"
	| "password"
	| "confirmPassword";

export type InitFormErrors = Partial<Record<InitFormField, string>>;

export function InitFormCard({
	username,
	email,
	password,
	confirmPassword,
	publicSiteUrl,
	errors,
	showPassword,
	loading,
	submitDisabled,
	passwordScore,
	passwordStrengthLabel,
	publicUrlStatus,
	onSubmit,
	onUsernameChange,
	onEmailChange,
	onPasswordChange,
	onConfirmPasswordChange,
	onPublicSiteUrlChange,
	onToggleShowPassword,
}: {
	username: string;
	email: string;
	password: string;
	confirmPassword: string;
	publicSiteUrl: string;
	errors: InitFormErrors;
	showPassword: boolean;
	loading: boolean;
	submitDisabled: boolean;
	passwordScore: number;
	passwordStrengthLabel: string;
	publicUrlStatus: PublicUrlStatus;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onUsernameChange: (value: string) => void;
	onEmailChange: (value: string) => void;
	onPasswordChange: (value: string) => void;
	onConfirmPasswordChange: (value: string) => void;
	onPublicSiteUrlChange: (value: string) => void;
	onToggleShowPassword: () => void;
}) {
	const { t } = useTranslation();
	return (
		<section className="auth-card-transition relative mx-auto w-full max-w-[520px] rounded-[1.35rem] border border-black/10 bg-white/78 p-6 shadow-[0_24px_90px_rgba(15,35,25,0.18),0_0_0_1px_rgba(255,255,255,0.52),0_0_58px_rgba(22,163,74,0.10)] backdrop-blur-2xl before:pointer-events-none before:absolute before:inset-0 before:rounded-[1.35rem] before:border before:border-emerald-700/8 dark:border-white/11 dark:bg-neutral-950/70 dark:shadow-[0_24px_90px_rgba(0,0,0,0.42),0_0_0_1px_rgba(120,255,190,0.04),0_0_58px_rgba(82,255,170,0.18)] dark:before:border-emerald-300/9 sm:p-9">
			<div>
				<h1 className="text-3xl font-semibold tracking-normal text-[#102118] sm:text-4xl dark:text-white">
					{t("init.title")}
				</h1>
				<p className="mt-2 text-sm leading-6 text-slate-600 dark:text-white/72">
					{t("init.cardDescription")}
				</p>
			</div>
			<form className="mt-7 grid gap-4" onSubmit={onSubmit} noValidate>
				<div className="grid gap-2">
					<Label
						htmlFor="username"
						className="text-slate-700 dark:text-white/88"
					>
						{t("login.username")}
					</Label>
					<div className="relative">
						<Icon
							name="User"
							className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
						/>
						<Input
							id="username"
							value={username}
							onChange={(event) => onUsernameChange(event.currentTarget.value)}
							autoComplete="username"
							minLength={4}
							maxLength={16}
							placeholder={t("login.usernamePlaceholder")}
							className={cn(initInputClassName, "pr-4 pl-11")}
							aria-invalid={Boolean(errors.username)}
							aria-describedby={
								errors.username ? "init-username-error" : undefined
							}
							required
						/>
					</div>
					<FormFieldError
						id="init-username-error"
						message={errors.username && t(errors.username)}
					/>
				</div>
				<div className="grid gap-2">
					<Label htmlFor="email" className="text-slate-700 dark:text-white/88">
						{t("login.email")}
					</Label>
					<div className="relative">
						<Icon
							name="EnvelopeSimple"
							className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
						/>
						<Input
							id="email"
							type="email"
							value={email}
							onChange={(event) => onEmailChange(event.currentTarget.value)}
							autoComplete="email"
							placeholder={t("login.emailPlaceholder")}
							className={cn(initInputClassName, "pr-4 pl-11")}
							aria-invalid={Boolean(errors.email)}
							aria-describedby={errors.email ? "init-email-error" : undefined}
							required
						/>
					</div>
					<FormFieldError
						id="init-email-error"
						message={errors.email && t(errors.email)}
					/>
				</div>
				<div className="grid gap-2">
					<Label
						htmlFor="password"
						className="text-slate-700 dark:text-white/88"
					>
						{t("login.password")}
					</Label>
					<div className="relative">
						<Icon
							name="Lock"
							className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
						/>
						<Input
							id="password"
							type={showPassword ? "text" : "password"}
							value={password}
							onChange={(event) => onPasswordChange(event.currentTarget.value)}
							autoComplete="new-password"
							placeholder={t("login.passwordPlaceholder")}
							maxLength={128}
							className={cn(initInputClassName, "pr-11 pl-11")}
							aria-invalid={Boolean(errors.password)}
							aria-describedby={
								errors.password ? "init-password-error" : undefined
							}
							required
						/>
						<button
							type="button"
							className="absolute top-1/2 right-3 flex size-6 -translate-y-1/2 items-center justify-center rounded-md bg-transparent text-slate-500 transition-colors outline-none hover:text-slate-800 focus-visible:ring-3 focus-visible:ring-emerald-500/18 dark:text-white/62 dark:hover:text-white dark:focus-visible:ring-emerald-400/20"
							onClick={onToggleShowPassword}
							aria-label={
								showPassword ? t("login.hidePassword") : t("login.showPassword")
							}
						>
							<Icon
								name={showPassword ? "EyeSlash" : "Eye"}
								className="size-4"
							/>
						</button>
					</div>
					<FormFieldError
						id="init-password-error"
						message={errors.password && t(errors.password)}
					/>
				</div>
				<div className="grid gap-2">
					<Label
						htmlFor="confirm-password"
						className="text-slate-700 dark:text-white/88"
					>
						{t("login.confirmPassword")}
					</Label>
					<div className="relative">
						<Icon
							name="Lock"
							className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
						/>
						<Input
							id="confirm-password"
							type={showPassword ? "text" : "password"}
							value={confirmPassword}
							onChange={(event) =>
								onConfirmPasswordChange(event.currentTarget.value)
							}
							autoComplete="new-password"
							placeholder={t("login.confirmPasswordPlaceholder")}
							maxLength={128}
							className={cn(initInputClassName, "pr-11 pl-11")}
							aria-invalid={Boolean(errors.confirmPassword)}
							aria-describedby={
								errors.confirmPassword
									? "init-confirm-password-error"
									: undefined
							}
							required
						/>
					</div>
					<FormFieldError
						id="init-confirm-password-error"
						message={errors.confirmPassword && t(errors.confirmPassword)}
					/>
				</div>
				<PasswordStrengthMeter
					label={t("login.passwordStrength")}
					value={passwordStrengthLabel}
					score={passwordScore}
				/>
				<div className="grid gap-2">
					<Label
						htmlFor="public-site-url"
						className="text-slate-700 dark:text-white/88"
					>
						{t("init.publicSiteUrl")}
					</Label>
					<div className="relative">
						<Icon
							name="Globe"
							className="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-slate-500 dark:text-white/46"
						/>
						<Input
							id="public-site-url"
							type="url"
							value={publicSiteUrl}
							onChange={(event) =>
								onPublicSiteUrlChange(event.currentTarget.value)
							}
							autoComplete="url"
							placeholder="https://skin.example.com"
							className={cn(initInputClassName, "pr-4 pl-11")}
							required
						/>
					</div>
					<p className="text-xs leading-5 text-slate-600 dark:text-white/66">
						{t("init.publicSiteUrlHelp")}
					</p>
					{publicUrlStatus.valid && publicUrlStatus.insecure ? (
						<p className="flex items-start gap-2 rounded-lg border border-amber-500/24 bg-amber-400/12 px-3 py-2 text-xs leading-5 text-amber-900 dark:border-amber-300/24 dark:bg-amber-300/10 dark:text-amber-100">
							<Icon name="Warning" className="mt-0.5 size-4 shrink-0" />
							<span>{t("init.publicSiteUrlHttpWarning")}</span>
						</p>
					) : null}
					{!publicUrlStatus.valid ? (
						<p className="flex items-start gap-2 rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-xs leading-5 text-red-800 dark:border-red-300/20 dark:bg-red-400/10 dark:text-red-100">
							<Icon name="CircleAlert" className="mt-0.5 size-4 shrink-0" />
							<span>{t(publicUrlStatus.messageKey)}</span>
						</p>
					) : null}
				</div>
				<Button
					type="submit"
					disabled={submitDisabled}
					className="h-13 rounded-lg border-0 bg-emerald-500 text-base font-semibold text-white shadow-lg shadow-emerald-950/25 hover:bg-emerald-400 disabled:bg-emerald-500/55 disabled:text-white/58"
				>
					<Icon
						name={loading ? "Spinner" : "User"}
						className={loading ? "size-4 animate-spin" : "size-4"}
					/>
					{t("init.createAdmin")}
				</Button>
			</form>
		</section>
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
