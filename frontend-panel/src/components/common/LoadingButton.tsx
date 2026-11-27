import type { ComponentProps, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Icon, type IconName } from "@/components/ui/icon";
import { cn } from "@/lib/utils";

type LoadingButtonProps = Omit<ComponentProps<typeof Button>, "children"> & {
	/** Async submit in flight; swaps the glyph for a spinner and disables. */
	loading: boolean;
	/** Semantic glyph shown while idle; omit for text-only buttons. */
	icon?: IconName;
	/** Classes for the glyph in both states; override the default margin/size here. */
	iconClassName?: string;
	children: ReactNode;
};

/**
 * Shared async submit/save button (frontend-uiux-guidelines.md 3.1 FormDialog:
 * "footer 固定，保存按钮提交中 loading"): renders `icon` while idle and a
 * spinner while `loading`, disabling re-entry in both cases. The ref-latched
 * submit guard itself stays page-owned (`usePendingAction` / `usePendingId`).
 */
export function LoadingButton({
	loading,
	icon,
	iconClassName = "mr-2 size-4",
	children,
	disabled,
	...props
}: LoadingButtonProps) {
	const shownIcon: IconName | null = loading ? "Spinner" : (icon ?? null);

	return (
		<Button disabled={disabled || loading} {...props}>
			{shownIcon ? (
				<Icon
					name={shownIcon}
					className={cn(iconClassName, loading && "animate-spin")}
					aria-hidden={loading ? true : undefined}
				/>
			) : null}
			{children}
		</Button>
	);
}
