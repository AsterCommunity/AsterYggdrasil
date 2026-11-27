import type { ComponentProps, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Icon, REFRESH_ICON_NAME } from "@/components/ui/icon";

type RefreshButtonProps = Omit<ComponentProps<typeof Button>, "children"> & {
	/** Async refresh in flight; swaps the glyph for a spinner and disables. */
	loading?: boolean;
	/** Defaults to the shared `common.refresh` label; override for retry/resend actions. */
	children?: ReactNode;
};

/**
 * The one refresh/retry action button (frontend-uiux-guidelines.md 2.3):
 * spinner while the refresh is in flight, `REFRESH_ICON_NAME` otherwise.
 * All hand-rolled `name={loading ? "Spinner" : ...}` refresh copies
 * migrate here; the icon name lives only in the icon registry.
 */
export function RefreshButton({
	loading = false,
	children,
	disabled,
	type = "button",
	variant = "outline",
	size = "sm",
	...props
}: RefreshButtonProps) {
	const { t } = useTranslation();

	return (
		<Button
			type={type}
			variant={variant}
			size={size}
			disabled={disabled || loading}
			{...props}
		>
			<Icon
				name={loading ? "Spinner" : REFRESH_ICON_NAME}
				className={loading ? "mr-2 size-4 animate-spin" : "mr-2 size-4"}
			/>
			{children ?? t("common.refresh")}
		</Button>
	);
}
