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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { TemplateVariableGroup, TemplateVariableItem } from "@/types/api";
import { translateOrFallback } from "./settingsCategories";
import {
	formatMailTemplateGroupLabel,
	formatTemplateVariableGroupLabel,
} from "./settingsMailTemplates";

export function MailTemplateVariablesDialog({
	activeGroup,
	activeGroupCode,
	onOpenChange,
}: {
	activeGroup: TemplateVariableGroup | null;
	activeGroupCode: string | null;
	onOpenChange: (open: boolean) => void;
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={activeGroupCode !== null} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-[calc(100%-1.5rem)] sm:max-w-[min(56rem,calc(100vw-2rem))]">
				<DialogHeader>
					<DialogTitle>
						{t("mail_template_variables_dialog_title", {
							name: activeGroup
								? formatTemplateVariableGroupLabel(activeGroup, t)
								: formatMailTemplateGroupLabel(activeGroupCode ?? "", t),
						})}
					</DialogTitle>
					<DialogDescription>
						{t("mail_template_variables_dialog_desc")}
					</DialogDescription>
				</DialogHeader>
				<div className="max-h-[min(70vh,38rem)] overflow-y-auto py-2 pr-1">
					{activeGroup && activeGroup.variables.length > 0 ? (
						<div className="grid gap-3 sm:grid-cols-2">
							{activeGroup.variables.map((variable) => (
								<TemplateVariableCard
									key={`${activeGroup.template_code}:${variable.token}`}
									variable={variable}
								/>
							))}
						</div>
					) : (
						<p className="text-sm text-muted-foreground">
							{t("mail_template_variables_dialog_empty")}
						</p>
					)}
				</div>
				<DialogFooter>
					<Button
						type="button"
						variant="outline"
						onClick={() => onOpenChange(false)}
					>
						{t("common.cancel")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function TemplateVariableCard({
	variable,
}: {
	variable: TemplateVariableItem;
}) {
	const { t } = useTranslation();
	const label = translateOrFallback(t, variable.label_i18n_key, variable.token);
	const description = translateOrFallback(t, variable.description_i18n_key, "");

	return (
		<div className="rounded-lg border border-border/60 bg-card/40 p-3">
			<div className="flex flex-wrap items-center gap-2">
				<code className="break-all rounded bg-muted px-2 py-1 font-mono text-xs">
					{variable.token}
				</code>
				<span className="text-sm font-medium">{label}</span>
			</div>
			{description ? (
				<p className="mt-2 break-words text-sm leading-6 text-muted-foreground">
					{description}
				</p>
			) : null}
		</div>
	);
}

export function TestEmailDialog({
	open,
	sending,
	target,
	onOpenChange,
	onSend,
	onTargetChange,
}: {
	open: boolean;
	sending: boolean;
	target: string;
	onOpenChange: (open: boolean) => void;
	onSend: () => void;
	onTargetChange: (value: string) => void;
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-md">
				<DialogHeader>
					<DialogTitle>{t("mail_test_email_dialog_title")}</DialogTitle>
					<DialogDescription>
						{t("mail_test_email_dialog_desc")}
					</DialogDescription>
				</DialogHeader>
				<div className="space-y-2 py-2">
					<Label htmlFor="settings-test-email-target">
						{t("mail_test_email_recipient_label")}
					</Label>
					<Input
						id="settings-test-email-target"
						type="email"
						value={target}
						onChange={(event) => onTargetChange(event.currentTarget.value)}
						placeholder={t("mail_test_email_recipient_placeholder")}
					/>
				</div>
				<DialogFooter>
					<Button
						type="button"
						variant="outline"
						disabled={sending}
						onClick={() => onOpenChange(false)}
					>
						{t("common.cancel")}
					</Button>
					<Button type="button" disabled={sending} onClick={onSend}>
						{sending ? t("mail_test_email_sending") : t("mail_send_test_email")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
