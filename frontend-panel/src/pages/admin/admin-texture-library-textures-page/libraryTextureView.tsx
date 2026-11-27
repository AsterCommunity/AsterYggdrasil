import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type {
	MinecraftTextureLibraryStatus,
	PublicTextureLibraryTextureMetadata,
} from "@/types/api";

export const TEXTURE_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
export const DEFAULT_TEXTURE_PAGE_SIZE = 20;
export const ALL_VALUE = "__all__";

export type TexturePageMode = "all" | "detail" | "reviews";
export type TexturePageSize = (typeof TEXTURE_PAGE_SIZE_OPTIONS)[number];
export type LibraryTexture = PublicTextureLibraryTextureMetadata;
export type ReviewAction = "approve" | "reject" | "unpublish";

export type ReviewDialogState = {
	action: ReviewAction;
	texture: LibraryTexture;
};

export type DeleteDialogState = {
	texture: LibraryTexture;
};

export type FilterValue<T extends string> = T | typeof ALL_VALUE;
export type PublishedFilter = "published" | "not_published" | typeof ALL_VALUE;
export type TextureCursor = { id: number; value: string };

export function publishedToQuery(value: PublishedFilter): boolean | undefined {
	if (value === "published") return true;
	if (value === "not_published") return false;
	return undefined;
}

export function parseTextureId(value: string | undefined) {
	const parsed = Number(value);
	return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
}

export function textureDisplayName(texture: LibraryTexture) {
	return texture.display_name?.trim() || texture.name;
}

export function LibraryStatusBadge({
	status,
}: {
	status: MinecraftTextureLibraryStatus;
}) {
	const { t } = useTranslation();
	return (
		<Badge
			variant={status === "published" ? "default" : "outline"}
			className={cn(
				"w-fit rounded-md",
				status === "pending_review" &&
					"border-amber-500/35 bg-amber-500/10 text-amber-700 dark:text-amber-300",
				status === "rejected" &&
					"border-destructive/35 bg-destructive/10 text-destructive",
			)}
		>
			{t(`admin.textureLibraryTexturesPage.libraryStatus.${status}`)}
		</Badge>
	);
}

export function libraryStatusOptions(
	t: ReturnType<typeof useTranslation>["t"],
) {
	return (
		[
			"private",
			"pending_review",
			"published",
			"rejected",
		] as const satisfies readonly MinecraftTextureLibraryStatus[]
	).map((status) => ({
		label: t(`admin.textureLibraryTexturesPage.libraryStatus.${status}`),
		value: status,
	}));
}
