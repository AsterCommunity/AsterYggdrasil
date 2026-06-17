import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import type { MinecraftTextureMetadata } from "./types";

export function ProfileTextureList({
	deletingTexture,
	loading,
	textures,
	onRefresh,
	onSelectDelete,
}: {
	deletingTexture: boolean;
	loading: boolean;
	textures: MinecraftTextureMetadata[];
	onRefresh: () => void;
	onSelectDelete: (texture: MinecraftTextureMetadata) => void;
}) {
	const { t } = useTranslation();
	return (
		<>
			<div className="flex items-center justify-between gap-3">
				<div>
					<h3 className="text-base font-semibold">
						{t("admin.minecraftProfilePage.textureList")}
					</h3>
					<p className="text-sm text-muted-foreground">
						{t("admin.minecraftProfilePage.textureListDescription")}
					</p>
				</div>
				<Button type="button" variant="outline" size="sm" onClick={onRefresh}>
					<Icon name="ArrowsClockwise" className="size-4" />
					{t("common.refresh")}
				</Button>
			</div>

			<div className="grid gap-2">
				{textures.length ? (
					textures.map((texture) => {
						const isDefaultTexture = texture.source === "default";
						return (
							<div
								key={`${texture.texture_type}-${texture.hash}`}
								className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border/70 bg-muted/20 px-3 py-3"
							>
								<div className="min-w-0">
									<div className="flex flex-wrap items-center gap-2">
										<span className="font-medium">
											{texture.texture_type.toUpperCase()}
										</span>
										<Badge
											variant="outline"
											className="rounded-md font-mono text-xs"
										>
											{texture.texture_model}
										</Badge>
										<Badge
											variant="outline"
											className="rounded-md font-mono text-xs"
										>
											{isDefaultTexture ? texture.source : texture.visibility}
										</Badge>
									</div>
									<p className="mt-1 truncate font-mono text-xs text-muted-foreground">
										{texture.hash}
									</p>
								</div>
								<div className="flex flex-wrap items-center gap-2">
									<Button
										type="button"
										variant="outline"
										size="sm"
										render={
											<Link to={texture.url} target="_blank" rel="noreferrer" />
										}
									>
										<Icon name="ArrowSquareOut" className="size-4" />
										{t("admin.minecraftProfilePage.openTexture")}
									</Button>
									<Button
										type="button"
										variant="destructive"
										size="sm"
										disabled={deletingTexture || isDefaultTexture}
										onClick={() => onSelectDelete(texture)}
									>
										<Icon name="Trash" className="size-4" />
										{t("common.delete")}
									</Button>
								</div>
							</div>
						);
					})
				) : (
					<p className="text-sm text-muted-foreground">
						{loading
							? t("admin.minecraftProfilePage.loading")
							: t("admin.minecraftProfilePage.noTextures")}
					</p>
				)}
			</div>
		</>
	);
}
