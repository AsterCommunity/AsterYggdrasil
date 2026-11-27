import { useEffect, useState } from "react";

/**
 * Debounces a rapidly changing value: the returned value only follows
 * `value` after it has stayed unchanged for `delayMs`. The initial render
 * returns `value` immediately so callers reading URL-derived state do not
 * trigger a second load on mount.
 */
export function useDebouncedValue<T>(value: T, delayMs: number): T {
	const [debounced, setDebounced] = useState(value);

	useEffect(() => {
		const timer = window.setTimeout(() => {
			setDebounced(value);
		}, delayMs);
		return () => window.clearTimeout(timer);
	}, [delayMs, value]);

	return debounced;
}
