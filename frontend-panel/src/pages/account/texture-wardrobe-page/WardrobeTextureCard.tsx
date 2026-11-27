import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { MinecraftTextureImagePreview } from "@/components/yggdrasil/MinecraftTextureImagePreview";
import { TextureTagChips } from "@/components/yggdrasil/TextureTagList";
import { formatBytes } from "@/lib/numberUnit";
import { cn } from "@/lib/utils";
import type { MinecraftWardrobeTextureMetadata } from "@/types/api";
import {
	LibraryStatusBadge,
	textureLabel,
	textureVisibilityBadgeClass,
} from "./WardrobeTextureSummary";

export function WardrobeTextureCard({
	active,
	date,
	onSelect,
	texture,
}: {
	active: boolean;
	date: ReactNode;
	onSelect: () => void;
	texture: MinecraftWardrobeTextureMetadata;
}) {
	const { t } = useTranslation();
	const label = textureLabel(texture);

	return (
		<button
			type="button"
			className={cn(
				"group grid overflow-hidden rounded-md border bg-background text-left shadow-xs transition hover:-translate-y-0.5 hover:border-primary/45 hover:shadow-md focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/35",
				active
					? "border-primary/70 ring-2 ring-primary/20"
					: "border-border/70",
			)}
			onClick={onSelect}
		>
			<WardrobeTexturePreview texture={texture} compact />
			<div className="grid gap-1.5 border-t border-border/70 bg-muted/30 p-2.5">
				<div className="flex min-w-0 items-center justify-between gap-2">
					<div className="truncate text-xs font-semibold">{label}</div>
					<div className="flex shrink-0 items-center gap-1">
						<span className="rounded-md bg-background/80 px-1.5 py-0.5 text-[0.6875rem] text-muted-foreground">
							{t(`wardrobe.type.${texture.texture_type}`)}
						</span>
						<LibraryStatusBadge status={texture.library_status} compact />
						<Badge
							variant="outline"
							className={cn(
								"h-5 rounded-md px-1.5 text-[0.6875rem]",
								textureVisibilityBadgeClass(texture.visibility),
							)}
						>
							{t(`wardrobe.visibility.${texture.visibility}`)}
						</Badge>
					</div>
				</div>
				<div className="flex flex-wrap gap-x-2 gap-y-1 text-[0.6875rem] text-muted-foreground">
					<span>
						{texture.width}x{texture.height}
					</span>
					<span>{formatBytes(texture.file_size)}</span>
					<span>{date}</span>
				</div>
				<TextureTagChips tags={texture.tags} />
			</div>
		</button>
	);
}

export function WardrobeTexturePreview({
	compact,
	texture,
}: {
	compact?: boolean;
	texture: MinecraftWardrobeTextureMetadata;
}) {
	const { t } = useTranslation();
	const alt = t("wardrobe.texturePreviewAlt", {
		type: t(`wardrobe.type.${texture.texture_type}`),
	});

	return (
		<MinecraftTextureImagePreview
			alt={alt}
			aspect={compact ? "wide" : "portrait"}
			previewUrl={texture.preview_url}
			textureUrl={texture.url}
		/>
	);
}
