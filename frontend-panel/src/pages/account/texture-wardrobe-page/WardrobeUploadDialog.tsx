import type { DragEvent, FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { TextureUploadForm } from "@/components/yggdrasil/TextureUploadForm";
import type {
	MinecraftTextureModel,
	MinecraftTextureType,
	MinecraftTextureVisibility,
	MinecraftWardrobeTextureMetadata,
} from "@/types/api";

export function WardrobeUploadDialog({
	clearStateAfterClose,
	dragActive,
	file,
	model,
	onCancel,
	onDragEnter,
	onDragLeave,
	onDrop,
	onFileChange,
	onModelChange,
	onNameChange,
	onOpenChange,
	onSubmit,
	onTextureTypeChange,
	onVisibilityChange,
	open,
	previewTexture,
	submitting,
	textureType,
	uploadName,
	visibility,
}: {
	clearStateAfterClose: () => void;
	dragActive: boolean;
	file: File | null;
	model: MinecraftTextureModel;
	onCancel: () => void;
	onDragEnter: (event: DragEvent<HTMLLabelElement>) => void;
	onDragLeave: () => void;
	onDrop: (event: DragEvent<HTMLLabelElement>) => void;
	onFileChange: (file: File | null) => void;
	onModelChange: (model: MinecraftTextureModel) => void;
	onNameChange: (name: string) => void;
	onOpenChange: (open: boolean) => void;
	onSubmit: (event: FormEvent<HTMLFormElement>) => void;
	onTextureTypeChange: (type: MinecraftTextureType) => void;
	onVisibilityChange: (visibility: MinecraftTextureVisibility) => void;
	open: boolean;
	previewTexture: MinecraftWardrobeTextureMetadata | null;
	submitting: boolean;
	textureType: MinecraftTextureType;
	uploadName: string;
	visibility: MinecraftTextureVisibility;
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				keepMounted
				className="sm:max-w-lg"
				onAnimationEnd={clearStateAfterClose}
			>
				<TextureUploadForm
					description={t("wardrobe.uploadDescription")}
					dragActive={dragActive}
					file={file}
					fileInputId="wardrobe-texture-file"
					model={model}
					name={uploadName}
					nameLabel={t("wardrobe.textureName")}
					namePlaceholder={
						previewTexture
							? t("wardrobe.textureNamePlaceholder", {
									name: previewTexture.name,
								})
							: t("wardrobe.textureNamePlaceholderFallback")
					}
					submitLabel={t("common.upload")}
					submittingLabel={t("wardrobe.uploading")}
					submitting={submitting}
					textureType={textureType}
					title={t("wardrobe.uploadTitle")}
					visibility={visibility}
					onCancel={onCancel}
					onDragEnter={onDragEnter}
					onDragLeave={onDragLeave}
					onDrop={onDrop}
					onFileChange={onFileChange}
					onModelChange={onModelChange}
					onNameChange={onNameChange}
					onSubmit={onSubmit}
					onTextureTypeChange={(nextType) => {
						onTextureTypeChange(nextType);
					}}
					onVisibilityChange={onVisibilityChange}
				/>
			</DialogContent>
		</Dialog>
	);
}
