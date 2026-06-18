import type { FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Icon } from "@/components/ui/icon";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ADMIN_CONTROL_HEIGHT_CLASS } from "@/lib/constants";
import type {
	AdminUserInvitationInfo,
	CreateUserInvitationRequest,
} from "@/types/api";

interface InviteUserDialogProps {
	createdInvitation: AdminUserInvitationInfo | null;
	error: string | null;
	form: CreateUserInvitationRequest;
	inviting: boolean;
	open: boolean;
	onCopyLink: (value: string) => void;
	onFieldChange: (value: string) => void;
	onOpenChange: (open: boolean) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
}

export function InviteUserDialog({
	createdInvitation,
	error,
	form,
	inviting,
	open,
	onCopyLink,
	onFieldChange,
	onOpenChange,
	onSubmit,
}: InviteUserDialogProps) {
	const { t } = useTranslation();
	const invitationUrl = createdInvitation?.invitation_url?.trim() ?? "";
	const emailErrorId = "invite-user-email-error";

	function handleOpenChange(nextOpen: boolean) {
		if (inviting && !nextOpen) return;
		onOpenChange(nextOpen);
	}

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent keepMounted className="sm:max-w-md">
				<form onSubmit={onSubmit} autoComplete="off" className="space-y-4">
					<DialogHeader>
						<DialogTitle>{t("admin.users.inviteUser")}</DialogTitle>
						<DialogDescription>
							{t("admin.users.inviteUserDescription")}
						</DialogDescription>
					</DialogHeader>
					<div className="space-y-2">
						<Label htmlFor="invite-user-email">
							{t("admin.users.inviteEmail")}
						</Label>
						<Input
							id="invite-user-email"
							name="admin-invite-user-email"
							type="email"
							value={form.email}
							onChange={(event) => onFieldChange(event.target.value)}
							autoComplete="off"
							required
							className={ADMIN_CONTROL_HEIGHT_CLASS}
							aria-invalid={Boolean(error)}
							aria-describedby={error ? emailErrorId : undefined}
						/>
						{error ? (
							<p id={emailErrorId} className="text-xs text-destructive">
								{error}
							</p>
						) : null}
					</div>
					{createdInvitation ? (
						<div className="space-y-2 rounded-lg border border-border/70 bg-muted/25 p-3">
							<div className="flex items-center justify-between gap-3">
								<p className="text-sm font-medium">
									{t("admin.users.invitationCreated")}
								</p>
								{createdInvitation.mail_queued ? (
									<span className="text-xs text-muted-foreground">
										{t("admin.users.invitationMailQueued")}
									</span>
								) : null}
							</div>
							{invitationUrl ? (
								<div className="flex min-w-0 items-center gap-2">
									<Input
										readOnly
										value={invitationUrl}
										className="h-9 min-w-0 font-mono text-xs"
										onFocus={(event) => event.target.select()}
									/>
									<Button
										type="button"
										variant="outline"
										size="icon"
										className="size-9 shrink-0"
										onClick={() => onCopyLink(invitationUrl)}
										aria-label={t("admin.users.copyInvitationLink")}
										title={t("admin.users.copyInvitationLink")}
									>
										<Icon name="Copy" className="size-4" />
									</Button>
								</div>
							) : null}
						</div>
					) : null}
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							onClick={() => handleOpenChange(false)}
							disabled={inviting}
						>
							{t("common.cancel")}
						</Button>
						<Button type="submit" disabled={inviting}>
							{inviting ? (
								<Icon name="Spinner" className="mr-2 size-4 animate-spin" />
							) : (
								<Icon name="EnvelopeSimple" className="mr-2 size-4" />
							)}
							{t("admin.users.sendInvitation")}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}
