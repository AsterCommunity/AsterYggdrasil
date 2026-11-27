import { type FormEvent, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
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
import { cn } from "@/lib/utils";
import { ApiError, formatUnknownError } from "@/services/http";
import { yggdrasilService } from "@/services/yggdrasilService";
import type { PublicTextureLibraryTextureMetadata } from "@/types/api";

export function PublicTextureCopyDialog({
	onCopied,
	onOpenChange,
	open,
	texture,
}: {
	onCopied?: () => void;
	onOpenChange: (open: boolean) => void;
	open: boolean;
	texture: PublicTextureLibraryTextureMetadata | null;
}) {
	const { t } = useTranslation();
	const [copying, setCopying] = useState(false);
	const [copyName, setCopyName] = useState("");
	const [copyError, setCopyError] = useState<string | null>(null);

	useEffect(() => {
		if (open && texture) {
			setCopyName(texture.name);
			setCopyError(null);
		}
	}, [open, texture]);

	function clearCopyDialogStateAfterClose() {
		if (open) return;
		setCopyName("");
		setCopyError(null);
	}

	async function copyTexture(event: FormEvent<HTMLFormElement>) {
		event.preventDefault();
		if (!texture) return;
		setCopying(true);
		setCopyError(null);
		try {
			const copied = await yggdrasilService.copyPublicTextureToWardrobe(
				texture.id,
				{
					display_name: copyName.trim() || null,
				},
			);
			toast.success(t("library.copySuccess", { name: copied.name }));
			onOpenChange(false);
			onCopied?.();
		} catch (error) {
			if (
				error instanceof ApiError &&
				error.code === "wardrobe.texture_name_taken"
			) {
				setCopyError(t("library.copyNameTaken"));
			} else {
				toast.error(formatUnknownError(error));
			}
		} finally {
			setCopying(false);
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				keepMounted
				className="grid-rows-[auto_minmax(0,1fr)_auto] sm:max-w-md"
				onAnimationEnd={clearCopyDialogStateAfterClose}
			>
				<DialogHeader>
					<DialogTitle>{t("library.copyDialogTitle")}</DialogTitle>
					<DialogDescription>
						{texture
							? t("library.copyDialogDescription", {
									name: texture.name,
								})
							: t("library.detailFallback")}
					</DialogDescription>
				</DialogHeader>

				{texture ? (
					<form
						id="public-texture-copy-form"
						className="min-h-0 space-y-3 overflow-y-auto pr-1"
						onSubmit={(event) => void copyTexture(event)}
					>
						<div className="grid gap-2">
							<Label htmlFor="public-texture-copy-name">
								{t("library.copyNameLabel")}
							</Label>
							<Input
								id="public-texture-copy-name"
								value={copyName}
								maxLength={96}
								disabled={copying}
								aria-invalid={copyError ? true : undefined}
								aria-describedby={
									copyError
										? "public-texture-copy-name-error"
										: "public-texture-copy-name-help"
								}
								onChange={(event) => {
									setCopyName(event.currentTarget.value);
									setCopyError(null);
								}}
							/>
							<p
								id="public-texture-copy-name-help"
								className="text-xs leading-5 text-muted-foreground"
							>
								{t("library.copyNameHelp")}
							</p>
							{copyError ? (
								<p
									id="public-texture-copy-name-error"
									className="text-sm font-medium text-destructive"
								>
									{copyError}
								</p>
							) : null}
						</div>
					</form>
				) : null}

				<DialogFooter>
					<Button
						type="button"
						variant="outline"
						disabled={copying}
						onClick={() => onOpenChange(false)}
					>
						{t("common.cancel")}
					</Button>
					<Button
						type="submit"
						form="public-texture-copy-form"
						disabled={!texture || copying}
					>
						<Icon
							name={copying ? "Spinner" : "Copy"}
							className={cn("size-4", copying && "animate-spin")}
						/>
						{copying ? t("library.copying") : t("library.copyConfirmAction")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
