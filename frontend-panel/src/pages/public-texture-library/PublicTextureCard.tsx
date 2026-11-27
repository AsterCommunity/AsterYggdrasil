import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Badge } from "@/components/ui/badge";
import { MinecraftTextureImagePreview } from "@/components/yggdrasil/MinecraftTextureImagePreview";
import { TextureLibraryTextureAvatar } from "@/components/yggdrasil/TextureLibraryTextureAvatar";
import { TextureTagChips } from "@/components/yggdrasil/TextureTagList";
import { formatBytes } from "@/lib/numberUnit";
import { publicTexturePath } from "@/routes/routePaths";
import type { PublicTextureLibraryTextureMetadata } from "@/types/api";

export function PublicTextureCard({
	texture,
}: {
	texture: PublicTextureLibraryTextureMetadata;
}) {
	const { t } = useTranslation();

	return (
		<Link
			to={publicTexturePath(texture.id)}
			className="group grid overflow-hidden rounded-md border border-border/70 bg-background text-left shadow-xs transition hover:-translate-y-0.5 hover:border-primary/45 hover:shadow-md focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35"
		>
			<MinecraftTextureImagePreview
				alt={t("library.texturePreviewAlt", {
					type: t(`wardrobe.type.${texture.texture_type}`),
				})}
				draggable={false}
				previewUrl={texture.preview_url}
				textureUrl={texture.url}
			/>
			<div className="flex min-w-0 gap-3 border-t border-border/70 bg-muted/25 p-3">
				<PublicTextureCardAvatar texture={texture} />
				<div className="grid min-w-0 flex-1 gap-3">
					<div className="min-w-0">
						<h2 className="truncate text-sm font-semibold tracking-normal group-hover:text-primary">
							{texture.name}
						</h2>
						<div className="mt-2 flex flex-wrap items-center gap-1.5">
							<Badge variant="secondary" className="rounded-md">
								{t(`wardrobe.type.${texture.texture_type}`)}
							</Badge>
							{texture.texture_type === "skin" ? (
								<Badge variant="outline" className="rounded-md">
									{texture.texture_model}
								</Badge>
							) : null}
						</div>
					</div>
					<div className="flex flex-wrap gap-x-2 gap-y-1 text-xs text-muted-foreground">
						<span>
							{texture.uploader?.name ?? t("library.unknownUploader")}
						</span>
						<span>
							{texture.width}x{texture.height}
						</span>
						<span>{formatBytes(texture.file_size)}</span>
					</div>
					<TextureTagChips tags={texture.tags} />
				</div>
			</div>
		</Link>
	);
}

function PublicTextureCardAvatar({
	texture,
}: {
	texture: PublicTextureLibraryTextureMetadata;
}) {
	return (
		<TextureLibraryTextureAvatar
			texture={texture}
			className="size-12 rounded-lg bg-background/70"
			testId={`public-texture-card-avatar-${texture.id}`}
			imageTestId={`public-texture-card-avatar-image-${texture.id}`}
		/>
	);
}
