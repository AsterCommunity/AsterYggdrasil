import { useCallback, useRef, useState } from "react";

/**
 * Ref-latched guard for per-resource-id async actions
 * (frontend-uiux-guidelines.md 3.3): the latch is set synchronously before
 * awaiting, released in `finally`, and only clears while the same id is
 * still pending so a stale resolution cannot clobber a newer action.
 */
export function usePendingId<T>() {
	const [pendingId, setPendingId] = useState<T | null>(null);
	const pendingIdRef = useRef<T | null>(null);

	const runWithPending = useCallback(
		async (id: T, action: () => Promise<void>): Promise<boolean> => {
			if (pendingIdRef.current !== null) {
				return false;
			}

			pendingIdRef.current = id;
			setPendingId(id);
			try {
				await action();
				return true;
			} finally {
				if (Object.is(pendingIdRef.current, id)) {
					pendingIdRef.current = null;
					setPendingId(null);
				}
			}
		},
		[],
	);

	const clearPending = useCallback(() => {
		pendingIdRef.current = null;
		setPendingId(null);
	}, []);

	return { clearPending, pendingId, runWithPending };
}
