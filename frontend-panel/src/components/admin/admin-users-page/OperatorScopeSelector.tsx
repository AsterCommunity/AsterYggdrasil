import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { OPERATOR_SCOPES } from "@/lib/operatorScopes";
import type { OperatorScope } from "@/types/api";

export function OperatorScopeSelector({
	disabled = false,
	onChange,
	value,
}: {
	disabled?: boolean;
	onChange: (value: OperatorScope[]) => void;
	value: OperatorScope[];
}) {
	const { t } = useTranslation();
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
			<div className="grid gap-2 sm:grid-cols-2">
				{OPERATOR_SCOPES.map((scope) => {
					const checked = selected.has(scope);
					const inputId = `operator-scope-${scope}`;
					return (
						<div
							key={scope}
							className="flex items-center justify-between gap-3 rounded-md border border-border/60 bg-card/55 px-3 py-2 dark:border-white/10 dark:bg-card/40"
						>
							<Label htmlFor={inputId} className="min-w-0 text-sm leading-5">
								{t(`admin.users.operatorScope.${scope}`)}
							</Label>
							<Switch
								id={inputId}
								size="sm"
								checked={checked}
								disabled={disabled}
								onCheckedChange={(nextChecked) => {
									if (nextChecked) {
										onChange([...value, scope]);
										return;
									}
									onChange(value.filter((item) => item !== scope));
								}}
								aria-label={t(`admin.users.operatorScope.${scope}`)}
							/>
						</div>
					);
				})}
			</div>
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
