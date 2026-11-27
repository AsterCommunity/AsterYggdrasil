import type { KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import {
	ADMIN_TABLE_MUTED_TEXT_CLASS,
	AdminTableCell,
	AdminTableHead,
	AdminTableHeader,
	AdminTableRow,
} from "@/components/common/AdminTable";
import { DateTimeText } from "@/components/common/DateTimeText";
import { UserAvatarImage } from "@/components/common/UserAvatarImage";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Icon } from "@/components/ui/icon";
import { TextureLibraryTextureAvatar } from "@/components/yggdrasil/TextureLibraryTextureAvatar";
import type { LibraryTexture } from "./libraryTextureView";
import { LibraryStatusBadge, textureDisplayName } from "./libraryTextureView";

export function TextureTableHeader() {
	const { t } = useTranslation();
	return (
		<AdminTableHeader>
			<AdminTableRow>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.texture")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.uploader")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.visibility")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.status")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.submittedAt")}
				</AdminTableHead>
				<AdminTableHead>
					{t("admin.textureLibraryTexturesPage.reviewedAt")}
				</AdminTableHead>
				<AdminTableHead className="text-right">
					{t("admin.textureLibraryTexturesPage.actions")}
				</AdminTableHead>
			</AdminTableRow>
		</AdminTableHeader>
	);
}

export function TextureTableRow({
	loading,
	onApprove,
	onDelete,
	onOpen,
	onReject,
	onUnpublish,
	texture,
}: {
	loading: boolean;
	onApprove: () => void;
	onDelete: () => void;
	onOpen: () => void;
	onReject: () => void;
	onUnpublish: () => void;
	texture: LibraryTexture;
}) {
	const { t } = useTranslation();
	const canApprove =
		texture.visibility === "public" &&
		texture.library_status === "pending_review";
	const canUnpublish = texture.library_status === "published";
	const handleRowKeyDown = (event: KeyboardEvent<HTMLTableRowElement>) => {
		if (event.defaultPrevented) return;
		if (event.key === "Enter" || event.key === " ") {
			event.preventDefault();
			onOpen();
		}
	};

	return (
		<AdminTableRow
			tabIndex={0}
			className="cursor-pointer outline-none focus-visible:ring-3 focus-visible:ring-ring/30"
			onClick={onOpen}
			onKeyDown={handleRowKeyDown}
		>
			<AdminTableCell className="min-w-56">
				<div className="flex min-w-0 items-center gap-3">
					<TextureLibraryTextureAvatar
						texture={texture}
						testId={`admin-texture-preview-${texture.id}`}
						imageTestId={`admin-texture-preview-image-${texture.id}`}
						className="size-10"
					/>
					<div className="min-w-0">
						<div className="truncate font-medium">
							{textureDisplayName(texture)}
						</div>
						<div className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
							#{texture.id} · {t(`wardrobe.type.${texture.texture_type}`)} ·{" "}
							{texture.width}x{texture.height}
						</div>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<div className="flex min-w-0 items-center gap-2.5">
					{texture.uploader ? (
						<UserAvatarImage
							avatar={texture.uploader.avatar}
							name={texture.uploader.name}
							alt=""
							size="sm"
							className="rounded-lg"
						/>
					) : null}
					<div className="min-w-0">
						<div className="truncate">
							{texture.uploader?.name ??
								t("admin.textureLibraryTexturesPage.unknownUploader")}
						</div>
						<div className={ADMIN_TABLE_MUTED_TEXT_CLASS}>
							{texture.uploader
								? `@${texture.uploader.username} · #${texture.uploader.id}`
								: "-"}
						</div>
					</div>
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<Badge variant="outline" className="rounded-md">
					{t(`wardrobe.visibility.${texture.visibility}`)}
				</Badge>
			</AdminTableCell>
			<AdminTableCell>
				<div className="grid gap-1">
					<LibraryStatusBadge status={texture.library_status} />
					{texture.library_review_note ? (
						<div className="max-w-56 truncate text-xs text-muted-foreground">
							{texture.library_review_note}
						</div>
					) : null}
				</div>
			</AdminTableCell>
			<AdminTableCell>
				<DateTimeText value={texture.library_submitted_at} />
			</AdminTableCell>
			<AdminTableCell>
				<DateTimeText value={texture.library_reviewed_at} />
			</AdminTableCell>
			<AdminTableCell className="text-right">
				<div className="flex justify-end gap-1.5">
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={!canApprove || loading}
						onClick={(event) => {
							event.stopPropagation();
							onApprove();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						{t("admin.textureLibraryTexturesPage.approveAction")}
					</Button>
					<Button
						type="button"
						size="sm"
						variant="destructive"
						disabled={!canApprove || loading}
						onClick={(event) => {
							event.stopPropagation();
							onReject();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						{t("admin.textureLibraryTexturesPage.rejectAction")}
					</Button>
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={!canUnpublish || loading}
						onClick={(event) => {
							event.stopPropagation();
							onUnpublish();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						{t("admin.textureLibraryTexturesPage.unpublishAction")}
					</Button>
					<Button
						type="button"
						size="sm"
						variant="destructive"
						disabled={loading}
						onClick={(event) => {
							event.stopPropagation();
							onDelete();
						}}
						onKeyDown={(event) => event.stopPropagation()}
					>
						<Icon name="Trash" className="size-4" />
						{t("common.delete")}
					</Button>
				</div>
			</AdminTableCell>
		</AdminTableRow>
	);
}
