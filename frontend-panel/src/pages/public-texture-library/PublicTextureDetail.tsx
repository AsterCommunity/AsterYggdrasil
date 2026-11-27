import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";
import { TextureTagChips } from "@/components/yggdrasil/TextureTagList";
import { formatBytes } from "@/lib/numberUnit";
import { cn } from "@/lib/utils";
import type { PublicTextureLibraryTextureMetadata } from "@/types/api";
import { formatTextureKind } from "./textureKind";

export function PublicTextureDetail({
	texture,
}: {
	texture: PublicTextureLibraryTextureMetadata;
}) {
	const { t } = useTranslation();
	const skinUrl = texture.texture_type === "skin" ? texture.url : null;
	const capeUrl = texture.texture_type === "cape" ? texture.url : null;

	return (
		<div className="grid gap-4 sm:grid-cols-[minmax(0,0.85fr)_minmax(0,1fr)]">
			<div
				data-testid="public-texture-detail-preview"
				data-cape-url={capeUrl ?? ""}
				data-model={texture.texture_model}
				data-player-name={texture.name}
				data-skin-url={skinUrl ?? ""}
			>
				<MinecraftPreviewPanel
					label={t("wardrobe.previewTitle")}
					playerName={texture.name}
					skinUrl={skinUrl}
					capeUrl={capeUrl}
					compactHeader
					model={texture.texture_model}
					emptyTitle={t("wardrobe.previewEmptyTitle")}
					emptyDescription={t("wardrobe.previewEmptyDescription")}
					failedTitle={t("profiles.previewFailedTitle")}
					failedDescription={t("profiles.previewFailedDescription")}
					noSkinLabel={t("profiles.noSkinTexture")}
					idleLabel={t("profiles.motionIdle")}
					walkLabel={t("profiles.motionWalk")}
					frameClassName="h-72 sm:h-80"
					skeletonClassName="h-[23rem]"
					containerClassName="min-w-0"
				/>
			</div>
			<div className="grid content-start gap-3">
				<DetailRow label={t("library.textureName")} value={texture.name} />
				<DetailRow
					label={t("library.textureType")}
					value={formatTextureKind(texture, t)}
				/>
				<DetailRow
					label={t("library.uploader")}
					value={texture.uploader?.name ?? t("library.unknownUploader")}
				/>
				<div className="grid gap-1.5">
					<div className="text-xs font-medium text-muted-foreground">
						{t("wardrobe.visibility.label")}
					</div>
					<div>
						<Badge
							variant="outline"
							className="rounded-md border-emerald-500/35 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/35 dark:bg-emerald-400/10 dark:text-emerald-300"
						>
							{t("wardrobe.visibility.public")}
						</Badge>
					</div>
				</div>
				<div className="grid grid-cols-2 gap-2">
					<DetailRow
						label={t("wardrobe.dimensions")}
						value={`${texture.width}x${texture.height}`}
					/>
					<DetailRow
						label={t("wardrobe.size")}
						value={formatBytes(texture.file_size)}
					/>
				</div>
				<div className="grid gap-1.5">
					<div className="text-xs font-medium text-muted-foreground">
						{t("library.tags")}
					</div>
					{texture.tags.length > 0 ? (
						<TextureTagChips tags={texture.tags} />
					) : (
						<div className="text-sm text-muted-foreground">
							{t("library.noTags")}
						</div>
					)}
				</div>
			</div>
		</div>
	);
}

function DetailRow({
	label,
	mono,
	value,
}: {
	label: string;
	mono?: boolean;
	value: string;
}) {
	return (
		<div className="grid min-w-0 gap-1.5">
			<div className="text-xs font-medium text-muted-foreground">{label}</div>
			<div
				className={cn(
					"min-w-0 break-words text-sm font-medium",
					mono && "font-mono text-xs leading-5",
				)}
			>
				{value}
			</div>
		</div>
	);
}
