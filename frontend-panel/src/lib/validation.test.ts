import { describe, expect, it } from "vitest";
import {
	passwordChangeSchema,
	passwordSchema,
	usernameSchema,
} from "./validation";

describe("validation schemas", () => {
	it("accepts username boundary lengths and allowed characters", () => {
		expect(usernameSchema.safeParse("abcd").success).toBe(true);
		expect(usernameSchema.safeParse("a".repeat(16)).success).toBe(true);
		expect(usernameSchema.safeParse("user-name_1").success).toBe(true);
	});

	it("rejects username length and character boundaries", () => {
		expect(usernameSchema.safeParse("abc").success).toBe(false);
		expect(usernameSchema.safeParse("a".repeat(17)).success).toBe(false);
		expect(usernameSchema.safeParse("user.name").success).toBe(false);
		expect(usernameSchema.safeParse("用户名").success).toBe(false);
		expect(usernameSchema.safeParse("user name").success).toBe(false);
	});

	it("accepts password boundary lengths", () => {
		expect(passwordSchema.safeParse("a".repeat(8)).success).toBe(true);
		expect(passwordSchema.safeParse("a".repeat(128)).success).toBe(true);
	});

	it("rejects password length boundaries", () => {
		expect(passwordSchema.safeParse("a".repeat(7)).success).toBe(false);
		expect(passwordSchema.safeParse("a".repeat(129)).success).toBe(false);
	});

	it("validates password change boundaries", () => {
		expect(
			passwordChangeSchema.safeParse({
				currentPassword: "current-password",
				newPassword: "new-password",
				confirmPassword: "new-password",
			}).success,
		).toBe(true);
		expect(
			passwordChangeSchema.safeParse({
				currentPassword: "same-password",
				newPassword: "same-password",
				confirmPassword: "same-password",
			}).success,
		).toBe(false);
		expect(
			passwordChangeSchema.safeParse({
				currentPassword: "current-password",
				newPassword: "new-password",
				confirmPassword: "different-password",
			}).success,
		).toBe(false);
		expect(
			passwordChangeSchema.safeParse({
				currentPassword: "",
				newPassword: "a".repeat(7),
				confirmPassword: "",
			}).success,
		).toBe(false);
	});
});
