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

type GeneratedPasswordDialogProps = {
	open: boolean;
	password: string | null;
	username: string;
	onCopy: () => void;
	onOpenChange: (open: boolean) => void;
};

export function GeneratedPasswordDialog({
	open,
	password,
	username,
	onCopy,
	onOpenChange,
}: GeneratedPasswordDialogProps) {
	const { t } = useTranslation();

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle>{t("admin.users.generatedPasswordTitle")}</DialogTitle>
					<DialogDescription>
						{t("admin.users.generatedPasswordDialogDescription", { username })}
					</DialogDescription>
				</DialogHeader>
				<div className="space-y-3 rounded-lg border border-primary/30 bg-primary/5 p-3">
					<div className="flex items-start gap-2">
						<Icon name="Key" className="mt-0.5 size-4 shrink-0 text-primary" />
						<p className="text-muted-foreground text-sm">
							{t("admin.users.generatedPasswordDescription")}
						</p>
					</div>
					<div className="flex gap-2">
						<Input
							readOnly
							value={password ?? ""}
							className="font-mono text-sm"
							aria-label={t("admin.users.generatedPassword")}
						/>
						<Button
							type="button"
							variant="outline"
							size="icon"
							disabled={!password}
							aria-label={t("admin.users.copyGeneratedPassword")}
							onClick={onCopy}
						>
							<Icon name="Copy" className="size-4" />
						</Button>
					</div>
				</div>
				<DialogFooter>
					<Button type="button" onClick={() => onOpenChange(false)}>
						{t("common.close")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
