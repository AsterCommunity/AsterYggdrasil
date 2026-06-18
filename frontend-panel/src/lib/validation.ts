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
