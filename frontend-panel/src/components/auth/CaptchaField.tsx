import { useTranslation } from "react-i18next";
import { authInputClassName } from "@/components/auth/AuthFormPrimitives";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { AuthFormFieldError } from "./AuthFormPrimitives";

export function CaptchaField({
	answer,
	disabled = false,
	error,
	imageBase64,
	loading,
	loadError,
	onAnswerChange,
	onRefresh,
}: {
	answer: string;
	disabled?: boolean;
	error?: string;
	imageBase64: string | null;
	loading: boolean;
	loadError: string | null;
	onAnswerChange: (value: string) => void;
	onRefresh: () => void;
}) {
	const { t } = useTranslation();
	return (
		<div className="grid gap-2">
			<Label
				htmlFor="captcha-answer"
				className="text-slate-700 dark:text-white/88"
			>
				{t("login.captcha")}
			</Label>
			<div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_12.75rem] sm:items-center">
				<Input
					id="captcha-answer"
					value={answer}
					onChange={(event) => onAnswerChange(event.currentTarget.value)}
					autoComplete="off"
					disabled={disabled || loading || !imageBase64}
					maxLength={12}
					placeholder={t("login.captchaPlaceholder")}
					className={cn(authInputClassName, "h-14")}
					aria-invalid={Boolean(error || loadError)}
					aria-describedby={
						error
							? "captcha-answer-error"
							: loadError
								? "captcha-load-error"
								: undefined
					}
					required
				/>
				<button
					type="button"
					className="flex h-14 w-full min-w-0 cursor-pointer items-center justify-center overflow-hidden rounded-lg bg-transparent p-0 text-slate-500 transition-shadow outline-none focus-visible:ring-3 focus-visible:ring-emerald-500/18 disabled:cursor-not-allowed disabled:opacity-60 dark:text-white/50 dark:focus-visible:ring-emerald-400/20 sm:w-[12.75rem]"
					onClick={onRefresh}
					disabled={disabled || loading}
					aria-label={t("login.refreshCaptcha")}
					title={t("login.refreshCaptcha")}
				>
					{loading ? (
						<Icon
							name="Spinner"
							className="size-5 animate-spin text-slate-500"
						/>
					) : imageBase64 ? (
						<img
							src={imageBase64}
							alt={t("login.captchaImageAlt")}
							className="h-full max-w-full select-none object-contain"
							draggable={false}
						/>
					) : (
						<span className="text-xs text-slate-500 dark:text-white/50">
							{t("login.captchaUnavailable")}
						</span>
					)}
				</button>
			</div>
			<AuthFormFieldError id="captcha-answer-error" message={error} />
			<AuthFormFieldError
				id="captcha-load-error"
				message={loadError ?? undefined}
			/>
		</div>
	);
}
