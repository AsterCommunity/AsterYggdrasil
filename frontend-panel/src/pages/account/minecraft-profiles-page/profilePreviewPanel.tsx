import { useTranslation } from "react-i18next";
import type { MinecraftProfilesPageController } from "@/components/account/profiles-page/useMinecraftProfilesPageController";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";

export function ProfilePreviewPanel({
	controller,
}: {
	controller: MinecraftProfilesPageController;
}) {
	const { t } = useTranslation();
	const { capeTexture, model, selectedProfile, skinTexture } = controller;

	return (
		<aside className="min-w-0">
			<MinecraftPreviewPanel
				label={t("profiles.previewPanelTitle")}
				playerName={selectedProfile?.name}
				skinUrl={skinTexture?.url ?? null}
				capeUrl={capeTexture?.url ?? null}
				model={skinTexture?.texture_model ?? model}
				emptyTitle={t("profiles.previewEmptyTitle")}
				emptyDescription={t("profiles.previewEmptyDescription")}
				failedTitle={t("profiles.previewFailedTitle")}
				failedDescription={t("profiles.previewFailedDescription")}
				noSkinLabel={t("profiles.noSkinTexture")}
				idleLabel={t("profiles.motionIdle")}
				walkLabel={t("profiles.motionWalk")}
				className="w-full"
				frameClassName="lg:h-[42rem]"
			/>
		</aside>
	);
}
