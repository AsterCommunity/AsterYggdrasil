import { useCallback, useEffect, useRef, useState } from "react";

const DEFAULT_COPIED_RESET_MS = 1200;

type UseClipboardOptions = {
	/** How long `copied` stays true after a successful write. */
	copiedResetMs?: number;
};

/**
 * Single owner of the clipboard write path (issue #58): every "copy to
 * clipboard" interaction goes through `copy`, which swallows write errors
 * and reports them as `false` so callers decide the user-facing feedback
 * (toast success / generic error). `copied` mirrors the last successful
 * write for "copied" check-icon feedback and resets after `copiedResetMs`;
 * the reset timer is cleared on unmount and on a newer write.
 */
export function useClipboard({
	copiedResetMs = DEFAULT_COPIED_RESET_MS,
}: UseClipboardOptions = {}) {
	const [copied, setCopied] = useState(false);
	const resetTimerRef = useRef<number | null>(null);

	useEffect(() => {
		return () => {
			if (resetTimerRef.current !== null) {
				window.clearTimeout(resetTimerRef.current);
			}
		};
	}, []);

	const copy = useCallback(
		async (value: string): Promise<boolean> => {
			try {
				await navigator.clipboard.writeText(value);
			} catch {
				return false;
			}

			setCopied(true);
			if (resetTimerRef.current !== null) {
				window.clearTimeout(resetTimerRef.current);
			}
			resetTimerRef.current = window.setTimeout(() => {
				resetTimerRef.current = null;
				setCopied(false);
			}, copiedResetMs);
			return true;
		},
		[copiedResetMs],
	);

	return { copied, copy };
}
