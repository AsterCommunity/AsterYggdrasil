import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { deferred } from "@/test/deferred";
import { usePendingId } from "./usePendingId";

type Target = { id: number };

function StaleResolutionProbe({
	activeAction,
}: {
	activeAction: { current: Promise<void> };
}) {
	const { pendingId, runWithPending } = usePendingId<number>();
	return (
		<div>
			<span>pending:{pendingId ?? "none"}</span>
			<button
				type="button"
				onClick={() => {
					const current = activeAction.current;
					void runWithPending(1, () => current);
				}}
			>
				run-1
			</button>
			<button
				type="button"
				onClick={() => {
					activeAction.current = Promise.resolve();
					void runWithPending(2, () => activeAction.current);
				}}
			>
				run-2
			</button>
		</div>
	);
}

function Probe({
	target,
	action,
	onEntered,
}: {
	target: Target;
	action: (id: number) => Promise<void>;
	onEntered?: (entered: boolean) => void;
}) {
	const { pendingId, runWithPending } = usePendingId<number>();
	return (
		<div>
			<span>pending:{pendingId ?? "none"}</span>
			<button
				type="button"
				onClick={() => {
					void runWithPending(target.id, () => action(target.id)).then(
						(entered) => {
							onEntered?.(entered);
						},
					);
				}}
			>
				run
			</button>
		</div>
	);
}

describe("usePendingId", () => {
	it("blocks concurrent actions and clears the id after completion", async () => {
		const gate = deferred<void>();
		const entered: boolean[] = [];
		render(
			<Probe
				target={{ id: 7 }}
				action={() => gate.promise}
				onEntered={(value) => {
					entered.push(value);
				}}
			/>,
		);

		const run = screen.getByRole("button", { name: "run" });
		act(() => {
			run.click();
			run.click();
		});
		expect(screen.getByText("pending:7")).toBeInTheDocument();

		act(() => {
			gate.resolve();
		});
		await act(async () => {
			await new Promise((resolve) => setTimeout(resolve, 0));
		});
		expect(screen.getByText("pending:none")).toBeInTheDocument();
		expect(entered).toEqual([false, true]);
	});

	it("does not let a stale resolution clear a newer id", async () => {
		const gate = deferred<void>();
		const activeAction: { current: Promise<void> } = {
			current: gate.promise,
		};
		render(<StaleResolutionProbe activeAction={activeAction} />);

		act(() => {
			screen.getByRole("button", { name: "run-1" }).click();
		});
		expect(screen.getByText("pending:1")).toBeInTheDocument();

		// While id 1 is pending a different id cannot start (single latch).
		act(() => {
			screen.getByRole("button", { name: "run-2" }).click();
		});
		expect(screen.getByText("pending:1")).toBeInTheDocument();

		act(() => {
			gate.resolve();
		});
		await act(async () => {
			await new Promise((resolve) => setTimeout(resolve, 0));
		});
		expect(screen.getByText("pending:none")).toBeInTheDocument();
	});
});
