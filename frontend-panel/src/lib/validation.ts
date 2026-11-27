import { z } from "zod/v4";

export const usernameSchema = z
	.string()
	.trim()
	.min(4, "login.validationUsernameLength")
	.max(16, "login.validationUsernameLength")
	.regex(/^[A-Za-z0-9_-]+$/, "login.validationUsernameChars");

export const emailSchema = z
	.string()
	.trim()
	.min(1, "login.validationEmailRequired")
	.email("login.validationEmailInvalid");

export const existingPasswordSchema = z
	.string()
	.min(1, "login.validationPasswordRequired");

export const passwordSchema = z
	.string()
	.min(8, "login.validationPasswordLength")
	.max(128, "login.validationPasswordLength");

export const confirmPasswordRequiredSchema = z
	.string()
	.min(1, "login.validationConfirmRequired");

export const passwordChangeMatchSchema = z
	.object({
		currentPassword: existingPasswordSchema,
		newPassword: passwordSchema,
		confirmPassword: confirmPasswordRequiredSchema,
	})
	.refine((value) => value.newPassword !== value.currentPassword, {
		path: ["newPassword"],
		message: "login.validationPasswordSameAsCurrent",
	})
	.refine((value) => value.newPassword === value.confirmPassword, {
		path: ["confirmPassword"],
		message: "login.passwordMismatch",
	});

export const passwordChangeSchema = passwordChangeMatchSchema;

/**
 * Admin invite dialog (AdminUsersPage / AdminUserInvitationsPage): the
 * invitation email reuses the shared `emailSchema`, which is the single
 * email validator in the app (issue #58 removed the per-page copies).
 */
export const inviteUserSchema = z.object({ email: emailSchema });

/**
 * Admin user-ban dialog (UserDetailBanSection): reason is required after
 * trimming and at least one capability scope must be selected. Field order
 * matches the dialog's manual check order (reason first, then scopes), so
 * `issues[0]` keeps reporting the same first error as before.
 */
export const banFormSchema = z.object({
	reason: z.string().trim().min(1, "admin.users.bans.reasonRequired"),
	scopes: z.array(z.string()).min(1, "admin.users.bans.scopeRequired"),
});

/**
 * System-config numeric draft (AdminSettingsPage). The unit-aware input
 * submits in the selected time unit and a cleared draft submits an empty
 * string on purpose, so only non-empty text must parse as a finite number.
 * This keeps the previous hand-written `Number.isFinite(Number(text))`
 * judgment (empty/whitespace passes, "12abc" fails) behind a schema.
 */
export const settingsNumberDraftSchema = z
	.string()
	.refine((value) => value.trim() === "" || Number.isFinite(Number(value)), {
		message: "settings_invalid_number",
	});
