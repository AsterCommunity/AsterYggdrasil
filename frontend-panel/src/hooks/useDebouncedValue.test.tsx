import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useDebouncedValue } from "./useDebouncedValue";

function Probe({ value }: { value: string }) {
	const debounced = useDebouncedValue(value, 250);
	return <span>debounced:{debounced}</span>;
}

describe("useDebouncedValue", () => {
	beforeEach(() => {
		vi.useFakeTimers();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it("returns the initial value immediately on the first render", () => {
		render(<Probe value="initial" />);
		expect(screen.getByText("debounced:initial")).toBeInTheDocument();

		// The trailing timer re-writes the same value; state stays stable.
		act(() => {
			vi.advanceTimersByTime(250);
		});
		expect(screen.getByText("debounced:initial")).toBeInTheDocument();
	});

	it("follows the value only after the delay has elapsed", () => {
		const { rerender } = render(<Probe value="initial" />);

		rerender(<Probe value="changed" />);
		expect(screen.getByText("debounced:initial")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(249);
		});
		expect(screen.getByText("debounced:initial")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(1);
		});
		expect(screen.getByText("debounced:changed")).toBeInTheDocument();
	});

	it("restarts the timer on every change so bursts collapse to the last value", () => {
		const { rerender } = render(<Probe value="initial" />);

		rerender(<Probe value="a" />);
		act(() => {
			vi.advanceTimersByTime(200);
		});
		rerender(<Probe value="b" />);
		act(() => {
			vi.advanceTimersByTime(200);
		});
		expect(screen.getByText("debounced:initial")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(50);
		});
		expect(screen.getByText("debounced:b")).toBeInTheDocument();
	});
});
