import { useCallback, useRef, useState } from "react";

/**
 * Ref-latched guard for per-resource-id async actions
 * (frontend-uiux-guidelines.md 3.3): the latch is set synchronously before
 * awaiting, and only the owning operation releases it. Each run takes an
 * operation token that `clearPending()` invalidates, so neither a stale
 * resolution for a different id nor a cleared action A in flight can unlock
 * a newer action B that started afterwards — even when both use the same id.
 */
export function usePendingId<T>() {
	const [pendingId, setPendingId] = useState<T | null>(null);
	const pendingIdRef = useRef<T | null>(null);
	const tokenRef = useRef(0);

	const runWithPending = useCallback(
		async (id: T, action: () => Promise<void>): Promise<boolean> => {
			if (pendingIdRef.current !== null) {
				return false;
			}

			pendingIdRef.current = id;
			const token = ++tokenRef.current;
			setPendingId(id);
			try {
				await action();
				return true;
			} finally {
				if (token === tokenRef.current) {
					pendingIdRef.current = null;
					setPendingId(null);
				}
			}
		},
		[],
	);

	const clearPending = useCallback(() => {
		// Invalidate the in-flight operation's token so its eventual `finally`
		// cannot release a lock that now belongs to a newer action.
		tokenRef.current += 1;
		pendingIdRef.current = null;
		setPendingId(null);
	}, []);

	return { clearPending, pendingId, runWithPending };
}
