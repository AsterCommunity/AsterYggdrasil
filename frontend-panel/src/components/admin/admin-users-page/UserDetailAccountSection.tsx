import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import type { OperatorScope, UserRole, UserStatus } from "@/types/api";
import {
	AdminScopePolicyNote,
	OperatorScopeSelector,
} from "./OperatorScopeSelector";
import { UserDetailField } from "./UserDetailField";
import { RoleBadge, StatusBadge } from "./UsersTable";

export function UserDetailAccountSection({
	email,
	operatorScopes,
	role,
	roleStatusLocked,
	savingProfile,
	status,
	username,
	onEmailChange,
	onOperatorScopesChange,
	onRoleChange,
	onStatusChange,
	onUsernameChange,
}: {
	email: string;
	operatorScopes: OperatorScope[];
	role: UserRole;
	roleStatusLocked: boolean;
	savingProfile: boolean;
	status: UserStatus;
	username: string;
	onEmailChange: (value: string) => void;
	onOperatorScopesChange: (value: OperatorScope[]) => void;
	onRoleChange: (value: UserRole) => void;
	onStatusChange: (value: UserStatus) => void;
	onUsernameChange: (value: string) => void;
}) {
	const { t } = useTranslation();
	return (
		<section className="rounded-lg border border-border/70 bg-background/55 p-4 dark:border-white/10 dark:bg-input/10">
			<div className="mb-4">
				<h3 className="font-medium text-foreground">
					{t("admin.users.accountSection")}
				</h3>
				<p className="mt-1 text-sm text-muted-foreground">
					{t("admin.users.accountSectionDescription")}
				</p>
			</div>
			<div className="grid gap-4 md:grid-cols-2">
				<UserDetailField label={t("admin.users.username")} required>
					<Input
						id="admin-user-detail-username"
						value={username}
						disabled={savingProfile}
						onChange={(event) => onUsernameChange(event.target.value)}
					/>
				</UserDetailField>
				<UserDetailField label={t("admin.users.email")} required>
					<Input
						id="admin-user-detail-email"
						type="email"
						value={email}
						disabled={savingProfile}
						onChange={(event) => onEmailChange(event.target.value)}
					/>
				</UserDetailField>
				<UserDetailField label={t("admin.users.roleLabel")}>
					<Select
						value={role}
						onValueChange={(value) => onRoleChange(value as UserRole)}
					>
						<SelectTrigger
							aria-label={t("admin.users.roleLabel")}
							disabled={savingProfile || roleStatusLocked}
						>
							<SelectValue>
								{(value: UserRole | null) => (
									<RoleBadge userRole={value ?? "user"} />
								)}
							</SelectValue>
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="user">
								<RoleBadge userRole="user" />
							</SelectItem>
							<SelectItem value="operator">
								<RoleBadge userRole="operator" />
							</SelectItem>
							<SelectItem value="admin">
								<RoleBadge userRole="admin" />
							</SelectItem>
						</SelectContent>
					</Select>
				</UserDetailField>
				<UserDetailField label={t("admin.users.statusLabel")}>
					<Select
						value={status}
						onValueChange={(value) => onStatusChange(value as UserStatus)}
					>
						<SelectTrigger
							aria-label={t("admin.users.statusLabel")}
							disabled={savingProfile || roleStatusLocked}
						>
							<SelectValue>
								{(value: UserStatus | null) => (
									<StatusBadge
										status={value === "disabled" ? "disabled" : "active"}
									/>
								)}
							</SelectValue>
						</SelectTrigger>
						<SelectContent>
							<SelectItem value="active">
								<StatusBadge status="active" />
							</SelectItem>
							<SelectItem value="disabled">
								<StatusBadge status="disabled" />
							</SelectItem>
						</SelectContent>
					</Select>
				</UserDetailField>
				{role === "operator" ? (
					<div className="md:col-span-2">
						<OperatorScopeSelector
							disabled={savingProfile || roleStatusLocked}
							value={operatorScopes}
							onChange={onOperatorScopesChange}
						/>
					</div>
				) : null}
				{role === "admin" ? (
					<div className="md:col-span-2">
						<AdminScopePolicyNote />
					</div>
				) : null}
			</div>
		</section>
	);
}
