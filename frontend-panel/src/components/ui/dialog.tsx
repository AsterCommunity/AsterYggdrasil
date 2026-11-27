"use client";

import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import type * as React from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "@/components/ui/icon";
import { cn } from "@/lib/utils";

const Dialog = DialogPrimitive.Root;
const DialogTrigger = DialogPrimitive.Trigger;
const DialogClose = DialogPrimitive.Close;
const DialogPortal = DialogPrimitive.Portal;

function DialogContent({
	className,
	children,
	keepMounted = false,
	...props
}: DialogPrimitive.Popup.Props & {
	keepMounted?: boolean;
}) {
	const { t } = useTranslation();

	return (
		<DialogPortal keepMounted={keepMounted}>
			<DialogPrimitive.Backdrop className="fixed inset-0 z-50 bg-background/70 backdrop-blur-sm duration-150 data-open:animate-in data-open:fade-in-0 data-closed:animate-out data-closed:fade-out-0" />
			<DialogPrimitive.Viewport className="fixed inset-0 z-50 grid place-items-center overflow-y-auto p-4">
				<DialogPrimitive.Popup
					data-slot="dialog-content"
					data-theme-surface="overlay"
					className={cn(
						"relative grid max-h-[min(720px,calc(100dvh-2rem))] w-full max-w-lg gap-4 overflow-hidden rounded-xl border border-border/70 bg-popover p-5 text-popover-foreground shadow-2xl shadow-black/25 ring-1 ring-foreground/5 duration-150 outline-none data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95",
						className,
					)}
					{...props}
				>
					{children}
					<DialogPrimitive.Close
						className="absolute top-3 right-3 inline-flex size-8 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/30 focus-visible:outline-none"
						aria-label={t("common.close")}
					>
						<Icon name="X" className="size-4" />
					</DialogPrimitive.Close>
				</DialogPrimitive.Popup>
			</DialogPrimitive.Viewport>
		</DialogPortal>
	);
}

function DialogHeader({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div className={cn("grid gap-1.5 pr-8 text-left", className)} {...props} />
	);
}

function DialogFooter({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			className={cn(
				"flex flex-col-reverse gap-2 border-t border-border/70 pt-4 sm:flex-row sm:justify-end",
				className,
			)}
			{...props}
		/>
	);
}

function DialogTitle({ className, ...props }: DialogPrimitive.Title.Props) {
	return (
		<DialogPrimitive.Title
			className={cn("text-lg font-semibold tracking-normal", className)}
			{...props}
		/>
	);
}

function DialogDescription({
	className,
	...props
}: DialogPrimitive.Description.Props) {
	return (
		<DialogPrimitive.Description
			className={cn("text-sm text-muted-foreground", className)}
			{...props}
		/>
	);
}

export {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogPortal,
	DialogTitle,
	DialogTrigger,
};
