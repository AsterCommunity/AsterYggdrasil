import { useCallback, useEffect, useRef, useState } from "react";

type PendingActionResult<T> =
	| { entered: false; value: undefined }
	| { entered: true; value: T };

/**
 * Synchronous ref-latched guard for singleton async submissions
 * (frontend-uiux-guidelines.md 3.3):
 *
 * - the latch is set synchronously before awaiting the action, so rapid
 *   double clicks or Enter repeats cannot re-enter;
 * - the latch is released in `finally`, including on thrown errors;
 * - the React state mirror is only written while the component is mounted,
 *   so late resolutions after unmount stay safe.
 */
export function usePendingAction() {
	const pendingRef = useRef(false);
	const mountedRef = useRef(true);
	const [pending, setPending] = useState(false);

	useEffect(() => {
		mountedRef.current = true;
		return () => {
			mountedRef.current = false;
		};
	}, []);

	const runWithPending = useCallback(
		async <T>(action: () => Promise<T>): Promise<PendingActionResult<T>> => {
			if (pendingRef.current) {
				return { entered: false, value: undefined };
			}

			pendingRef.current = true;
			setPending(true);
			try {
				const value = await action();
				return { entered: true, value };
			} finally {
				pendingRef.current = false;
				if (mountedRef.current) {
					setPending(false);
				}
			}
		},
		[],
	);

	const clearPending = useCallback(() => {
		pendingRef.current = false;
		if (mountedRef.current) {
			setPending(false);
		}
	}, []);

	return { clearPending, pending, runWithPending };
}
