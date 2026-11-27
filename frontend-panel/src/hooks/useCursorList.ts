import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { formatUnknownError } from "@/services/http";

export type ManagedCursorListPage<Item, Cursor> = {
	items: Item[];
	nextCursor: Cursor | null;
	total: number;
};

type UseCursorListArgs<Item, Cursor> = {
	/**
	 * Fetches one page. `cursor` is the anchor of the page being requested;
	 * `null` means the first page.
	 */
	loadPage: (args: {
		cursor: Cursor | null;
	}) => Promise<ManagedCursorListPage<Item, Cursor>>;
	/**
	 * Filter/pageSize values the request depends on. Every entry must be a
	 * JSON-serializable primitive; when the serialized signature changes the
	 * list resets to the first page and reloads.
	 */
	deps: unknown[];
	/**
	 * When false the hook stops loading and keeps an empty view (features
	 * gated by config flags or modes without a list).
	 */
	enabled?: boolean;
	/**
	 * Called with the raw error whenever a page load fails. Errors are also
	 * stored (normalized via formatUnknownError) in `error`. Pages that show
	 * toasts pass handleApiError here; pages that render an error empty state
	 * omit it.
	 */
	onError?: (error: unknown) => void;
};

/**
 * Shared cursor-pagination list skeleton. Owns the cursor stack (page
 * navigation), race protection via a generation counter, the
 * loading/error/items/total state machine, the empty-page walk-back (a page
 * that returns no items while newer pages exist pops one cursor and
 * retries), and automatic reloading when `deps` change.
 *
 * The stack is semantically `[null, cursorAfterPage1, cursorAfterPage2, ...]`;
 * its length equals the current page number. Cursor anchors are opaque to
 * this hook, so date-time/id/sort-key cursors all share one implementation.
 */
export function useCursorList<Item, Cursor>({
	loadPage,
	deps,
	enabled = true,
	onError,
}: UseCursorListArgs<Item, Cursor>) {
	const [cursorStack, setCursorStack] = useState<(Cursor | null)[]>([null]);
	const [items, setItems] = useState<Item[]>([]);
	const [total, setTotal] = useState(0);
	const [nextCursor, setNextCursor] = useState<Cursor | null>(null);
	const [loading, setLoading] = useState(enabled);
	const [error, setError] = useState<string | null>(null);

	const generationRef = useRef(0);
	const mountedRef = useRef(true);
	const cursorStackRef = useRef<(Cursor | null)[]>(cursorStack);
	const loadPageRef = useRef(loadPage);
	const onErrorRef = useRef(onError);

	useEffect(() => {
		mountedRef.current = true;
		return () => {
			mountedRef.current = false;
		};
	}, []);

	useEffect(() => {
		loadPageRef.current = loadPage;
	}, [loadPage]);

	useEffect(() => {
		onErrorRef.current = onError;
	}, [onError]);

	const applyCursorStack = useCallback((next: (Cursor | null)[]) => {
		cursorStackRef.current = next;
		setCursorStack(next);
	}, []);

	const load = useCallback(async (): Promise<ManagedCursorListPage<
		Item,
		Cursor
	> | null> => {
		const generation = ++generationRef.current;
		const stack = cursorStackRef.current;
		setLoading(true);
		setError(null);
		try {
			const page = await loadPageRef.current({
				cursor: stack.at(-1) ?? null,
			});
			if (generation !== generationRef.current || !mountedRef.current) {
				return null;
			}
			if (page.items.length === 0 && page.total > 0 && stack.length > 1) {
				// The page behind the last cursor vanished (rows deleted or
				// filtered out); walk back one page and let the stack change
				// trigger the retry.
				applyCursorStack(stack.slice(0, -1));
				return null;
			}
			setItems(page.items);
			setTotal(page.total);
			setNextCursor(page.nextCursor);
			return page;
		} catch (caught) {
			if (generation !== generationRef.current || !mountedRef.current) {
				return null;
			}
			setError(formatUnknownError(caught));
			onErrorRef.current?.(caught);
			return null;
		} finally {
			if (generation === generationRef.current && mountedRef.current) {
				setLoading(false);
			}
		}
	}, [applyCursorStack]);

	// Anchor-signature of the stack: identical anchors produce no reload
	// (except the initial mount, which always loads).
	const cursorSignature = useMemo(
		() => JSON.stringify(cursorStack),
		[cursorStack],
	);
	const lastLoadedCursorSignatureRef = useRef<string | null>(null);
	const depsSignature = useMemo(() => JSON.stringify(deps), [deps]);
	const lastLoadedSignatureRef = useRef(depsSignature);

	// Resets to the first page and loads it immediately, covering the case
	// where the stack is already `[null]` (no signature change would fire the
	// load effect). Effect-driven reloads stay signature-deduplicated.
	const resetAndLoad = useCallback(async () => {
		applyCursorStack([null]);
		lastLoadedCursorSignatureRef.current = JSON.stringify([null]);
		lastLoadedSignatureRef.current = depsSignature;
		return load();
	}, [applyCursorStack, depsSignature, load]);

	// Filter/pageSize signature changes restart the list at the first page,
	// loading immediately (once) with the fresh deps via resetAndLoad.
	// Declared BEFORE the signature-deduplicated load effect so, within one
	// commit, the stack and signature refs are already reconciled.
	useEffect(() => {
		if (!enabled || lastLoadedSignatureRef.current === depsSignature) {
			return;
		}
		lastLoadedSignatureRef.current = depsSignature;
		void resetAndLoad();
	}, [depsSignature, enabled, resetAndLoad]);

	useEffect(() => {
		if (!enabled) {
			generationRef.current += 1;
			setLoading(false);
			return;
		}
		if (
			lastLoadedCursorSignatureRef.current !== null &&
			lastLoadedCursorSignatureRef.current === cursorSignature
		) {
			return;
		}
		lastLoadedCursorSignatureRef.current = cursorSignature;
		void load();
	}, [cursorSignature, enabled, load]);

	const goNext = useCallback(() => {
		if (nextCursor == null) {
			return;
		}
		applyCursorStack([...cursorStackRef.current, nextCursor]);
	}, [applyCursorStack, nextCursor]);

	const goPrevious = useCallback(() => {
		const stack = cursorStackRef.current;
		if (stack.length <= 1) {
			return;
		}
		applyCursorStack(stack.slice(0, -1));
	}, [applyCursorStack]);

	const reset = useCallback(() => {
		if (cursorStackRef.current.length === 1) {
			return;
		}
		applyCursorStack([null]);
	}, [applyCursorStack]);

	/** Reloads the page the cursor stack currently points at. */
	const reload = load;

	/**
	 * Returns to the first page and loads it. Resolves with the loaded page,
	 * or null when the load was discarded (superseded, failed, or the
	 * component unmounted).
	 */
	const reloadFirstPage = resetAndLoad;

	const currentPage = cursorStack.length;
	const totalPages = Math.max(currentPage + (nextCursor !== null ? 1 : 0), 1);
	const prevPageDisabled = cursorStack.length <= 1;
	const nextPageDisabled = nextCursor === null;

	return {
		/** Rows of the current page. */
		items,
		/** Server-side total for the current query. */
		total,
		/** True while a page request is in flight (previous rows stay visible). */
		loading,
		/** Normalized load error of the latest request, null when it succeeded. */
		error,
		/** 1-based current page number. */
		currentPage,
		/** Known-lower-bound page count: pages seen plus one when a next cursor exists. */
		totalPages,
		prevPageDisabled,
		nextPageDisabled,
		/** Anchor returned by the server for the page after the current one. */
		nextCursor,
		/** Applies a local row update (e.g. after an inline mutation). */
		setItems,
		/** Applies a local total adjustment (e.g. after add/remove mutations). */
		setTotal,
		goNext,
		goPrevious,
		/** Drops navigation state and returns to the first page (no reload). */
		reset,
		reload,
		reloadFirstPage,
	};
}
