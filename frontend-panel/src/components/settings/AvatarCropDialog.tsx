import { type SyntheticEvent, useEffect, useRef, useState } from "react";
import ReactCrop, {
	centerCrop,
	convertToPixelCrop,
	type PercentCrop,
	type PixelCrop,
} from "react-image-crop";
import "react-image-crop/dist/ReactCrop.css";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Icon } from "@/components/ui/icon";
import { Skeleton } from "@/components/ui/skeleton";
import { cropAvatarImage, renderAvatarCropPreview } from "@/lib/avatarCrop";

type AvatarCropDialogProps = {
	open: boolean;
	file: File | null;
	busy?: boolean;
	onOpenChange: (open: boolean) => void;
	onConfirm: (file: File) => Promise<boolean>;
};

const CROPPER_SIZE_PERCENT = 64;
const MIN_CROP_SIZE = 88;
const PREVIEW_SIZE = 192;

function createCenteredAvatarCrop(
	containerWidth: number,
	containerHeight: number,
): PercentCrop {
	const baseCrop =
		containerWidth <= containerHeight
			? {
					unit: "%" as const,
					width: CROPPER_SIZE_PERCENT,
					height: (containerWidth / containerHeight) * CROPPER_SIZE_PERCENT,
				}
			: {
					unit: "%" as const,
					width: (containerHeight / containerWidth) * CROPPER_SIZE_PERCENT,
					height: CROPPER_SIZE_PERCENT,
				};

	return centerCrop(
		{ ...baseCrop, x: 0, y: 0 },
		containerWidth,
		containerHeight,
	);
}

export function AvatarCropDialog({
	open,
	file,
	busy = false,
	onOpenChange,
	onConfirm,
}: AvatarCropDialogProps) {
	if (!file) return null;

	const sessionKey = `${file.name}:${file.size}:${file.lastModified}`;

	return (
		<AvatarCropDialogSession
			key={sessionKey}
			open={open}
			file={file}
			busy={busy}
			onOpenChange={onOpenChange}
			onConfirm={onConfirm}
		/>
	);
}

function AvatarCropDialogSession({
	open,
	file,
	busy,
	onOpenChange,
	onConfirm,
}: Omit<AvatarCropDialogProps, "file"> & { file: File }) {
	const { t } = useTranslation();
	const imageRef = useRef<HTMLImageElement | null>(null);
	const previewCanvasRef = useRef<HTMLCanvasElement | null>(null);
	const [crop, setCrop] = useState<PercentCrop>();
	const [completedCrop, setCompletedCrop] = useState<PixelCrop | null>(null);
	const [imageUrl] = useState(() => URL.createObjectURL(file));
	const [processing, setProcessing] = useState(false);

	useEffect(() => {
		return () => URL.revokeObjectURL(imageUrl);
	}, [imageUrl]);

	useEffect(() => {
		if (
			!open ||
			!completedCrop ||
			!imageRef.current ||
			!previewCanvasRef.current
		) {
			return;
		}

		renderAvatarCropPreview(
			imageRef.current,
			previewCanvasRef.current,
			completedCrop,
			PREVIEW_SIZE,
		);
	}, [completedCrop, open]);

	const lockClose = busy || processing;

	const handleDialogOpenChange = (nextOpen: boolean) => {
		if (lockClose && !nextOpen) return;
		onOpenChange(nextOpen);
	};

	const handleImageLoad = (event: SyntheticEvent<HTMLImageElement>) => {
		const image = event.currentTarget;
		imageRef.current = image;

		const nextCrop = createCenteredAvatarCrop(image.width, image.height);
		setCrop(nextCrop);
		setCompletedCrop(convertToPixelCrop(nextCrop, image.width, image.height));
	};

	const handleReset = () => {
		const image = imageRef.current;
		if (!image) return;

		const nextCrop = createCenteredAvatarCrop(image.width, image.height);
		setCrop(nextCrop);
		setCompletedCrop(convertToPixelCrop(nextCrop, image.width, image.height));
	};

	const handleConfirm = async () => {
		if (!file || !completedCrop || !imageRef.current) return;

		try {
			setProcessing(true);
			const croppedFile = await cropAvatarImage(
				imageRef.current,
				file,
				completedCrop,
			);
			const shouldClose = await onConfirm(croppedFile);
			if (shouldClose) onOpenChange(false);
		} finally {
			setProcessing(false);
		}
	};

	return (
		<Dialog open={open} onOpenChange={handleDialogOpenChange}>
			<DialogContent className="flex max-h-[min(820px,calc(100dvh-2rem))] flex-col gap-0 overflow-hidden p-0 sm:max-w-[min(1040px,calc(100vw-2rem))]">
				<DialogHeader className="shrink-0 border-b border-border/70 px-6 pt-5 pb-4 pr-14">
					<DialogTitle>{t("personalSettings.avatarCropTitle")}</DialogTitle>
					<DialogDescription>
						{t("personalSettings.avatarCropDescription")}
					</DialogDescription>
				</DialogHeader>

				<div className="grid min-h-0 flex-1 gap-0 lg:grid-cols-[320px_minmax(0,1fr)]">
					<aside className="flex min-h-0 flex-col border-b border-border/70 bg-muted/15 lg:border-r lg:border-b-0">
						<div className="min-h-0 flex-1 space-y-5 overflow-auto p-6">
							<section className="rounded-lg border border-border/70 bg-background p-5">
								<p className="text-xs font-medium text-muted-foreground uppercase tracking-normal">
									{t("personalSettings.avatarCropPreview")}
								</p>
								<div className="mt-5 flex justify-center">
									<canvas
										ref={previewCanvasRef}
										className="size-48 rounded-full bg-muted ring-1 ring-border/45"
										aria-label={t("personalSettings.avatarCropPreview")}
									/>
								</div>
								<p className="mt-4 text-center text-xs text-muted-foreground">
									{t("personalSettings.avatarCropOutputHint", {
										size: "1024x1024",
									})}
								</p>
							</section>
						</div>

						<div className="shrink-0 border-t border-border/70 px-6 py-4">
							<Button
								type="button"
								variant="outline"
								className="w-full"
								disabled={lockClose || !crop}
								onClick={handleReset}
							>
								<Icon name="Undo" className="mr-1 size-4" />
								{t("personalSettings.avatarCropReset")}
							</Button>
						</div>
					</aside>

					<section className="flex min-h-0 flex-1 items-center justify-center overflow-auto bg-muted/10 p-6 md:p-8">
						{imageUrl ? (
							<ReactCrop
								crop={crop}
								aspect={1}
								circularCrop
								keepSelection
								minWidth={MIN_CROP_SIZE}
								minHeight={MIN_CROP_SIZE}
								className="max-w-full"
								onChange={(pixelCrop, percentCrop) => {
									setCrop(percentCrop);
									setCompletedCrop(pixelCrop);
								}}
							>
								<img
									src={imageUrl}
									alt=""
									draggable={false}
									onLoad={handleImageLoad}
									className="block h-auto max-h-[min(58vh,540px)] w-auto max-w-full select-none object-contain"
								/>
							</ReactCrop>
						) : (
							<div
								className="flex min-h-40 w-full items-center justify-center"
								role="status"
							>
								<span className="sr-only">{t("common.loading")}</span>
								<Skeleton className="h-40 w-full max-w-md rounded-lg" />
							</div>
						)}
					</section>
				</div>

				<DialogFooter className="mx-0 mb-0 w-full shrink-0 bg-muted/10 px-6 py-4 sm:flex-row sm:items-center sm:justify-end">
					<Button
						type="button"
						variant="outline"
						disabled={lockClose}
						onClick={() => onOpenChange(false)}
					>
						{t("common.cancel")}
					</Button>
					<Button
						type="button"
						disabled={lockClose || !completedCrop}
						onClick={() => void handleConfirm()}
					>
						{lockClose ? (
							<Icon name="Spinner" className="mr-1 size-4 animate-spin" />
						) : (
							<Icon name="Check" className="mr-1 size-4" />
						)}
						{t("personalSettings.avatarCropApply")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
