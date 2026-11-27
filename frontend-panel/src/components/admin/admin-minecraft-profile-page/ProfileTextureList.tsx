import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { RefreshButton } from "@/components/common/RefreshButton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { adminTextureLibraryPath } from "@/routes/routePaths";
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
			<div className="flex min-w-0 flex-col items-stretch gap-3 sm:flex-row sm:items-center sm:justify-between">
				<div className="min-w-0">
					<h3 className="text-base font-semibold">
						{t("admin.minecraftProfilePage.textureList")}
					</h3>
					<p className="text-sm text-muted-foreground">
						{t("admin.minecraftProfilePage.textureListDescription")}
					</p>
				</div>
				<RefreshButton
					className="w-full sm:w-auto"
					loading={false}
					onClick={onRefresh}
				/>
			</div>

			<div className="grid gap-2">
				{textures.length ? (
					textures.map((texture) => {
						const isDefaultTexture = texture.source === "default";
						const textureName = textureDisplayName(texture);
						return (
							<div
								key={`${texture.texture_type}-${texture.hash}`}
								className="flex min-w-0 flex-col items-stretch gap-3 rounded-lg border border-border/70 bg-muted/20 px-3 py-3 sm:flex-row sm:items-center sm:justify-between"
							>
								<div className="min-w-0 max-w-full">
									<div className="flex flex-wrap items-center gap-2">
										<span className="min-w-0 truncate font-medium">
											{textureName}
										</span>
										<Badge
											variant="outline"
											className="min-w-0 rounded-md break-all font-mono text-xs"
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
									<p className="mt-1 break-all font-mono text-xs text-muted-foreground">
										{texture.hash}
									</p>
								</div>
								<div className="flex flex-wrap items-center gap-2 sm:justify-end">
									<Button
										type="button"
										variant="outline"
										size="sm"
										className="w-full sm:w-auto"
										render={
											isDefaultTexture ? (
												<Link
													to={texture.url}
													target="_blank"
													rel="noreferrer"
												/>
											) : (
												<Link
													to={adminTextureLibraryPath(texture.texture_id)}
												/>
											)
										}
									>
										<Icon
											name={isDefaultTexture ? "ArrowSquareOut" : "FileImage"}
											className="size-4"
										/>
										{t("admin.minecraftProfilePage.openTexture")}
									</Button>
									<Button
										type="button"
										variant="destructive"
										size="sm"
										className="w-full sm:w-auto"
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

function textureDisplayName(texture: MinecraftTextureMetadata) {
	return texture.display_name?.trim() || texture.name || texture.hash;
}
