import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { deferred } from "@/test/deferred";
import { usePendingAction } from "./usePendingAction";

function Probe({
	action,
	onResult,
}: {
	action: () => Promise<string>;
	onResult?: (result: { entered: boolean; value?: string }) => void;
}) {
	const { pending, runWithPending } = usePendingAction();
	return (
		<div>
			<span>pending:{String(pending)}</span>
			<button
				type="button"
				onClick={() => {
					void runWithPending(action).then((result) => {
						onResult?.(result);
					});
				}}
			>
				run
			</button>
		</div>
	);
}

describe("usePendingAction", () => {
	it("rejects a concurrent re-entry synchronously while the first action is pending", async () => {
		const gate = deferred<string>();
		const results: Array<{ entered: boolean; value?: string }> = [];
		render(
			<Probe
				action={() => gate.promise}
				onResult={(result) => {
					results.push(result);
				}}
			/>,
		);

		const run = screen.getByRole("button", { name: "run" });
		act(() => {
			run.click();
			// Second click in the same synchronous batch: the ref latch must
			// already be set, so this never enters the action.
			run.click();
		});

		act(() => {
			gate.resolve("done");
		});
		await act(async () => {
			await Promise.resolve();
		});

		expect(results).toEqual([
			{ entered: false, value: undefined },
			{ entered: true, value: "done" },
		]);
		await waitForPendingFalse();
	});

	it("releases the latch after the action settles and after a thrown error", async () => {
		const failing = async () => {
			throw new Error("boom");
		};
		let attempts = 0;
		render(
			<ErrorProbe
				failing={failing}
				onAttempt={() => {
					attempts += 1;
				}}
			/>,
		);

		const run = screen.getByRole("button", { name: "run" });
		await act(async () => {
			run.click();
			await Promise.resolve();
		});
		expect(attempts).toBe(1);
		expect(screen.getByText("pending:false")).toBeInTheDocument();

		await act(async () => {
			run.click();
		});
		expect(attempts).toBe(2);
		expect(screen.getByText("pending:false")).toBeInTheDocument();
	});

	it("stays safe when the component unmounts mid-action", async () => {
		const gate = deferred<string>();
		const { unmount } = render(<Probe action={() => gate.promise} />);

		act(() => {
			screen.getByRole("button", { name: "run" }).click();
		});
		unmount();

		expect(() => {
			act(() => {
				gate.resolve("late");
			});
		}).not.toThrow();
		await act(async () => {
			await gate.promise;
		});
	});
});

function ErrorProbe({
	failing,
	onAttempt,
}: {
	failing: () => Promise<never>;
	onAttempt: () => void;
}) {
	const { pending, runWithPending } = usePendingAction();
	return (
		<div>
			<span>pending:{String(pending)}</span>
			<button
				type="button"
				onClick={() => {
					// Page code owns error handling; the rejected run
					// promise mirrors the thrown action error.
					runWithPending(async () => {
						onAttempt();
						await failing();
					}).catch(() => {});
				}}
			>
				run
			</button>
		</div>
	);
}

async function waitForPendingFalse() {
	await act(async () => {
		await new Promise((resolve) => setTimeout(resolve, 0));
	});
}
