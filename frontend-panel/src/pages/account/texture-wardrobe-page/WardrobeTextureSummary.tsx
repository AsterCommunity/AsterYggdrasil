import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { TextureTagChips } from "@/components/yggdrasil/TextureTagList";
import { formatBytes } from "@/lib/numberUnit";
import { cn } from "@/lib/utils";
import type { MinecraftWardrobeTextureMetadata } from "@/types/api";

export function LibraryStatusBadge({
	compact,
	status,
}: {
	compact?: boolean;
	status: MinecraftWardrobeTextureMetadata["library_status"];
}) {
	const { t } = useTranslation();
	return (
		<Badge
			variant={status === "published" ? "default" : "outline"}
			className={cn(
				"rounded-md",
				compact && "h-5 px-1.5 text-[0.6875rem]",
				status === "pending_review" &&
					"border-amber-500/35 bg-amber-500/10 text-amber-700 dark:text-amber-300",
				status === "rejected" &&
					"border-destructive/35 bg-destructive/10 text-destructive",
			)}
		>
			{t(`wardrobe.libraryStatus.${status}`)}
		</Badge>
	);
}

export function TextureSummary({
	texture,
}: {
	texture: MinecraftWardrobeTextureMetadata;
}) {
	const { t } = useTranslation();

	return (
		<div className="grid gap-2 rounded-lg border border-border/70 bg-muted/20 p-3 text-sm">
			<div className="min-w-0 truncate font-semibold">{texture.name}</div>
			<div className="flex flex-wrap items-center gap-2">
				<Badge variant="secondary" className="rounded-md">
					{t(`wardrobe.type.${texture.texture_type}`)}
				</Badge>
				{texture.texture_type === "skin" ? (
					<Badge variant="outline" className="rounded-md">
						{texture.texture_model}
					</Badge>
				) : null}
				<Badge variant="outline" className="rounded-md">
					{texture.width}x{texture.height}
				</Badge>
				<Badge variant="outline" className="rounded-md">
					{formatBytes(texture.file_size)}
				</Badge>
				<Badge
					variant="outline"
					className={cn(
						"rounded-md",
						textureVisibilityBadgeClass(texture.visibility),
					)}
				>
					{t(`wardrobe.visibility.${texture.visibility}`)}
				</Badge>
				<LibraryStatusBadge status={texture.library_status} />
			</div>
			<TextureTagChips tags={texture.tags} />
		</div>
	);
}

export function textureLabel(texture: MinecraftWardrobeTextureMetadata) {
	return texture.name;
}

export function textureVisibilityBadgeClass(
	visibility: MinecraftWardrobeTextureMetadata["visibility"],
) {
	return visibility === "public"
		? "border-emerald-500/35 bg-emerald-500/10 text-emerald-700 dark:border-emerald-400/35 dark:bg-emerald-400/10 dark:text-emerald-300"
		: "border-border/80 bg-muted/70 text-muted-foreground";
}
