import { cn } from "@/lib/utils";

type MinecraftTextureImagePreviewAspect = "wide" | "portrait";

type MinecraftTextureImagePreviewProps = {
	alt: string;
	aspect?: MinecraftTextureImagePreviewAspect;
	className?: string;
	draggable?: boolean;
	previewUrl?: string | null;
	textureUrl: string;
};

const CHECKERBOARD_BACKGROUND =
	"bg-[linear-gradient(45deg,hsl(var(--muted))_25%,transparent_25%),linear-gradient(-45deg,hsl(var(--muted))_25%,transparent_25%),linear-gradient(45deg,transparent_75%,hsl(var(--muted))_75%),linear-gradient(-45deg,transparent_75%,hsl(var(--muted))_75%)] bg-[length:18px_18px] bg-[position:0_0,0_9px,9px_-9px,-9px_0]";

const PREVIEW_ASPECT_CLASS: Record<MinecraftTextureImagePreviewAspect, string> =
	{
		portrait: "aspect-[4/5]",
		wide: "aspect-square",
	};

export function MinecraftTextureImagePreview({
	alt,
	aspect = "wide",
	className,
	draggable,
	previewUrl,
	textureUrl,
}: MinecraftTextureImagePreviewProps) {
	const src = previewUrl || textureUrl;

	return (
		<div
			className={cn(
				"grid overflow-hidden p-4 place-items-center",
				CHECKERBOARD_BACKGROUND,
				PREVIEW_ASPECT_CLASS[aspect],
				className,
			)}
		>
			<img
				src={src}
				alt={alt}
				crossOrigin="anonymous"
				draggable={draggable}
				className="max-h-full max-w-full object-contain [image-rendering:pixelated]"
			/>
		</div>
	);
}
