import { type DragEvent, type FormEvent, type ReactNode, useId } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import type {
	MinecraftTextureModel,
	MinecraftTextureType,
	MinecraftTextureVisibility,
} from "@/types/api";

export type TextureUploadFormProps = {
	cancelLabel?: ReactNode;
	description: ReactNode;
	dragActive: boolean;
	file: File | null;
	fileInputId: string;
	model: MinecraftTextureModel;
	name?: string;
	nameLabel?: ReactNode;
	namePlaceholder?: string;
	onCancel: () => void;
	onDragEnter: (event: DragEvent<HTMLLabelElement>) => void;
	onDragLeave: () => void;
	onDrop: (event: DragEvent<HTMLLabelElement>) => void;
	onFileChange: (file: File | null) => void | Promise<void>;
	onModelChange: (model: MinecraftTextureModel) => void;
	onNameChange?: (name: string) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onTextureTypeChange: (type: MinecraftTextureType) => void;
	onVisibilityChange: (visibility: MinecraftTextureVisibility) => void;
	submitDisabled?: boolean;
	submitLabel: ReactNode;
	submitting: boolean;
	submittingLabel: ReactNode;
	textureType: MinecraftTextureType;
	textureTypeLocked?: boolean;
	title: ReactNode;
	visibility: MinecraftTextureVisibility;
};

export function TextureUploadForm({
	cancelLabel,
	description,
	dragActive,
	file,
	fileInputId,
	model,
	name,
	nameLabel,
	namePlaceholder,
	onCancel,
	onDragEnter,
	onDragLeave,
	onDrop,
	onFileChange,
	onModelChange,
	onNameChange,
	onSubmit,
	onTextureTypeChange,
	onVisibilityChange,
	submitDisabled,
	submitLabel,
	submitting,
	submittingLabel,
	textureType,
	textureTypeLocked = false,
	title,
	visibility,
}: TextureUploadFormProps) {
	const { t } = useTranslation();

	return (
		<form className="grid gap-4" onSubmit={onSubmit}>
			<DialogHeader>
				<DialogTitle>{title}</DialogTitle>
				<DialogDescription>{description}</DialogDescription>
			</DialogHeader>
			<div className="grid gap-3">
				{textureTypeLocked ? null : (
					<SegmentedChoiceField
						label={t("profiles.textureType")}
						value={textureType}
						onChange={(next) =>
							onTextureTypeChange(next as MinecraftTextureType)
						}
						options={[
							{ label: t("home.textureTypeSkin"), value: "skin" },
							{ label: t("home.textureTypeCape"), value: "cape" },
						]}
					/>
				)}
				{textureType === "skin" ? (
					<SegmentedChoiceField
						label={t("profiles.model")}
						value={model}
						onChange={(next) => onModelChange(next as MinecraftTextureModel)}
						options={[
							{ label: t("profiles.defaultModel"), value: "default" },
							{ label: t("profiles.slimModel"), value: "slim" },
						]}
					/>
				) : null}
				<SegmentedChoiceField
					label={t("wardrobe.visibility.label")}
					value={visibility}
					onChange={(next) =>
						onVisibilityChange(next as MinecraftTextureVisibility)
					}
					options={[
						{
							label: t("wardrobe.visibility.private"),
							value: "private",
						},
						{
							label: t("wardrobe.visibility.public"),
							value: "public",
						},
					]}
				/>
				{onNameChange ? (
					<div className="grid gap-1.5">
						<Label htmlFor={`${fileInputId}-name`}>
							{nameLabel ?? t("wardrobe.textureName")}
						</Label>
						<Input
							id={`${fileInputId}-name`}
							value={name ?? ""}
							maxLength={96}
							placeholder={namePlaceholder}
							onChange={(event) => onNameChange(event.currentTarget.value)}
						/>
					</div>
				) : null}
				<Label
					htmlFor={fileInputId}
					className={cn(
						"grid cursor-pointer place-items-center rounded-lg border border-dashed px-4 py-6 text-center transition-colors",
						"border-emerald-500/50 bg-emerald-500/5 hover:border-emerald-500 hover:bg-emerald-500/10 dark:border-emerald-400/40 dark:bg-emerald-400/5 dark:hover:border-emerald-300 dark:hover:bg-emerald-400/10",
						dragActive &&
							"border-emerald-500 bg-emerald-500/15 ring-3 ring-emerald-500/20 dark:border-emerald-300 dark:bg-emerald-400/15 dark:ring-emerald-300/20",
					)}
					onDragEnter={onDragEnter}
					onDragOver={onDragEnter}
					onDragLeave={onDragLeave}
					onDrop={onDrop}
				>
					<Input
						id={fileInputId}
						type="file"
						accept="image/png"
						aria-label={t("profiles.file")}
						className="sr-only"
						onChange={(event) =>
							void onFileChange(event.currentTarget.files?.[0] ?? null)
						}
					/>
					{file ? (
						<span className="flex max-w-full flex-col items-center gap-1 text-sm font-medium">
							<span>{t("profiles.selectedFileLabel")}</span>
							<span className="max-w-full break-all font-mono text-xs leading-5 text-foreground/90">
								{file.name}
							</span>
						</span>
					) : (
						<span className="max-w-full text-sm font-medium">
							{dragActive
								? t("profiles.fileDropActiveTitle")
								: t("profiles.fileDropTitle")}
						</span>
					)}
					<span className="mt-1 text-xs text-muted-foreground">
						{dragActive
							? t("profiles.fileDropActiveDescription")
							: t("profiles.fileDropDescription")}
					</span>
				</Label>
			</div>
			<DialogFooter>
				<Button
					type="button"
					variant="outline"
					disabled={submitting}
					onClick={onCancel}
				>
					{cancelLabel ?? t("common.cancel")}
				</Button>
				<Button
					type="submit"
					disabled={submitDisabled ?? (!file || submitting)}
				>
					{submitting ? submittingLabel : submitLabel}
				</Button>
			</DialogFooter>
		</form>
	);
}

function SegmentedChoiceField<T extends string>({
	label,
	onChange,
	options,
	value,
}: {
	label: string;
	onChange: (value: T) => void;
	options: { label: string; value: T }[];
	value: T;
}) {
	const generatedId = useId();

	return (
		<div className="grid gap-1.5">
			<Label>{label}</Label>
			<div
				role="radiogroup"
				aria-label={label}
				className="grid grid-cols-2 gap-1 rounded-lg border border-border/70 bg-muted/30 p-1"
			>
				{options.map((option) => {
					const active = option.value === value;
					const inputId = `${generatedId}-${option.value}`;
					return (
						<label
							key={option.value}
							htmlFor={inputId}
							className={cn(
								"relative grid h-8 cursor-pointer place-items-center overflow-hidden rounded-md px-3 text-sm font-medium transition-colors has-focus-visible:ring-3 has-focus-visible:ring-ring/35",
								active
									? "bg-primary text-primary-foreground shadow-xs"
									: "text-muted-foreground hover:bg-background/75 hover:text-foreground",
							)}
						>
							<input
								id={inputId}
								type="radio"
								name={generatedId}
								value={option.value}
								checked={active}
								className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
								onChange={() => onChange(option.value)}
							/>
							{option.label}
						</label>
					);
				})}
			</div>
		</div>
	);
}
