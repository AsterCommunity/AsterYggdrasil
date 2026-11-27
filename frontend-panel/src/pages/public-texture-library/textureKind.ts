import type { useTranslation } from "react-i18next";
import type { PublicTextureLibraryTextureMetadata } from "@/types/api";

export function formatTextureKind(
	texture: Pick<
		PublicTextureLibraryTextureMetadata,
		"texture_model" | "texture_type"
	>,
	t: ReturnType<typeof useTranslation>["t"],
) {
	if (texture.texture_type !== "skin") {
		return t(`wardrobe.type.${texture.texture_type}`);
	}
	return `${t("wardrobe.type.skin")} / ${
		texture.texture_model === "slim"
			? t("profiles.slimModel")
			: t("profiles.defaultModel")
	}`;
}
