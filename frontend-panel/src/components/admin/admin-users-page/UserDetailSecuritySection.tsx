import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { UserDetailField } from "./UserDetailField";

export function UserDetailSecuritySection({
	activeSessionCount,
	confirmPassword,
	confirmPasswordError,
	mustChangePassword,
	password,
	passwordError,
	revokingSessions,
	savingForcePasswordChange,
	savingPassword,
	onConfirmPasswordChange,
	onForcePasswordChangeToggle,
	onPasswordChange,
	onPasswordReset,
	onSessionRevoke,
}: {
	activeSessionCount: number;
	confirmPassword: string;
	confirmPasswordError?: string;
	mustChangePassword: boolean;
	password: string;
	passwordError?: string;
	revokingSessions: boolean;
	savingForcePasswordChange: boolean;
	savingPassword: boolean;
	onConfirmPasswordChange: (value: string) => void;
	onForcePasswordChangeToggle: (value: boolean) => void;
	onPasswordChange: (value: string) => void;
	onPasswordReset: () => void;
	onSessionRevoke: () => void;
}) {
	const { t } = useTranslation();
	return (
		<section className="rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
			<div className="mb-4">
				<h3 className="font-medium text-foreground">
					{t("admin.users.securitySection")}
				</h3>
				<p className="mt-1 text-sm text-muted-foreground">
					{t("admin.users.securitySectionDescription")}
				</p>
			</div>
			<div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto] md:items-start">
				<UserDetailField
					label={t("admin.users.newPassword")}
					description={t("admin.users.passwordCreateHint")}
					error={passwordError}
				>
					<Input
						id="admin-user-detail-new-password"
						type="password"
						value={password}
						disabled={savingPassword}
						onChange={(event) => onPasswordChange(event.target.value)}
					/>
				</UserDetailField>
				<UserDetailField
					label={t("admin.users.confirmPassword")}
					error={confirmPasswordError}
				>
					<Input
						id="admin-user-detail-confirm-password"
						type="password"
						value={confirmPassword}
						disabled={savingPassword}
						onChange={(event) => onConfirmPasswordChange(event.target.value)}
					/>
				</UserDetailField>
				<div className="space-y-2 md:min-w-36">
					<Label className="invisible max-md:hidden">
						{t("admin.users.resetPassword")}
					</Label>
					<Button
						type="button"
						variant="outline"
						className="w-full"
						disabled={savingPassword || !password || !confirmPassword}
						onClick={onPasswordReset}
					>
						<Icon
							name={savingPassword ? "Spinner" : "Key"}
							className={cn("mr-2 size-4", savingPassword && "animate-spin")}
						/>
						{t("admin.users.resetPassword")}
					</Button>
				</div>
			</div>

			<div className="mt-5 rounded-lg border border-border/70 bg-background/70 p-4 dark:border-white/10 dark:bg-input/10">
				<div className="flex items-start justify-between gap-4">
					<div className="space-y-1">
						<Label htmlFor="admin-user-force-password-change">
							{t("admin.users.forcePasswordChange")}
						</Label>
						<p className="text-muted-foreground text-sm leading-5">
							{t("admin.users.forcePasswordChangeDescription")}
						</p>
						<p className="font-medium text-muted-foreground text-xs">
							{mustChangePassword
								? t("admin.users.forcePasswordChangeEnabled")
								: t("admin.users.forcePasswordChangeDisabled")}
						</p>
					</div>
					<div className="flex items-center gap-2">
						{savingForcePasswordChange ? (
							<Icon
								name="Spinner"
								className="size-4 animate-spin text-muted-foreground"
							/>
						) : null}
						<Switch
							id="admin-user-force-password-change"
							checked={mustChangePassword}
							disabled={savingForcePasswordChange}
							aria-label={t("admin.users.forcePasswordChange")}
							onCheckedChange={onForcePasswordChangeToggle}
						/>
					</div>
				</div>
			</div>

			<div className="mt-5 rounded-lg border border-destructive/30 bg-destructive/10 p-4">
				<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
					<div>
						<p className="font-medium text-destructive text-sm">
							{t("admin.users.revokeSessions")}
						</p>
						<p className="mt-1 text-destructive/80 text-sm">
							{t("admin.users.revokeSessionsDescription")}
						</p>
					</div>
					<Button
						type="button"
						variant="destructive"
						disabled={revokingSessions || activeSessionCount === 0}
						onClick={onSessionRevoke}
					>
						<Icon
							name={revokingSessions ? "Spinner" : "SignOut"}
							className={cn("mr-2 size-4", revokingSessions && "animate-spin")}
						/>
						{t("admin.users.revokeSessions")}
					</Button>
				</div>
			</div>
		</section>
	);
}
