import { useEffect, useRef, useState } from "react";

/**
 * Resolved source for a texture preview slot.
 *
 * - `preview`: the generated preview PNG, fetched as a blob object URL.
 * - `source`: render the raw texture URL instead — the preview is still
 *   generating (`202`), the URL is missing, or generation failed (`404`,
 *   network). The component shows the plain texture in every non-preview
 *   case, so users see the skin immediately and the preview replaces it
 *   seamlessly when it lands.
 */
export type PollingPreviewState =
	| { kind: "preview"; src: string }
	| { kind: "source"; src: string };

type UsePollingTexturePreviewOptions = {
	/**
	 * 202 retries within one round before scheduling a delayed refresh round,
	 * mirroring the AsterDrive thumbnail contract.
	 */
	maxAttempts?: number;
	/**
	 * Cross-round budget: after this many total attempts (202 polling across
	 * refresh rounds plus network retries) the hook stops and keeps showing
	 * the raw texture. Guards against a worker that never runs serving `202`
	 * forever (issue #40 retry ceiling).
	 */
	maxTotalAttempts?: number;
};

const DEFAULT_MAX_ATTEMPTS = 5;
const DEFAULT_MAX_TOTAL_ATTEMPTS = 25;
const DEFAULT_RETRY_AFTER_SECS = 2;
const MAX_RETRY_AFTER_SECS = 30;

function retryDelayMs(response: Response): number {
	const raw = Number.parseInt(response.headers.get("retry-after") ?? "", 10);
	if (!Number.isFinite(raw) || raw <= 0) {
		return DEFAULT_RETRY_AFTER_SECS * 1000;
	}
	return Math.min(raw, MAX_RETRY_AFTER_SECS) * 1000;
}

/**
 * Fetch-driven preview loading with `Retry-After` polling, mirroring the
 * AsterDrive thumbnail contract.
 *
 * A plain `<img src>` cannot distinguish a `202` (generation still running)
 * from a `404` (terminal), so this hook fetches the preview URL itself: `200`
 * resolves to a blob object URL; `202` sleeps for `Retry-After` and retries
 * within the round, then schedules a delayed refresh round; transient network
 * failures retry within the same shared budget. `404` and an exhausted budget
 * are terminal, falling back to the raw texture. When no preview URL is
 * provided the hook reports the texture URL directly and never fetches,
 * preserving plain-image behavior for textures without previews.
 */
export function usePollingTexturePreview(
	previewUrl: string | null | undefined,
	textureUrl: string,
	options: UsePollingTexturePreviewOptions = {},
): PollingPreviewState {
	const maxAttempts = options.maxAttempts ?? DEFAULT_MAX_ATTEMPTS;
	const maxTotalAttempts =
		options.maxTotalAttempts ?? DEFAULT_MAX_TOTAL_ATTEMPTS;
	const [refreshTick, setRefreshTick] = useState(0);
	const [state, setState] = useState<PollingPreviewState>(() => ({
		kind: "source",
		src: textureUrl,
	}));
	// Shared across refresh rounds: the budget belongs to the preview URL,
	// not to a single round, so a never-ending 202 stream still terminates.
	const totalAttemptsRef = useRef(0);
	const attemptedUrlRef = useRef<string | null>(null);

	// biome-ignore lint/correctness/useExhaustiveDependencies: refreshTick is an intentional re-fetch trigger
	useEffect(() => {
		if (!previewUrl) {
			attemptedUrlRef.current = null;
			totalAttemptsRef.current = 0;
			setState({ kind: "source", src: textureUrl });
			return;
		}
		if (attemptedUrlRef.current !== previewUrl) {
			// New preview URL: the budget restarts with it.
			attemptedUrlRef.current = previewUrl;
			totalAttemptsRef.current = 0;
		}

		// A texture switch must not keep showing the previous texture's
		// preview (which this effect's cleanup is about to revoke).
		setState({ kind: "source", src: textureUrl });

		const controller = new AbortController();
		let cancelled = false;
		let timer: ReturnType<typeof setTimeout> | undefined;
		let objectUrl: string | undefined;

		const attempt = async (round: number): Promise<void> => {
			try {
				totalAttemptsRef.current += 1;
				const response = await fetch(previewUrl, {
					signal: controller.signal,
					headers: { Accept: "image/png" },
				});
				if (cancelled) return;

				if (response.status === 200) {
					// Body reads have their own failure path (truncated
					// transfer, abort mid-read) — treat them like any other
					// failure and fall back to the raw texture.
					const blob = await response.blob();
					if (cancelled) return;
					objectUrl = URL.createObjectURL(blob);
					setState({ kind: "preview", src: objectUrl });
					return;
				}

				if (response.status === 202) {
					const delay = retryDelayMs(response);
					if (totalAttemptsRef.current >= maxTotalAttempts) {
						// Cross-round budget exhausted: stop polling and keep
						// the raw texture (issue #40 retry ceiling).
						setState({ kind: "source", src: textureUrl });
						return;
					}
					if (round >= maxAttempts) {
						// Round exhausted: schedule a fresh round after one more
						// Retry-After instead of giving up, and keep showing the
						// source texture meanwhile.
						timer = setTimeout(() => {
							if (!cancelled) setRefreshTick((tick) => tick + 1);
						}, delay);
						setState({ kind: "source", src: textureUrl });
						return;
					}
					timer = setTimeout(() => {
						void attempt(round + 1);
					}, delay);
					return;
				}

				// 404 (unknown hash / failed generation / stale variant) and any
				// other non-200 are terminal for this preview URL.
				setState({ kind: "source", src: textureUrl });
			} catch (error) {
				// Our own abort: nothing to recover.
				if (cancelled || controller.signal.aborted) return;
				// Transient network failure: retry within the shared budget so
				// a flaky connection does not hide a preview that already
				// rendered (issue #40 limited retry).
				if (totalAttemptsRef.current >= maxTotalAttempts) {
					setState({ kind: "source", src: textureUrl });
					return;
				}
				timer = setTimeout(() => {
					void attempt(round);
				}, DEFAULT_RETRY_AFTER_SECS * 1000);
				void error;
			}
		};

		void attempt(1);

		return () => {
			cancelled = true;
			controller.abort();
			if (timer !== undefined) clearTimeout(timer);
			if (objectUrl !== undefined) URL.revokeObjectURL(objectUrl);
		};
	}, [previewUrl, textureUrl, maxAttempts, maxTotalAttempts, refreshTick]);

	return state;
}
