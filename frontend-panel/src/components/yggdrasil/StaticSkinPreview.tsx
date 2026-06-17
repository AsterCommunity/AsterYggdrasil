import { useEffect, useRef, useState } from "react";
import { type SkinLoadOptions, SkinViewer } from "skinview3d";
import { cn } from "@/lib/utils";
import type { MinecraftTextureModel } from "@/types/api";

type StaticSkinPreviewProps = {
	alt: string;
	className?: string;
	model?: MinecraftTextureModel;
	skinUrl: string;
};

const STATIC_SKIN_YAW_OFFSET = Math.PI / 6;
const STATIC_SKIN_PITCH = Math.PI / 7;
const STATIC_SKIN_VIEWS = [
	{
		className: "translate-x-5",
		key: "back",
		rotationY: Math.PI - STATIC_SKIN_YAW_OFFSET,
	},
	{
		className: "-translate-x-5",
		key: "front",
		rotationY: -STATIC_SKIN_YAW_OFFSET,
	},
] as const;

export function StaticSkinPreview({
	alt,
	className,
	model = "default",
	skinUrl,
}: StaticSkinPreviewProps) {
	const previewKey = `${model}:${skinUrl}`;

	return (
		<div
			className={cn(
				"grid aspect-[5/4] grid-cols-2 overflow-hidden border-b border-border/70 bg-[linear-gradient(180deg,oklch(0.98_0.004_255),oklch(0.93_0.012_151))] dark:bg-[linear-gradient(180deg,oklch(0.22_0.022_255),oklch(0.17_0.018_255))]",
				className,
			)}
			aria-label={alt}
			role="img"
		>
			<StaticSkinCanvasGroup key={previewKey}>
				{STATIC_SKIN_VIEWS.map((view) => (
					<StaticSkinCanvas
						key={view.key}
						className={view.className}
						model={model}
						rotationY={view.rotationY}
						skinUrl={skinUrl}
					/>
				))}
			</StaticSkinCanvasGroup>
		</div>
	);
}

function StaticSkinCanvasGroup({ children }: { children: React.ReactNode }) {
	return children;
}

function StaticSkinCanvas({
	className,
	model,
	rotationY,
	skinUrl,
}: {
	className?: string;
	model: MinecraftTextureModel;
	rotationY: number;
	skinUrl: string;
}) {
	const canvasRef = useRef<HTMLCanvasElement | null>(null);
	const frameRef = useRef<HTMLDivElement | null>(null);
	const viewerRef = useRef<SkinViewer | null>(null);
	const [failed, setFailed] = useState(false);

	useEffect(() => {
		if (!canvasRef.current || !frameRef.current) return;

		let disposed = false;
		const frame = frameRef.current;
		const canvas = canvasRef.current;
		const rect = frame.getBoundingClientRect();
		const viewer = new SkinViewer({
			canvas,
			width: Math.max(120, Math.round(rect.width)),
			height: Math.max(170, Math.round(rect.height)),
			fov: 36,
			zoom: 0.78,
			enableControls: false,
			renderPaused: true,
		});
		viewer.autoRotate = false;
		viewer.animation = null;
		viewer.playerWrapper.rotation.x = STATIC_SKIN_PITCH;
		viewer.playerWrapper.rotation.y = rotationY;
		viewerRef.current = viewer;

		const render = () => {
			if (!disposed && !viewer.disposed) viewer.render();
		};
		const observer = new ResizeObserver(([entry]) => {
			if (!entry || disposed || viewer.disposed) return;
			const { width, height } = entry.contentRect;
			viewer.setSize(Math.max(120, Math.round(width)), Math.round(height));
			render();
		});
		observer.observe(frame);
		render();

		return () => {
			disposed = true;
			observer.disconnect();
			viewer.dispose();
			viewerRef.current = null;
		};
	}, [rotationY]);

	useEffect(() => {
		const viewer = viewerRef.current;
		if (!viewer) return;

		const options: SkinLoadOptions = {
			model: model === "slim" ? "slim" : "default",
			ears: "load-only",
		};

		void viewer.loadSkin(skinUrl, options).then(
			() => {
				viewer.render();
			},
			() => {
				setFailed(true);
				viewer.loadSkin(null);
				viewer.render();
			},
		);
	}, [skinUrl, model]);

	return (
		<div ref={frameRef} className={cn("relative min-h-44", className)}>
			<canvas
				ref={canvasRef}
				className="block size-full"
				data-slot="static-skin-preview-canvas"
			/>
			{failed ? (
				<div className="absolute inset-0 grid place-items-center bg-background/78">
					<span className="text-sm font-semibold text-muted-foreground">!</span>
				</div>
			) : null}
		</div>
	);
}
