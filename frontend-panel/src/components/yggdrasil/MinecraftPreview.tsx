import { useEffect, useRef, useState } from "react";
import {
	IdleAnimation,
	type SkinLoadOptions,
	SkinViewer,
	WalkingAnimation,
} from "skinview3d";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { MinecraftTextureModel } from "@/types/api";

export type PreviewMotion = "idle" | "walk";

export type MinecraftPreviewProps = {
	capeUrl?: string | null;
	className?: string;
	compactHeader?: boolean;
	emptyDescription?: string;
	emptyTitle?: string;
	failedDescription?: string;
	failedTitle?: string;
	frameClassName?: string;
	idleLabel?: string;
	initialMotion?: PreviewMotion;
	label: string;
	model?: MinecraftTextureModel;
	noSkinLabel?: string;
	playerName?: string | null;
	skinUrl?: string | null;
	walkLabel?: string;
};

export function MinecraftPreview({
	capeUrl,
	className,
	compactHeader = false,
	emptyDescription = "PNG skins render here with rotation and idle animation.",
	emptyTitle = "Upload a skin to preview",
	failedDescription = "The texture URL could not be loaded by the 3D viewer.",
	failedTitle = "Preview failed",
	frameClassName,
	idleLabel = "Idle",
	initialMotion = "idle",
	label,
	model = "default",
	noSkinLabel = "No skin texture",
	playerName,
	skinUrl,
	walkLabel = "Walk",
}: MinecraftPreviewProps) {
	const canvasRef = useRef<HTMLCanvasElement | null>(null);
	const frameRef = useRef<HTMLDivElement | null>(null);
	const viewerRef = useRef<SkinViewer | null>(null);
	const skinKey = skinUrl ? `${model}:${skinUrl}` : null;
	const [failedSkinKey, setFailedSkinKey] = useState<string | null>(null);
	const [motion, setMotion] = useState<PreviewMotion>(initialMotion);
	const failed = skinKey !== null && failedSkinKey === skinKey;
	const hasTexture = Boolean(skinUrl || capeUrl);

	useEffect(() => {
		if (!canvasRef.current || !frameRef.current) return;

		let disposed = false;
		const frame = frameRef.current;
		const canvas = canvasRef.current;
		const rect = frame.getBoundingClientRect();
		const viewer = new SkinViewer({
			canvas,
			width: Math.max(280, Math.round(rect.width)),
			height: Math.max(360, Math.round(rect.height)),
			fov: 42,
			zoom: 0.82,
			enableControls: true,
		});
		viewer.autoRotate = true;
		viewer.autoRotateSpeed = 0.45;
		viewer.controls.enablePan = false;
		viewer.controls.enableZoom = false;
		viewer.controls.rotateSpeed = 0.55;
		viewerRef.current = viewer;

		const observer = new ResizeObserver(([entry]) => {
			if (!entry || disposed || viewer.disposed) return;
			const { width, height } = entry.contentRect;
			viewer.setSize(Math.max(240, Math.round(width)), Math.round(height));
		});
		observer.observe(frame);

		return () => {
			disposed = true;
			observer.disconnect();
			viewer.dispose();
			viewerRef.current = null;
		};
	}, []);

	useEffect(() => {
		const viewer = viewerRef.current;
		if (!viewer) return;
		const animation =
			motion === "walk" ? new WalkingAnimation() : new IdleAnimation();
		animation.speed = motion === "walk" ? 0.78 : 0.9;
		viewer.animation = animation;
	}, [motion]);

	useEffect(() => {
		const viewer = viewerRef.current;
		if (!viewer) return;
		let cancelled = false;
		const loadingSkinKey = skinKey;

		const options: SkinLoadOptions = {
			model: model === "slim" ? "slim" : "default",
			ears: "load-only",
		};

		if (!skinUrl) {
			viewer.loadSkin(null);
			return;
		}

		void viewer.loadSkin(skinUrl, options).then(
			() => {
				if (cancelled) return;
				setFailedSkinKey((current) =>
					current === loadingSkinKey ? null : current,
				);
			},
			() => {
				if (cancelled || loadingSkinKey === null) return;
				setFailedSkinKey(loadingSkinKey);
				viewer.loadSkin(null);
			},
		);
		return () => {
			cancelled = true;
		};
	}, [skinUrl, model, skinKey]);

	useEffect(() => {
		const viewer = viewerRef.current;
		if (!viewer) return;

		if (!capeUrl) {
			viewer.loadCape(null);
			return;
		}
		void viewer.loadCape(capeUrl).catch(() => {
			viewer.loadCape(null);
		});
	}, [capeUrl]);

	return (
		<div
			className={cn(
				"w-full overflow-hidden rounded-lg border border-border/70 bg-card shadow-xs",
				className,
			)}
		>
			<div
				className={cn(
					"flex min-h-12 gap-3 border-b border-border/70 px-4 py-3",
					compactHeader
						? "flex-row items-center justify-between"
						: "flex-col sm:flex-row sm:items-center sm:justify-between",
				)}
			>
				<div className="min-w-0">
					<div className="truncate text-sm font-semibold">{label}</div>
					<div className="truncate text-xs text-muted-foreground">
						{playerName || noSkinLabel}
					</div>
				</div>
				<div className="flex w-fit shrink-0 rounded-lg border border-border/70 bg-muted/30 p-1">
					<Button
						type="button"
						size="sm"
						variant={motion === "idle" ? "default" : "ghost"}
						onClick={() => setMotion("idle")}
					>
						{idleLabel}
					</Button>
					<Button
						type="button"
						size="sm"
						variant={motion === "walk" ? "default" : "ghost"}
						onClick={() => setMotion("walk")}
					>
						{walkLabel}
					</Button>
				</div>
			</div>
			<div
				ref={frameRef}
				className={cn(
					"relative h-[26rem] bg-[radial-gradient(circle_at_50%_18%,oklch(0.92_0.024_151_/_0.75),transparent_42%),linear-gradient(180deg,oklch(0.96_0.004_255),oklch(0.9_0.01_255))] dark:bg-[radial-gradient(circle_at_50%_18%,oklch(0.32_0.06_151_/_0.5),transparent_42%),linear-gradient(180deg,oklch(0.2_0.02_255),oklch(0.17_0.018_255))]",
					frameClassName,
				)}
			>
				<canvas ref={canvasRef} className="block size-full" />
				{hasTexture && !failed ? null : (
					<div className="absolute inset-0 grid place-items-center p-6 text-center">
						<div className="rounded-lg border border-border/70 bg-background/82 px-4 py-3 shadow-lg backdrop-blur">
							<div className="text-sm font-semibold">
								{failed ? failedTitle : emptyTitle}
							</div>
							<p className="mt-1 max-w-52 text-xs leading-5 text-muted-foreground">
								{failed ? failedDescription : emptyDescription}
							</p>
						</div>
					</div>
				)}
			</div>
		</div>
	);
}
