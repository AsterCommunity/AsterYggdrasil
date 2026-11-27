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
import { Textarea } from "@/components/ui/textarea";
import {
	type ReviewDialogState,
	textureDisplayName,
} from "./libraryTextureView";

export function TextureReviewDialog({
	onClose,
	onNoteChange,
	onSubmit,
	open,
	reviewDialog,
	reviewNote,
	submitting,
}: {
	onClose: () => void;
	onNoteChange: (value: string) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	open: boolean;
	reviewDialog: ReviewDialogState | null;
	reviewNote: string;
	submitting: boolean;
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={open} onOpenChange={onClose}>
			<DialogContent keepMounted className="sm:max-w-lg">
				<form className="grid gap-4" onSubmit={onSubmit}>
					<DialogHeader>
						<DialogTitle>
							{reviewDialog
								? t(
										`admin.textureLibraryTexturesPage.${reviewDialog.action}Title`,
									)
								: t("admin.textureLibraryTexturesPage.reviewTitle")}
						</DialogTitle>
						<DialogDescription>
							{reviewDialog
								? t(
										`admin.textureLibraryTexturesPage.${reviewDialog.action}Description`,
										{
											name: textureDisplayName(reviewDialog.texture),
										},
									)
								: t("admin.textureLibraryTexturesPage.reviewDescription")}
						</DialogDescription>
					</DialogHeader>
					<div className="grid gap-2">
						<label
							htmlFor="texture-library-review-note"
							className="text-sm font-medium"
						>
							{t("admin.textureLibraryTexturesPage.reviewNote")}
						</label>
						<Textarea
							id="texture-library-review-note"
							value={reviewNote}
							maxLength={500}
							placeholder={t(
								"admin.textureLibraryTexturesPage.reviewNotePlaceholder",
							)}
							onChange={(event) => onNoteChange(event.currentTarget.value)}
						/>
						<p className="text-xs text-muted-foreground">
							{reviewDialog?.action === "reject"
								? t("admin.textureLibraryTexturesPage.rejectNoteHint")
								: t("admin.textureLibraryTexturesPage.reviewNoteHint")}
						</p>
					</div>
					<DialogFooter>
						<Button
							type="button"
							variant="outline"
							disabled={submitting}
							onClick={onClose}
						>
							{t("common.cancel")}
						</Button>
						<Button
							type="submit"
							variant={
								reviewDialog?.action === "reject" ? "destructive" : "default"
							}
							disabled={submitting}
						>
							{submitting ? (
								<Icon name="Spinner" className="size-4 animate-spin" />
							) : null}
							{reviewDialog
								? t(
										`admin.textureLibraryTexturesPage.${reviewDialog.action}Action`,
									)
								: t("common.save")}
						</Button>
					</DialogFooter>
				</form>
			</DialogContent>
		</Dialog>
	);
}
