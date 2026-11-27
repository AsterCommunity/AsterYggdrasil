import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type {
	MinecraftWardrobeTextureMetadata,
	YggdrasilProfile,
} from "@/types/api";

export function WardrobeBindDialog({
	activeTexture,
	clearStateAfterClose,
	dialogOpen,
	filteredProfiles,
	onClose,
	onProfileQueryChange,
	onProfileSelect,
	onSubmitBind,
	profiles,
	profileQuery,
	selectedProfileId,
	submitting,
}: {
	activeTexture: MinecraftWardrobeTextureMetadata | null;
	clearStateAfterClose: () => void;
	dialogOpen: boolean;
	filteredProfiles: YggdrasilProfile[];
	onClose: (open: boolean) => void;
	onProfileQueryChange: (query: string) => void;
	onProfileSelect: (profileId: string) => void;
	onSubmitBind: () => void;
	profiles: YggdrasilProfile[];
	profileQuery: string;
	selectedProfileId: string;
	submitting: boolean;
}) {
	const { t } = useTranslation();

	return (
		<Dialog open={dialogOpen} onOpenChange={onClose}>
			<DialogContent keepMounted onAnimationEnd={clearStateAfterClose}>
				<DialogHeader>
					<DialogTitle>{t("wardrobe.bindDialogTitle")}</DialogTitle>
					<DialogDescription>
						{activeTexture
							? t("wardrobe.bindDialogDescription", {
									type: t(`wardrobe.type.${activeTexture.texture_type}`),
								})
							: t("wardrobe.bindDialogFallback")}
					</DialogDescription>
				</DialogHeader>

				<div className="grid gap-3">
					<Input
						value={profileQuery}
						placeholder={t("wardrobe.profileSearchPlaceholder")}
						onChange={(event) =>
							onProfileQueryChange(event.currentTarget.value)
						}
					/>

					<div className="max-h-72 overflow-y-auto rounded-lg border border-border/70">
						{filteredProfiles.length === 0 ? (
							<div className="px-4 py-8 text-center text-sm text-muted-foreground">
								{profiles.length === 0
									? t("wardrobe.noProfiles")
									: t("wardrobe.noProfileSearchResults")}
							</div>
						) : (
							<div className="divide-y divide-border/70">
								{filteredProfiles.map((profile) => (
									<button
										key={profile.id}
										type="button"
										className={cn(
											"grid w-full gap-1 px-3 py-3 text-left transition-colors hover:bg-accent/35",
											selectedProfileId === profile.id && "bg-accent/50",
										)}
										onClick={() => onProfileSelect(profile.id)}
									>
										<span className="flex min-w-0 items-center gap-2">
											<span className="truncate font-medium">
												{profile.name}
											</span>
											{selectedProfileId === profile.id ? (
												<Badge variant="outline" className="rounded-md">
													{t("profiles.selected")}
												</Badge>
											) : null}
										</span>
									</button>
								))}
							</div>
						)}
					</div>
				</div>

				<DialogFooter>
					<DialogClose
						render={
							<Button type="button" variant="outline" disabled={submitting} />
						}
					>
						{t("common.cancel")}
					</DialogClose>
					<Button
						type="button"
						disabled={!activeTexture || !selectedProfileId || submitting}
						onClick={onSubmitBind}
					>
						{submitting ? t("wardrobe.saving") : t("wardrobe.bindAction")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
