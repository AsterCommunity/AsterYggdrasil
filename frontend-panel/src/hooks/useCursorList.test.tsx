import { act, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { deferred } from "@/test/deferred";
import { useCursorList } from "./useCursorList";

type Row = { id: number };
type Cursor = { id: number };

function page(total: number, nextCursor: Cursor | null, ids: number[]) {
	return {
		items: ids.map((id) => ({ id })),
		nextCursor,
		total,
	};
}

function ReloadFirstPageProbe({
	loadPage,
	onReload,
}: {
	loadPage: (args: { cursor: Cursor | null }) => Promise<{
		items: Row[];
		nextCursor: Cursor | null;
		total: number;
	}>;
	onReload: (result: unknown) => void;
}) {
	const list = useCursorList<Row, Cursor>({ deps: [], loadPage });
	return (
		<div>
			<button
				type="button"
				onClick={() => {
					void list.reloadFirstPage().then(onReload);
				}}
			>
				reload-first
			</button>
			<ul>
				{list.items.map((row) => (
					<li key={row.id}>row:{row.id}</li>
				))}
			</ul>
		</div>
	);
}

function Probe({
	deps,
	enabled,
	onError,
	loadPage,
	onItems,
}: {
	deps?: unknown[];
	enabled?: boolean;
	loadPage: (args: { cursor: Cursor | null }) => Promise<{
		items: Row[];
		nextCursor: Cursor | null;
		total: number;
	}>;
	onError?: (error: unknown) => void;
	onItems?: (items: Row[]) => void;
}) {
	const list = useCursorList<Row, Cursor>({
		deps: deps ?? [],
		enabled,
		loadPage,
		onError,
	});
	onItems?.(list.items);
	return (
		<div>
			<span>current:{list.currentPage}</span>
			<span>total-pages:{list.totalPages}</span>
			<span>loading:{String(list.loading)}</span>
			<button
				type="button"
				disabled={list.nextPageDisabled}
				onClick={list.goNext}
			>
				next
			</button>
			<button
				type="button"
				disabled={list.prevPageDisabled}
				onClick={list.goPrevious}
			>
				previous
			</button>
			<button type="button" onClick={list.reset}>
				reset
			</button>
			<ul>
				{list.items.map((row) => (
					<li key={row.id}>row:{row.id}</li>
				))}
			</ul>
		</div>
	);
}

describe("useCursorList", () => {
	it("loads the first page with a null cursor", async () => {
		const loadPage = vi.fn(async () => page(2, { id: 9 }, [1]));
		render(<Probe loadPage={loadPage} />);

		await waitFor(() => {
			expect(screen.getByText("row:1")).toBeInTheDocument();
		});
		expect(loadPage).toHaveBeenCalledWith({ cursor: null });
		expect(screen.getByText("current:1")).toBeInTheDocument();
		expect(screen.getByText("total-pages:2")).toBeInTheDocument();
	});

	it("navigates forward using the next cursor and back through the stack", async () => {
		const loadPage = vi
			.fn<
				(args: { cursor: Cursor | null }) => Promise<{
					items: Row[];
					nextCursor: Cursor | null;
					total: number;
				}>
			>()
			.mockResolvedValueOnce(page(20, { id: 11 }, [1]))
			.mockResolvedValueOnce(page(20, { id: 12 }, [2]))
			.mockResolvedValue(page(20, null, [3]));
		render(<Probe loadPage={loadPage} />);

		await waitFor(() => {
			expect(screen.getByText("row:1")).toBeInTheDocument();
		});

		act(() => {
			screen.getByRole("button", { name: "next" }).click();
		});
		await waitFor(() => {
			expect(loadPage).toHaveBeenLastCalledWith({ cursor: { id: 11 } });
		});
		expect(screen.getByText("current:2")).toBeInTheDocument();

		act(() => {
			screen.getByRole("button", { name: "previous" }).click();
		});
		await waitFor(() => {
			expect(loadPage).toHaveBeenLastCalledWith({ cursor: null });
		});
		expect(screen.getByText("current:1")).toBeInTheDocument();
	});

	it("walks back a page when a page resolves empty while newer pages exist", async () => {
		const loadPage = vi
			.fn<
				(args: { cursor: Cursor | null }) => Promise<{
					items: Row[];
					nextCursor: Cursor | null;
					total: number;
				}>
			>()
			.mockResolvedValueOnce(page(10, { id: 11 }, [1]))
			.mockResolvedValueOnce(page(10, null, []))
			.mockResolvedValue(page(10, null, [9]));
		render(<Probe loadPage={loadPage} />);

		await waitFor(() => {
			expect(screen.getByText("row:1")).toBeInTheDocument();
		});
		act(() => {
			screen.getByRole("button", { name: "next" }).click();
		});

		await waitFor(() => {
			expect(loadPage).toHaveBeenLastCalledWith({ cursor: null });
		});
		await waitFor(() => {
			expect(screen.getByText("row:9")).toBeInTheDocument();
		});
		expect(screen.getByText("current:1")).toBeInTheDocument();
	});

	it("resets to the first page when deps change", async () => {
		const loadPage = vi.fn(async ({ cursor }: { cursor: Cursor | null }) =>
			cursor ? page(20, null, [2]) : page(20, { id: 11 }, [1]),
		);
		const { rerender } = render(<Probe loadPage={loadPage} deps={["a", 10]} />);

		await waitFor(() => {
			expect(screen.getByText("row:1")).toBeInTheDocument();
		});
		act(() => {
			screen.getByRole("button", { name: "next" }).click();
		});
		await waitFor(() => {
			expect(screen.getByText("row:2")).toBeInTheDocument();
		});

		rerender(<Probe loadPage={loadPage} deps={["b", 10]} />);
		await waitFor(() => {
			expect(loadPage).toHaveBeenLastCalledWith({ cursor: null });
		});
	});

	it("discards superseded responses via generation counting", async () => {
		const first = deferred<{
			items: Row[];
			nextCursor: Cursor | null;
			total: number;
		}>();
		const loadPage = vi
			.fn<
				(args: { cursor: Cursor | null }) => Promise<{
					items: Row[];
					nextCursor: Cursor | null;
					total: number;
				}>
			>()
			.mockReturnValueOnce(first.promise)
			.mockResolvedValue(page(20, null, [2]));
		const { rerender } = render(<Probe loadPage={loadPage} deps={["a"]} />);

		await waitFor(() => {
			expect(screen.getByText("loading:true")).toBeInTheDocument();
		});
		rerender(<Probe loadPage={loadPage} deps={["b"]} />);

		await waitFor(() => {
			expect(screen.getByText("row:2")).toBeInTheDocument();
		});

		act(() => {
			first.resolve(page(20, null, [1]));
		});
		await waitFor(() => {
			expect(screen.getByText("row:2")).toBeInTheDocument();
		});
		expect(screen.queryByText("row:1")).not.toBeInTheDocument();
	});

	it("does not load while disabled and loads once enabled", async () => {
		const loadPage = vi.fn(async () => page(1, null, [1]));
		const { rerender } = render(<Probe loadPage={loadPage} enabled={false} />);

		expect(loadPage).not.toHaveBeenCalled();

		rerender(<Probe loadPage={loadPage} enabled={true} />);
		await waitFor(() => {
			expect(screen.getByText("row:1")).toBeInTheDocument();
		});
	});

	it("reloads after disabling mid-flight, despite an unchanged signature", async () => {
		// Regression: disabling discards the in-flight response, so re-enabling
		// with the same stack must reload instead of hitting the signature
		// dedup short-circuit with the discarded cursor recorded as loaded.
		const resolvers: Array<
			(value: {
				items: Row[];
				nextCursor: Cursor | null;
				total: number;
			}) => void
		> = [];
		const loadPage = vi
			.fn<
				(args: { cursor: Cursor | null }) => Promise<{
					items: Row[];
					nextCursor: Cursor | null;
					total: number;
				}>
			>()
			.mockImplementation(
				() =>
					new Promise((resolve) => {
						resolvers.push(resolve);
					}),
			);
		const { rerender } = render(<Probe loadPage={loadPage} enabled={true} />);
		expect(loadPage).toHaveBeenCalledTimes(1);

		// Disable while the first request is still pending; its response (if
		// any) would be discarded by the generation check.
		rerender(<Probe loadPage={loadPage} enabled={false} />);
		resolvers[0]?.(page(1, null, [1]));
		rerender(<Probe loadPage={loadPage} enabled={true} />);

		await waitFor(() => {
			expect(loadPage).toHaveBeenCalledTimes(2);
		});
		resolvers[1]?.(page(1, null, [1]));
		await waitFor(() => {
			expect(screen.getByText("row:1")).toBeInTheDocument();
		});
	});

	it("stores a normalized error and calls onError without throwing", async () => {
		const onError = vi.fn();
		const loadPage = vi
			.fn<
				(args: { cursor: Cursor | null }) => Promise<{
					items: Row[];
					nextCursor: Cursor | null;
					total: number;
				}>
			>()
			.mockRejectedValue(new Error("load failed"));
		render(<Probe loadPage={loadPage} onError={onError} />);

		await waitFor(() => {
			expect(onError).toHaveBeenCalled();
		});
		expect(screen.getByText("loading:false")).toBeInTheDocument();
	});

	it("reloadFirstPage returns the loaded page", async () => {
		let resolveFirst: (value: {
			items: Row[];
			nextCursor: Cursor | null;
			total: number;
		}) => void = () => {};
		const loadPage = vi
			.fn<
				(args: { cursor: Cursor | null }) => Promise<{
					items: Row[];
					nextCursor: Cursor | null;
					total: number;
				}>
			>()
			.mockImplementationOnce(
				() =>
					new Promise((resolve) => {
						resolveFirst = resolve;
					}),
			)
			.mockResolvedValue(page(20, null, [7]));

		const resolvedResults: unknown[] = [];
		render(
			<ReloadFirstPageProbe
				loadPage={loadPage}
				onReload={(result) => {
					resolvedResults.push(result);
				}}
			/>,
		);
		act(() => {
			resolveFirst(page(20, { id: 11 }, [1]));
		});
		await waitFor(() => {
			expect(screen.getByText("row:1")).toBeInTheDocument();
		});

		act(() => {
			screen.getByRole("button", { name: "reload-first" }).click();
		});
		await waitFor(() => {
			expect(resolvedResults.length).toBeGreaterThan(0);
		});
		expect(resolvedResults[0]).toMatchObject({ total: 20 });
		expect(loadPage).toHaveBeenLastCalledWith({ cursor: null });
		expect(screen.getByText("row:7")).toBeInTheDocument();
	});
});
