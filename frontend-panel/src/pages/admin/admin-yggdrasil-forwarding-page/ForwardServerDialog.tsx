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
import { Switch } from "@/components/ui/switch";
import type { AdminYggdrasilSessionForwardServerInfo } from "@/types/api";
import type { ForwardForm } from "./forwardServerForm";

export function ForwardServerDialog({
	form,
	onFieldChange,
	onOpenChange,
	onSubmit,
	open,
	server,
	submitting,
}: {
	form: ForwardForm;
	onFieldChange: <K extends keyof ForwardForm>(
		key: K,
		value: ForwardForm[K],
	) => void;
	onOpenChange: (open: boolean) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	open: boolean;
	server: AdminYggdrasilSessionForwardServerInfo | null;
	submitting: boolean;
}) {
	const { t } = useTranslation();
	const editingLocal = server?.local === true;
	const editing = server !== null;

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent keepMounted className="sm:max-w-2xl">
				<form onSubmit={onSubmit} className="grid min-h-0 gap-4">
					<DialogHeader>
						<DialogTitle>
							{editing
								? t("admin.yggdrasilForwarding.editTitle")
								: t("admin.yggdrasilForwarding.createTitle")}
						</DialogTitle>
						<DialogDescription>
							{editingLocal
								? t("admin.yggdrasilForwarding.localEditDescription")
								: t("admin.yggdrasilForwarding.dialogDescription")}
						</DialogDescription>
					</DialogHeader>

					<div className="grid gap-4 sm:grid-cols-2">
						<div className="grid gap-1.5">
							<Label htmlFor="forward-display-name">
								{t("admin.yggdrasilForwarding.displayName")}
							</Label>
							<Input
								id="forward-display-name"
								value={form.displayName}
								required
								maxLength={128}
								onChange={(event) =>
									onFieldChange("displayName", event.currentTarget.value)
								}
							/>
						</div>
						<div className="grid gap-1.5">
							<Label htmlFor="forward-base-url">
								{t("admin.yggdrasilForwarding.baseUrl")}
							</Label>
							<Input
								id="forward-base-url"
								type="url"
								value={form.baseUrl}
								required={!editingLocal}
								disabled={editingLocal}
								placeholder="https://auth.example.com/yggdrasil"
								onChange={(event) =>
									onFieldChange("baseUrl", event.currentTarget.value)
								}
							/>
						</div>
						<NumberField
							id="forward-priority"
							label={t("admin.yggdrasilForwarding.priority")}
							value={form.priority}
							min={-10000}
							max={10000}
							onChange={(value) => onFieldChange("priority", value)}
						/>
						<NumberField
							id="forward-weight"
							label={t("admin.yggdrasilForwarding.weight")}
							value={form.weight}
							min={1}
							max={1000}
							onChange={(value) => onFieldChange("weight", value)}
						/>
						<NumberField
							id="forward-timeout"
							label={t("admin.yggdrasilForwarding.timeoutMs")}
							value={form.timeoutMs}
							min={100}
							max={10000}
							onChange={(value) => onFieldChange("timeoutMs", value)}
						/>
						<div className="flex flex-col justify-end gap-3 rounded-lg border border-border/70 bg-muted/20 px-3 py-2">
							<div className="flex items-center justify-between gap-3">
								<span className="text-sm font-medium">
									{t("admin.yggdrasilForwarding.enabled")}
								</span>
								<Switch
									checked={form.enabled}
									aria-label={t("admin.yggdrasilForwarding.enabled")}
									onCheckedChange={(value) => onFieldChange("enabled", value)}
								/>
							</div>
							<div className="flex items-center justify-between gap-3">
								<span className="text-sm font-medium">
									{t("admin.yggdrasilForwarding.textureForward")}
								</span>
								<Switch
									checked={form.textureForwardEnabled}
									disabled={editingLocal}
									aria-label={t("admin.yggdrasilForwarding.textureForward")}
									onCheckedChange={(value) =>
										onFieldChange("textureForwardEnabled", value)
									}
								/>
							</div>
						</div>
					</div>

					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={submitting}
							onClick={() => onOpenChange(false)}
						>
							{t("common.cancel")}
						</Button>
						<Button type="submit" disabled={submitting}>
							{submitting ? (
								<Icon name="Spinner" className="mr-2 size-4 animate-spin" />
							) : null}
							{editing
								? t("admin.yggdrasilForwarding.saveAction")
								: t("admin.yggdrasilForwarding.createAction")}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}

function NumberField({
	id,
	label,
	max,
	min,
	onChange,
	value,
}: {
	id: string;
	label: string;
	max: number;
	min: number;
	onChange: (value: string) => void;
	value: string;
}) {
	return (
		<div className="grid gap-1.5">
			<Label htmlFor={id}>{label}</Label>
			<Input
				id={id}
				type="number"
				value={value}
				min={min}
				max={max}
				onChange={(event) => onChange(event.currentTarget.value)}
			/>
		</div>
	);
}
