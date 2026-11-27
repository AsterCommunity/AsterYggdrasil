import { type FormEvent, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { RefreshButton } from "@/components/common/RefreshButton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { handleApiError } from "@/hooks/useApiError";
import { cn } from "@/lib/utils";
import { emailSchema } from "@/lib/validation";
import { DetailRow } from "@/pages/account/account-settings-page/settingsSection";
import { authService } from "@/services/authService";
import { useAuthStore } from "@/stores/authStore";
import type { AuthUserInfo } from "@/types/api";

export function EmailChangeSection({ user }: { user: AuthUserInfo }) {
	const { t } = useTranslation();
	const refreshUser = useAuthStore((state) => state.refreshUser);
	const [newEmail, setNewEmail] = useState(user.pending_email ?? "");
	const [error, setError] = useState<string | null>(null);
	const [submitting, setSubmitting] = useState(false);
	const [resending, setResending] = useState(false);
	const pendingEmail = user.pending_email?.trim() || null;
	const normalizedCurrent = user.email.trim().toLowerCase();
	const normalizedPending = pendingEmail?.toLowerCase() ?? null;
	const normalizedNext = newEmail.trim().toLowerCase();
	const validation = emailSchema.safeParse(newEmail);
	const canSubmit =
		validation.success &&
		normalizedNext.length > 0 &&
		normalizedNext !== normalizedCurrent &&
		normalizedNext !== normalizedPending;

	async function submit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		const result = emailSchema.safeParse(newEmail);
		if (!result.success) {
			setError(result.error.issues[0]?.message ?? "");
			return;
		}

		setError(null);
		setSubmitting(true);
		try {
			await authService.requestEmailChange({ new_email: result.data });
			await refreshUser();
			toast.success(t("personalSettings.emailChangeRequested"));
		} catch (nextError) {
			handleApiError(nextError);
		} finally {
			setSubmitting(false);
		}
	}

	async function resend() {
		setResending(true);
		try {
			await authService.resendEmailChange();
			toast.success(t("personalSettings.emailChangeResent"));
		} catch (nextError) {
			handleApiError(nextError);
		} finally {
			setResending(false);
		}
	}

	return (
		<div className="rounded-lg border border-border/70 bg-background/55 dark:border-white/10 dark:bg-input/10">
			<div className="flex flex-col gap-3 border-b border-border/70 px-4 py-4 sm:flex-row sm:items-start sm:justify-between dark:border-white/10">
				<div className="min-w-0">
					<h3 className="text-sm font-semibold">
						{t("personalSettings.emailSecurityTitle")}
					</h3>
					<p className="mt-1 text-xs leading-5 text-muted-foreground">
						{t("personalSettings.emailSecurityDescription")}
					</p>
				</div>
				<Badge variant="outline" className="w-fit rounded-md">
					{user.email_verified
						? t("personalSettings.emailVerified")
						: t("personalSettings.emailUnverified")}
				</Badge>
			</div>
			<div className="grid gap-4 px-4 py-4">
				<div className="grid gap-3 @min-[44rem]/settings:grid-cols-2">
					<DetailRow
						label={t("personalSettings.currentEmail")}
						value={<span className="block min-w-0 truncate">{user.email}</span>}
					/>
					<DetailRow
						label={t("personalSettings.pendingEmail")}
						value={
							pendingEmail ? (
								<span className="block min-w-0 truncate">{pendingEmail}</span>
							) : (
								<span className="text-muted-foreground">
									{t("personalSettings.emptyValue")}
								</span>
							)
						}
					/>
				</div>
				<form className="grid gap-3" onSubmit={submit}>
					<div className="grid gap-2">
						<label
							htmlFor="new-email"
							className="text-xs font-medium text-muted-foreground"
						>
							{t("personalSettings.newEmail")}
						</label>
						<div className="flex flex-col gap-2 sm:flex-row">
							<Input
								id="new-email"
								type="email"
								value={newEmail}
								onChange={(event) => {
									const value = event.currentTarget.value;
									setNewEmail(value);
									if (error) {
										const next = emailSchema.safeParse(value);
										setError(
											next.success
												? null
												: (next.error.issues[0]?.message ?? ""),
										);
									}
								}}
								autoComplete="email"
								placeholder={t("personalSettings.newEmailPlaceholder")}
								aria-invalid={Boolean(error)}
								aria-describedby={error ? "new-email-error" : undefined}
							/>
							<Button type="submit" disabled={submitting || !canSubmit}>
								<Icon
									name={submitting ? "Spinner" : "EnvelopeSimple"}
									className={cn("mr-2 size-4", submitting && "animate-spin")}
								/>
								{t("personalSettings.emailChangeSubmit")}
							</Button>
						</div>
						{error ? (
							<p
								id="new-email-error"
								className="text-xs leading-5 text-destructive"
							>
								{t(error)}
							</p>
						) : null}
					</div>
				</form>
				{pendingEmail ? (
					<div className="flex flex-col gap-2 rounded-md border border-border/60 bg-muted/18 p-3 sm:flex-row sm:items-center sm:justify-between dark:border-white/10 dark:bg-muted/10">
						<div className="text-sm leading-6 text-muted-foreground">
							{t("personalSettings.emailChangePendingHint", {
								email: pendingEmail,
							})}
						</div>
						<RefreshButton loading={resending} onClick={() => void resend()}>
							{t("personalSettings.emailChangeResend")}
						</RefreshButton>
					</div>
				) : null}
			</div>
		</div>
	);
}
