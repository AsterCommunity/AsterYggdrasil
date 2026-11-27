import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useClipboard } from "./useClipboard";

function makeWriteText() {
	return vi.fn<(value: string) => Promise<void>>().mockResolvedValue();
}

function installClipboard(writeText: ReturnType<typeof makeWriteText>) {
	Object.defineProperty(navigator, "clipboard", {
		configurable: true,
		value: { writeText },
	});
}

type ProbeProps = {
	value?: string;
	copiedResetMs?: number;
	onResult?: (result: boolean) => void;
};

function ClipboardProbe({
	value = "copy-me",
	copiedResetMs,
	onResult,
}: ProbeProps) {
	const { copied, copy } = useClipboard(
		copiedResetMs === undefined ? undefined : { copiedResetMs },
	);
	return (
		<div>
			<span data-testid="copied">{String(copied)}</span>
			<button
				type="button"
				onClick={() => {
					void copy(value).then((result) => onResult?.(result));
				}}
			>
				copy
			</button>
		</div>
	);
}

describe("useClipboard", () => {
	beforeEach(() => {
		installClipboard(makeWriteText());
	});

	afterEach(() => {
		Object.defineProperty(navigator, "clipboard", {
			configurable: true,
			value: undefined,
		});
	});

	it("writes the value, flips copied, and resets it after the configured delay", async () => {
		vi.useFakeTimers();
		const writeText = makeWriteText();
		installClipboard(writeText);

		render(<ClipboardProbe copiedResetMs={1200} />);
		expect(screen.getByTestId("copied")).toHaveTextContent("false");

		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "copy" }));
		});
		expect(writeText).toHaveBeenCalledWith("copy-me");
		expect(screen.getByTestId("copied")).toHaveTextContent("true");

		await act(async () => {
			vi.advanceTimersByTime(1199);
		});
		expect(screen.getByTestId("copied")).toHaveTextContent("true");

		await act(async () => {
			vi.advanceTimersByTime(1);
		});
		expect(screen.getByTestId("copied")).toHaveTextContent("false");
	});

	it("returns false and never flips copied when the write rejects", async () => {
		vi.useFakeTimers();
		const results: boolean[] = [];
		const writeText = makeWriteText().mockRejectedValue(new Error("denied"));
		installClipboard(writeText);

		render(<ClipboardProbe onResult={(result) => results.push(result)} />);

		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "copy" }));
		});

		expect(results).toEqual([false]);
		expect(screen.getByTestId("copied")).toHaveTextContent("false");
	});

	it("clears the pending reset timer when a newer copy succeeds", async () => {
		vi.useFakeTimers();
		render(<ClipboardProbe copiedResetMs={1200} />);

		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "copy" }));
		});
		await act(async () => {
			vi.advanceTimersByTime(600);
		});
		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "copy" }));
		});

		await act(async () => {
			vi.advanceTimersByTime(600);
		});
		// Only 600ms after the second write: the first timer must not fire.
		expect(screen.getByTestId("copied")).toHaveTextContent("true");

		await act(async () => {
			vi.advanceTimersByTime(600);
		});
		expect(screen.getByTestId("copied")).toHaveTextContent("false");
	});

	it("does not reset copied after unmount", async () => {
		vi.useFakeTimers();
		const { unmount } = render(<ClipboardProbe copiedResetMs={1200} />);

		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "copy" }));
		});
		expect(screen.getByTestId("copied")).toHaveTextContent("true");
		unmount();

		expect(() => {
			vi.advanceTimersByTime(2000);
		}).not.toThrow();
	});
});
