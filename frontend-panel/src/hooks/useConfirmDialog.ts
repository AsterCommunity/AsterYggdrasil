import { useCallback, useState } from "react";

export type ConfirmDialogProps<T> = {
	confirmTarget: T | null;
	open: boolean;
};

/**
 * Generic confirm-flow state shared by every "pick a target, show a
 * confirm dialog, run an action" page interaction. `dialogProps` spreads
 * directly onto `ConfirmDialog` (`open` / `onOpenChange` / `onConfirm`);
 * title and description stay page-owned because they need `t()` and the
 * current target.
 */
export function useConfirmDialog<T>(
	onConfirm: (target: T) => void | Promise<void>,
): {
	confirmTarget: T | null;
	requestConfirm: (target: T) => void;
	clearConfirm: () => void;
	dialogProps: {
		open: boolean;
		onOpenChange: (open: boolean) => void;
		onConfirm: () => void;
	};
} {
	const [confirmTarget, setConfirmTarget] = useState<T | null>(null);

	const requestConfirm = useCallback((target: T) => {
		setConfirmTarget(target);
	}, []);

	const clearConfirm = useCallback(() => {
		setConfirmTarget(null);
	}, []);

	const handleOpenChange = useCallback((open: boolean) => {
		if (!open) {
			setConfirmTarget(null);
		}
	}, []);

	const handleConfirm = useCallback(() => {
		const target = confirmTarget;
		setConfirmTarget(null);
		if (target !== null) {
			void onConfirm(target);
		}
	}, [confirmTarget, onConfirm]);

	return {
		confirmTarget,
		clearConfirm,
		requestConfirm,
		dialogProps: {
			open: confirmTarget !== null,
			onConfirm: handleConfirm,
			onOpenChange: handleOpenChange,
		},
	};
}
