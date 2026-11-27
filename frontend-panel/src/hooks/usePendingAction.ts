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
 * - every run takes an operation token; the token is invalidated by
 *   `clearPending()`, and a late `finally` only releases the lock when its
 *   token is still current — a cleared action A can therefore never unlock
 *   a newer action B that started while A was still in flight;
 * - the React state mirror is only written while the component is mounted,
 *   so late resolutions after unmount stay safe.
 */
export function usePendingAction() {
	const pendingRef = useRef(false);
	const tokenRef = useRef(0);
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
			const token = ++tokenRef.current;
			setPending(true);
			try {
				const value = await action();
				return { entered: true, value };
			} finally {
				if (token === tokenRef.current) {
					pendingRef.current = false;
					if (mountedRef.current) {
						setPending(false);
					}
				}
			}
		},
		[],
	);

	const clearPending = useCallback(() => {
		// Invalidate the in-flight operation's token so its eventual `finally`
		// cannot release a lock that now belongs to a newer action.
		tokenRef.current += 1;
		pendingRef.current = false;
		if (mountedRef.current) {
			setPending(false);
		}
	}, []);

	return { clearPending, pending, runWithPending };
}
