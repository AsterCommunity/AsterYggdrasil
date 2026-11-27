import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";
import type { MinecraftWardrobeTextureMetadata } from "@/types/api";
import { LibraryStatusBadge, TextureSummary } from "./WardrobeTextureSummary";

export function WardrobePreviewPanel({
	libraryConfig,
	onBind,
	onDelete,
	onEdit,
	onSubmitLibrary,
	onWithdrawLibrary,
	submitting,
	texture,
	total,
}: {
	libraryConfig: { enabled: boolean; review_required: boolean };
	onBind: () => void;
	onDelete: () => void;
	onEdit: () => void;
	onSubmitLibrary: () => void;
	onWithdrawLibrary: () => void;
	submitting: boolean;
	texture: MinecraftWardrobeTextureMetadata | null;
	total: number;
}) {
	const { t } = useTranslation();
	const skinUrl = texture?.texture_type === "skin" ? texture.url : null;
	const capeUrl = texture?.texture_type === "cape" ? texture.url : null;
	const libraryAction = texture
		? textureLibraryAction(texture, libraryConfig)
		: null;

	return (
		<aside className="grid min-w-0 gap-3 xl:sticky xl:top-20 xl:self-start">
			<MinecraftPreviewPanel
				label={t("wardrobe.previewTitle")}
				playerName={texture ? t(`wardrobe.type.${texture.texture_type}`) : null}
				skinUrl={skinUrl}
				capeUrl={capeUrl}
				model={texture?.texture_model ?? "default"}
				emptyTitle={t("wardrobe.previewEmptyTitle")}
				emptyDescription={t("wardrobe.previewEmptyDescription")}
				failedTitle={t("profiles.previewFailedTitle")}
				failedDescription={t("profiles.previewFailedDescription")}
				noSkinLabel={t("wardrobe.totalTextures", {
					count: total.toString(),
				})}
				idleLabel={t("profiles.motionIdle")}
				walkLabel={t("profiles.motionWalk")}
				frameClassName="h-[34rem]"
				skeletonClassName="h-[38rem]"
			/>
			<div className="grid gap-3 rounded-lg border border-border/70 bg-card/95 p-4 shadow-xs">
				{texture ? <TextureSummary texture={texture} /> : null}
				{texture ? (
					<div className="grid gap-2 rounded-lg border border-border/70 bg-muted/20 p-3 text-sm">
						<div className="flex flex-wrap items-center justify-between gap-2">
							<div className="font-medium">
								{t("wardrobe.libraryStatusTitle")}
							</div>
							<LibraryStatusBadge status={texture.library_status} />
						</div>
						<p className="text-xs leading-5 text-muted-foreground">
							{libraryStatusDescription(texture, libraryConfig, t)}
						</p>
						{texture.library_review_note ? (
							<div className="rounded-md border border-border/70 bg-background/80 px-2.5 py-2 text-xs text-muted-foreground">
								<span className="font-medium text-foreground">
									{t("wardrobe.libraryReviewNote")}
								</span>{" "}
								{texture.library_review_note}
							</div>
						) : null}
						{libraryAction ? (
							<div className="flex flex-wrap gap-2">
								<Button
									type="button"
									size="sm"
									variant={libraryAction.variant}
									disabled={submitting || libraryAction.disabled}
									onClick={
										libraryAction.kind === "withdraw"
											? onWithdrawLibrary
											: onSubmitLibrary
									}
								>
									{submitting ? (
										<Icon name="Spinner" className="size-4 animate-spin" />
									) : null}
									{t(libraryAction.labelKey)}
								</Button>
							</div>
						) : null}
					</div>
				) : null}
				<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
					<div className="flex flex-wrap gap-2">
						<Button type="button" disabled={!texture} onClick={onBind}>
							{t("wardrobe.bindToProfile")}
						</Button>
						<Button
							type="button"
							variant="outline"
							disabled={!texture}
							onClick={onEdit}
						>
							<Icon name="PencilSimple" className="mr-2 size-4" />
							{t("wardrobe.editAction")}
						</Button>
					</div>
					<Button
						type="button"
						variant="destructive"
						disabled={!texture}
						onClick={onDelete}
					>
						{t("wardrobe.deleteAction")}
					</Button>
				</div>
			</div>
		</aside>
	);
}

function textureLibraryAction(
	texture: MinecraftWardrobeTextureMetadata,
	config: { enabled: boolean; review_required: boolean },
): {
	disabled: boolean;
	kind: "submit" | "withdraw";
	labelKey: string;
	variant: "default" | "outline";
} | null {
	if (!config.enabled) return null;
	if (texture.library_status === "pending_review") {
		return {
			disabled: false,
			kind: "withdraw",
			labelKey: "wardrobe.libraryWithdrawAction",
			variant: "outline",
		};
	}
	if (texture.library_status === "published") {
		return {
			disabled: false,
			kind: "withdraw",
			labelKey: "wardrobe.libraryUnpublishAction",
			variant: "outline",
		};
	}
	return {
		disabled: texture.visibility !== "public",
		kind: "submit",
		labelKey: config.review_required
			? "wardrobe.librarySubmitReviewAction"
			: "wardrobe.libraryPublishAction",
		variant: "default",
	};
}

function libraryStatusDescription(
	texture: MinecraftWardrobeTextureMetadata,
	config: { enabled: boolean; review_required: boolean },
	t: ReturnType<typeof useTranslation>["t"],
) {
	if (!config.enabled) {
		return t("wardrobe.libraryStatusDescription.disabled");
	}
	if (texture.visibility !== "public") {
		return t("wardrobe.libraryStatusDescription.privateVisibility");
	}
	if (texture.library_status === "pending_review") {
		return t("wardrobe.libraryStatusDescription.pending");
	}
	if (texture.library_status === "published") {
		return t("wardrobe.libraryStatusDescription.published");
	}
	if (texture.library_status === "rejected") {
		return t("wardrobe.libraryStatusDescription.rejected");
	}
	return config.review_required
		? t("wardrobe.libraryStatusDescription.readyForReview")
		: t("wardrobe.libraryStatusDescription.readyToPublish");
}
