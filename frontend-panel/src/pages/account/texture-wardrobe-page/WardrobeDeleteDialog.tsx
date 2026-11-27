import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import type { MinecraftWardrobeTextureMetadata } from "@/types/api";
import { TextureSummary } from "./WardrobeTextureSummary";

export function WardrobeDeleteDialog({
	clearStateAfterClose,
	deleteDialogOpen,
	deleteTexture,
	onClose,
	onConfirmDelete,
	submitting,
}: {
	clearStateAfterClose: () => void;
	deleteDialogOpen: boolean;
	deleteTexture: MinecraftWardrobeTextureMetadata | null;
	onClose: (open: boolean) => void;
	onConfirmDelete: () => void;
	submitting: boolean;
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={deleteDialogOpen} onOpenChange={onClose}>
			<DialogContent keepMounted onAnimationEnd={clearStateAfterClose}>
				<DialogHeader>
					<DialogTitle>{t("wardrobe.deleteDialogTitle")}</DialogTitle>
					<DialogDescription>
						{deleteTexture
							? t("wardrobe.deleteDialogDescription", {
									type: t(`wardrobe.type.${deleteTexture.texture_type}`),
								})
							: t("wardrobe.deleteDialogFallback")}
					</DialogDescription>
				</DialogHeader>

				{deleteTexture ? <TextureSummary texture={deleteTexture} /> : null}

				<DialogFooter>
					<DialogClose
						render={
							<Button type="button" variant="outline" disabled={submitting} />
						}
					>
						{t("common.cancel")}
					</DialogClose>
					<Button
						type="button"
						variant="destructive"
						disabled={!deleteTexture || submitting}
						onClick={onConfirmDelete}
					>
						{submitting ? t("wardrobe.saving") : t("wardrobe.deleteAction")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
