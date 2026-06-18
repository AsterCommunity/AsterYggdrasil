import { useState } from "react";
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
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { emailSchema, passwordSchema, usernameSchema } from "@/lib/validation";
import type { CreateAdminUserRequest, UserRole, UserStatus } from "@/types/api";
import { RoleBadge, StatusBadge } from "./UsersTable";

type UserForm = {
	email: string;
	mustChangePassword: boolean;
	password: string;
	role: UserRole;
	status: UserStatus;
	username: string;
};

type UserFormErrors = Partial<
	Record<keyof Pick<UserForm, "email" | "password" | "username">, string>
>;

const emptyForm: UserForm = {
	email: "",
	mustChangePassword: false,
	password: "",
	role: "user",
	status: "active",
	username: "",
};

function validateField(field: keyof UserForm, value: string) {
	if (field === "username") {
		const result = usernameSchema.safeParse(value.trim());
		return result.success ? undefined : (result.error.issues[0]?.message ?? "");
	}
	if (field === "email") {
		const result = emailSchema.safeParse(value.trim());
		return result.success ? undefined : (result.error.issues[0]?.message ?? "");
	}
	if (field === "password") {
		if (!value) return undefined;
		const result = passwordSchema.safeParse(value);
		return result.success ? undefined : (result.error.issues[0]?.message ?? "");
	}
	return undefined;
}

export function UserDialog({
	onOpenChange,
	onSubmit,
	open,
	submitting,
}: {
	onOpenChange: (open: boolean) => void;
	onSubmit: (data: CreateAdminUserRequest) => void;
	open: boolean;
	submitting: boolean;
}) {
	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			{open ? (
				<UserDialogForm
					onOpenChange={onOpenChange}
					onSubmit={onSubmit}
					submitting={submitting}
				/>
			) : null}
		</Dialog>
	);
}

function UserDialogForm({
	onOpenChange,
	onSubmit,
	submitting,
}: {
	onOpenChange: (open: boolean) => void;
	onSubmit: (data: CreateAdminUserRequest) => void;
	submitting: boolean;
}) {
	const { t } = useTranslation();
	const [form, setForm] = useState<UserForm>(emptyForm);
	const [errors, setErrors] = useState<UserFormErrors>({});
	const roleOptions = [
		{ label: t("admin.users.role.user"), value: "user" },
		{ label: t("admin.users.role.admin"), value: "admin" },
	];
	const statusOptions = [
		{ label: t("admin.users.status.active"), value: "active" },
		{ label: t("admin.users.status.disabled"), value: "disabled" },
	];
	const canSubmit =
		usernameSchema.safeParse(form.username.trim()).success &&
		emailSchema.safeParse(form.email.trim()).success &&
		(!form.password || passwordSchema.safeParse(form.password).success);

	function updateTextField(
		field: "email" | "password" | "username",
		value: string,
	) {
		const nextValue = field === "password" ? value.trim() : value;
		setForm((current) => ({
			...current,
			[field]: nextValue,
		}));
		setErrors((current) => ({
			...current,
			[field]: validateField(field, nextValue),
		}));
	}

	function submit() {
		const next = {
			email: form.email.trim(),
			password: form.password,
			username: form.username.trim(),
		};
		const nextErrors: UserFormErrors = {};
		const usernameResult = usernameSchema.safeParse(next.username);
		if (!usernameResult.success) {
			nextErrors.username = usernameResult.error.issues[0]?.message ?? "";
		}
		const emailResult = emailSchema.safeParse(next.email);
		if (!emailResult.success) {
			nextErrors.email = emailResult.error.issues[0]?.message ?? "";
		}
		const passwordResult = passwordSchema.safeParse(next.password);
		if (next.password && !passwordResult.success) {
			nextErrors.password = passwordResult.error.issues[0]?.message ?? "";
		}
		setErrors(nextErrors);
		if (Object.keys(nextErrors).length > 0) return;

		onSubmit({
			email: next.email,
			password: next.password || null,
			must_change_password: form.mustChangePassword,
			role: form.role,
			status: form.status,
			username: next.username,
		});
	}

	return (
		<DialogContent className="sm:max-w-2xl">
			<DialogHeader>
				<DialogTitle>{t("admin.users.create")}</DialogTitle>
				<DialogDescription>
					{t("admin.users.createDescription")}
				</DialogDescription>
			</DialogHeader>
			<form
				className="grid gap-4 md:grid-cols-2"
				onSubmit={(event) => {
					event.preventDefault();
					submit();
				}}
			>
				<Field
					label={t("admin.users.username")}
					htmlFor="admin-create-user-username"
					required
					error={errors.username && t(errors.username)}
				>
					<Input
						id="admin-create-user-username"
						value={form.username}
						minLength={4}
						maxLength={16}
						onChange={(event) =>
							updateTextField("username", event.target.value)
						}
					/>
				</Field>
				<Field
					label={t("admin.users.email")}
					htmlFor="admin-create-user-email"
					required
					error={errors.email && t(errors.email)}
				>
					<Input
						id="admin-create-user-email"
						type="email"
						value={form.email}
						onChange={(event) => updateTextField("email", event.target.value)}
					/>
				</Field>
				<Field
					label={t("admin.users.password")}
					htmlFor="admin-create-user-password"
					description={t("admin.users.passwordCreateHint")}
					error={errors.password && t(errors.password)}
				>
					<Input
						id="admin-create-user-password"
						type="password"
						value={form.password}
						minLength={8}
						maxLength={128}
						placeholder={t("admin.users.passwordCreatePlaceholder")}
						onChange={(event) =>
							updateTextField("password", event.target.value)
						}
					/>
				</Field>
				<Field label={t("admin.users.roleLabel")}>
					<Select
						items={roleOptions}
						value={form.role}
						onValueChange={(value) =>
							setForm((current) => ({
								...current,
								role: value as UserRole,
							}))
						}
					>
						<SelectTrigger>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="user">
								<span className="flex items-center gap-2">
									<RoleBadge userRole="user" />
								</span>
							</SelectItem>
							<SelectItem value="admin">
								<span className="flex items-center gap-2">
									<RoleBadge userRole="admin" />
								</span>
							</SelectItem>
						</SelectContent>
					</Select>
				</Field>
				<Field label={t("admin.users.statusLabel")}>
					<Select
						items={statusOptions}
						value={form.status}
						onValueChange={(value) =>
							setForm((current) => ({
								...current,
								status: value as UserStatus,
							}))
						}
					>
						<SelectTrigger>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="active">
								<span className="flex items-center gap-2">
									<StatusBadge status="active" />
								</span>
							</SelectItem>
							<SelectItem value="disabled">
								<span className="flex items-center gap-2">
									<StatusBadge status="disabled" />
								</span>
							</SelectItem>
						</SelectContent>
					</Select>
				</Field>
				<div className="rounded-lg border border-border/70 bg-background/70 p-4 md:col-span-2 dark:border-white/10 dark:bg-input/10">
					<div className="flex items-start justify-between gap-4">
						<div className="space-y-1">
							<Label htmlFor="admin-create-user-must-change-password">
								{t("admin.users.forcePasswordChange")}
							</Label>
							<p className="text-muted-foreground text-sm leading-5">
								{t("admin.users.createForcePasswordChangeDescription")}
							</p>
						</div>
						<Switch
							id="admin-create-user-must-change-password"
							checked={form.mustChangePassword}
							onCheckedChange={(value) =>
								setForm((current) => ({
									...current,
									mustChangePassword: value,
								}))
							}
							aria-label={t("admin.users.forcePasswordChange")}
						/>
					</div>
				</div>
				<DialogFooter className="md:col-span-2">
					<Button
						type="button"
						variant="outline"
						disabled={submitting}
						onClick={() => onOpenChange(false)}
					>
						{t("common.cancel")}
					</Button>
					<Button type="submit" disabled={submitting || !canSubmit}>
						{submitting ? (
							<Icon name="Spinner" className="mr-2 size-4 animate-spin" />
						) : (
							<Icon name="FloppyDisk" className="mr-2 size-4" />
						)}
						{t("common.create")}
					</Button>
				</DialogFooter>
			</form>
		</DialogContent>
	);
}

function Field({
	children,
	description,
	error,
	htmlFor,
	label,
	required,
}: {
	children: React.ReactNode;
	description?: string;
	error?: string;
	htmlFor?: string;
	label: string;
	required?: boolean;
}) {
	return (
		<div className="space-y-2">
			<Label htmlFor={htmlFor}>
				{label}
				{required ? <span className="text-destructive"> *</span> : null}
			</Label>
			{children}
			{description ? (
				<p className="text-xs leading-5 text-muted-foreground">{description}</p>
			) : null}
			{error ? <p className="text-destructive text-sm">{error}</p> : null}
		</div>
	);
}
