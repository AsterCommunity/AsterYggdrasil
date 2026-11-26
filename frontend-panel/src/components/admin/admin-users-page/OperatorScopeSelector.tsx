import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
	permissionLabel,
	useRegistryPermissions,
} from "@/lib/permissionRegistry";
import type { PermissionId } from "@/types/api";

export function OperatorScopeSelector({
	disabled = false,
	onChange,
	value,
}: {
	disabled?: boolean;
	onChange: (value: PermissionId[]) => void;
	value: PermissionId[];
}) {
	const { t } = useTranslation();
	const {
		permissions: adminAreas,
		status,
		retry,
	} = useRegistryPermissions("admin_area");
	const selected = new Set(value);

	return (
		<div className="rounded-lg border border-border/70 bg-background/70 p-4 dark:border-white/10 dark:bg-input/10">
			<div className="mb-3">
				<div className="font-medium text-sm">
					{t("admin.users.operatorScopesLabel")}
				</div>
				<p className="mt-1 text-muted-foreground text-xs leading-5">
					{t("admin.users.operatorScopesDescription")}
				</p>
			</div>
			{status === "error" ? (
				<div className="flex items-center justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm">
					<span className="text-destructive">
						{t("admin.users.permissionsLoadFailed")}
					</span>
					<Button type="button" variant="outline" size="sm" onClick={retry}>
						{t("admin.users.retryPermissionsLoad")}
					</Button>
				</div>
			) : status === "loading" ? (
				<div className="rounded-md border border-border/60 bg-muted/20 px-3 py-2 text-muted-foreground text-sm dark:border-white/10 dark:bg-white/[0.03]">
					{t("admin.users.operatorScopesLoading")}
				</div>
			) : (
				<div className="grid gap-2 sm:grid-cols-2">
					{adminAreas.map((permission) => {
						const checked = selected.has(permission.id);
						const inputId = `operator-scope-${permission.id}`;
						return (
							<div
								key={permission.id}
								className="flex items-center justify-between gap-3 rounded-md border border-border/60 bg-card/55 px-3 py-2 dark:border-white/10 dark:bg-card/40"
							>
								<Label htmlFor={inputId} className="min-w-0 text-sm leading-5">
									{permissionLabel(permission, t)}
								</Label>
								<Switch
									id={inputId}
									size="sm"
									checked={checked}
									disabled={disabled}
									onCheckedChange={(nextChecked) => {
										if (nextChecked) {
											onChange([...value, permission.id]);
											return;
										}
										onChange(value.filter((item) => item !== permission.id));
									}}
									aria-label={permissionLabel(permission, t)}
								/>
							</div>
						);
					})}
				</div>
			)}
		</div>
	);
}

export function AdminScopePolicyNote() {
	const { t } = useTranslation();
	return (
		<div className="rounded-lg border border-sky-500/25 bg-sky-500/10 p-4 text-sky-900 text-sm leading-6 dark:text-sky-100">
			{t("admin.users.adminScopePolicy")}
		</div>
	);
}
