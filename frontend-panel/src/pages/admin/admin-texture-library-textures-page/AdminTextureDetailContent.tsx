import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { ADMIN_TABLE_MONO_TEXT_CLASS } from "@/components/common/AdminTable";
import { DateTimeText } from "@/components/common/DateTimeText";
import { RefreshButton } from "@/components/common/RefreshButton";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";
import { AdminSurface } from "@/components/layout/AdminSurface";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { Skeleton } from "@/components/ui/skeleton";
import { MinecraftPreviewPanel } from "@/components/yggdrasil/MinecraftPreviewPanel";
import { TextureLibraryTextureAvatar } from "@/components/yggdrasil/TextureLibraryTextureAvatar";
import { TextureTagChips } from "@/components/yggdrasil/TextureTagList";
import { formatBytes } from "@/lib/numberUnit";
import { cn } from "@/lib/utils";
import { adminPaths } from "@/routes/routePaths";
import type { LibraryTexture } from "./libraryTextureView";
import { LibraryStatusBadge, textureDisplayName } from "./libraryTextureView";

export function AdminTextureDetailContent({
	loading,
	onApprove,
	onDelete,
	onRefresh,
	onReject,
	onUnpublish,
	submitting,
	texture,
	validTextureId,
}: {
	loading: boolean;
	onApprove: (texture: LibraryTexture) => void;
	onDelete: (texture: LibraryTexture) => void;
	onRefresh: () => void;
	onReject: (texture: LibraryTexture) => void;
	onUnpublish: (texture: LibraryTexture) => void;
	submitting: boolean;
	texture: LibraryTexture | null;
	validTextureId: number | null;
}) {
	const { t } = useTranslation();

	if (!validTextureId) {
		return (
			<AdminSurface>
				<div className="space-y-2">
					<h1 className="text-lg font-semibold">
						{t("admin.textureLibraryDetailPage.title")}
					</h1>
					<p className="text-sm text-muted-foreground">
						{t("admin.textureLibraryDetailPage.invalidId")}
					</p>
				</div>
			</AdminSurface>
		);
	}

	const canApprove =
		texture?.visibility === "public" &&
		texture.library_status === "pending_review";
	const canUnpublish = texture?.library_status === "published";

	return (
		<>
			<AdminPageHeader
				title={
					texture
						? textureDisplayName(texture)
						: t("admin.textureLibraryDetailPage.title")
				}
				description={t("admin.textureLibraryDetailPage.description")}
				actions={
					<>
						<Button
							type="button"
							variant="outline"
							size="sm"
							render={<Link to={adminPaths.textureLibrary} />}
						>
							<Icon name="ArrowLeft" className="size-4" />
							{t("admin.textureLibraryDetailPage.backToLibrary")}
						</Button>
						<RefreshButton loading={loading} onClick={onRefresh} />
					</>
				}
			/>

			{loading ? (
				<div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.72fr)]">
					<Skeleton className="h-[34rem] rounded-lg" />
					<Skeleton className="h-[34rem] rounded-lg" />
				</div>
			) : !texture ? (
				<AdminSurface>
					<div className="space-y-2">
						<h2 className="text-lg font-semibold">
							{t("admin.textureLibraryDetailPage.notFoundTitle")}
						</h2>
						<p className="text-sm text-muted-foreground">
							{t("admin.textureLibraryDetailPage.notFoundDescription")}
						</p>
					</div>
				</AdminSurface>
			) : (
				<div className="grid items-start gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.72fr)]">
					<AdminSurface padded={false} className="min-w-0 overflow-hidden">
						<TextureDetailHeader texture={texture} />
						<div className="grid gap-5 p-4 sm:p-5">
							<TextureRecordSection texture={texture} />
							<TextureReviewSection texture={texture} />
						</div>
					</AdminSurface>

					<aside className="grid min-w-0 max-w-full gap-3 xl:sticky xl:top-20 xl:self-start">
						<MinecraftPreviewPanel
							label={t("admin.textureLibraryDetailPage.preview")}
							playerName={textureDisplayName(texture)}
							skinUrl={texture.texture_type === "skin" ? texture.url : null}
							capeUrl={texture.texture_type === "cape" ? texture.url : null}
							model={texture.texture_model}
							emptyTitle={t("profiles.previewEmptyTitle")}
							emptyDescription={t("profiles.previewEmptyDescription")}
							failedTitle={t("profiles.previewFailedTitle")}
							failedDescription={t("profiles.previewFailedDescription")}
							noSkinLabel={t("profiles.noSkinTexture")}
							idleLabel={t("profiles.motionIdle")}
							walkLabel={t("profiles.motionWalk")}
							frameClassName="h-[34rem]"
							skeletonClassName="h-[38rem]"
						/>
						<TextureReviewActions
							canApprove={canApprove}
							canUnpublish={canUnpublish}
							submitting={submitting}
							texture={texture}
							onApprove={onApprove}
							onDelete={onDelete}
							onReject={onReject}
							onUnpublish={onUnpublish}
						/>
					</aside>
				</div>
			)}
		</>
	);
}

function TextureDetailHeader({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	return (
		<div className="border-b border-border/70 bg-muted/16 px-4 py-4 dark:border-white/10 dark:bg-white/4 sm:px-5">
			<div className="flex min-w-0 items-start gap-3">
				<TextureLibraryTextureAvatar
					texture={texture}
					className="size-12 rounded-lg"
					testId={`admin-texture-detail-avatar-${texture.id}`}
					imageTestId={`admin-texture-detail-avatar-image-${texture.id}`}
				/>
				<div className="min-w-0">
					<div className="flex min-w-0 flex-wrap items-center gap-2">
						<h2 className="break-words text-lg font-semibold text-foreground">
							{textureDisplayName(texture)}
						</h2>
						<LibraryStatusBadge status={texture.library_status} />
					</div>
					<p className="mt-1 break-all font-mono text-xs text-muted-foreground">
						{texture.hash}
					</p>
					<div className="mt-2 flex flex-wrap gap-1.5">
						<Badge variant="outline" className="rounded-md">
							{t(`wardrobe.type.${texture.texture_type}`)}
						</Badge>
						{texture.texture_type === "skin" ? (
							<Badge variant="outline" className="rounded-md">
								{texture.texture_model}
							</Badge>
						) : null}
						<Badge variant="outline" className="rounded-md">
							{t(`wardrobe.visibility.${texture.visibility}`)}
						</Badge>
					</div>
				</div>
			</div>
		</div>
	);
}

function TextureRecordSection({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	return (
		<section className="grid gap-3">
			<div>
				<h3 className="text-base font-semibold text-foreground">
					{t("admin.textureLibraryDetailPage.recordTitle")}
				</h3>
				<p className="mt-1 text-sm leading-6 text-muted-foreground">
					{t("admin.textureLibraryDetailPage.recordDescription")}
				</p>
			</div>
			<div className="grid min-w-0 gap-3 md:grid-cols-2 xl:grid-cols-3">
				<TextureUploaderTile texture={texture} />
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.textureId")}
					value={`#${texture.id}`}
					mono
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.textureName")}
					value={texture.name}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.hash")}
					value={texture.hash}
					mono
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.dimensions")}
					value={`${texture.width}x${texture.height}`}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.fileSize")}
					value={formatBytes(texture.file_size)}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.mimeType")}
					value={texture.mime_type}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.createdAt")}
					value={<DateTimeText value={texture.created_at} />}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryDetailPage.updatedAt")}
					value={<DateTimeText value={texture.updated_at} />}
				/>
			</div>
			<div className="grid gap-1.5">
				<div className="text-xs font-medium text-muted-foreground">
					{t("admin.textureLibraryDetailPage.tags")}
				</div>
				{texture.tags.length > 0 ? (
					<TextureTagChips tags={texture.tags} />
				) : (
					<div className="text-sm text-muted-foreground">
						{t("admin.textureLibraryDetailPage.noTags")}
					</div>
				)}
			</div>
		</section>
	);
}

function TextureUploaderTile({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	const uploader = texture.uploader;

	return (
		<div className="min-w-0 rounded-lg border border-border/70 bg-background/60 p-3">
			<p className="text-xs uppercase tracking-wide text-muted-foreground">
				{t("admin.textureLibraryDetailPage.uploader")}
			</p>
			{uploader ? (
				<div className="mt-2 flex min-w-0 items-center gap-3 rounded-md">
					<UserAvatarImage
						avatar={uploader.avatar}
						name={uploader.name}
						alt=""
						size="sm"
						className="rounded-lg"
					/>
					<div className="min-w-0">
						<div className="truncate text-sm font-medium text-foreground">
							{uploader.name}
						</div>
						<div className="mt-1 truncate text-xs text-muted-foreground">
							@{uploader.username} · #{uploader.id}
						</div>
					</div>
				</div>
			) : (
				<p className="mt-2 text-sm text-muted-foreground">
					{t("admin.textureLibraryTexturesPage.unknownUploader")}
				</p>
			)}
		</div>
	);
}

function TextureReviewSection({ texture }: { texture: LibraryTexture }) {
	const { t } = useTranslation();
	return (
		<section className="grid gap-3">
			<div>
				<h3 className="text-base font-semibold text-foreground">
					{t("admin.textureLibraryDetailPage.reviewTitle")}
				</h3>
				<p className="mt-1 text-sm leading-6 text-muted-foreground">
					{t("admin.textureLibraryDetailPage.reviewDescription")}
				</p>
			</div>
			<div className="grid min-w-0 gap-3 md:grid-cols-3">
				<TextureInfoTile
					label={t("admin.textureLibraryTexturesPage.status")}
					value={<LibraryStatusBadge status={texture.library_status} />}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryTexturesPage.submittedAt")}
					value={<DateTimeText value={texture.library_submitted_at} />}
				/>
				<TextureInfoTile
					label={t("admin.textureLibraryTexturesPage.reviewedAt")}
					value={<DateTimeText value={texture.library_reviewed_at} />}
				/>
			</div>
			<div className="rounded-lg border border-border/70 bg-background/60 p-3">
				<div className="text-xs font-medium text-muted-foreground">
					{t("admin.textureLibraryTexturesPage.reviewNote")}
				</div>
				<div className="mt-2 text-sm leading-6">
					{texture.library_review_note?.trim() || "-"}
				</div>
			</div>
		</section>
	);
}

function TextureReviewActions({
	canApprove,
	canUnpublish,
	onApprove,
	onDelete,
	onReject,
	onUnpublish,
	submitting,
	texture,
}: {
	canApprove: boolean;
	canUnpublish: boolean;
	onApprove: (texture: LibraryTexture) => void;
	onDelete: (texture: LibraryTexture) => void;
	onReject: (texture: LibraryTexture) => void;
	onUnpublish: (texture: LibraryTexture) => void;
	submitting: boolean;
	texture: LibraryTexture;
}) {
	const { t } = useTranslation();
	return (
		<div className="grid gap-3 rounded-lg border border-border/70 bg-card/95 p-4 shadow-xs dark:border-white/10">
			<div className="flex min-w-0 flex-wrap items-center gap-2">
				<Badge variant="outline" className="rounded-md">
					{t(`wardrobe.visibility.${texture.visibility}`)}
				</Badge>
				<LibraryStatusBadge status={texture.library_status} />
			</div>
			<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-end">
				<Button
					type="button"
					variant="outline"
					disabled={!canApprove || submitting}
					onClick={() => onApprove(texture)}
				>
					{t("admin.textureLibraryTexturesPage.approveAction")}
				</Button>
				<Button
					type="button"
					variant="destructive"
					disabled={!canApprove || submitting}
					onClick={() => onReject(texture)}
				>
					{t("admin.textureLibraryTexturesPage.rejectAction")}
				</Button>
				<Button
					type="button"
					variant="outline"
					disabled={!canUnpublish || submitting}
					onClick={() => onUnpublish(texture)}
				>
					{t("admin.textureLibraryTexturesPage.unpublishAction")}
				</Button>
				<Button
					type="button"
					variant="destructive"
					disabled={submitting}
					onClick={() => onDelete(texture)}
				>
					<Icon name="Trash" className="size-4" />
					{t("common.delete")}
				</Button>
			</div>
		</div>
	);
}

function TextureInfoTile({
	label,
	mono,
	value,
}: {
	label: string;
	mono?: boolean;
	value: ReactNode;
}) {
	return (
		<div className="min-w-0 rounded-lg border border-border/70 bg-background/60 p-3">
			<p className="text-xs uppercase tracking-wide text-muted-foreground">
				{label}
			</p>
			<div
				className={cn(
					"mt-2 min-w-0 break-words text-sm font-medium text-foreground",
					mono && ADMIN_TABLE_MONO_TEXT_CLASS,
				)}
			>
				{value}
			</div>
		</div>
	);
}
