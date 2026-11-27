import { useTranslation } from "react-i18next";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import {
	type DeleteDialogState,
	textureDisplayName,
} from "./libraryTextureView";

export function TextureDeleteDialog({
	deleteDialog,
	onClose,
	onConfirm,
	open,
	submitting,
}: {
	deleteDialog: DeleteDialogState | null;
	onClose: (open: boolean) => void;
	onConfirm: () => void;
	open: boolean;
	submitting: boolean;
}) {
	const { t } = useTranslation();
	return (
		<ConfirmDialog
			open={open}
			loading={submitting}
			title={t("admin.textureLibraryTexturesPage.deleteTitle")}
			description={t("admin.textureLibraryTexturesPage.deleteDescription", {
				name: deleteDialog ? textureDisplayName(deleteDialog.texture) : "",
			})}
			cancelLabel={t("common.cancel")}
			confirmLabel={t("common.delete")}
			variant="destructive"
			onConfirm={onConfirm}
			onOpenChange={onClose}
		/>
	);
}
