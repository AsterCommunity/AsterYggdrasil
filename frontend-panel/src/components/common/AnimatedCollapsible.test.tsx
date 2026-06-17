import { act, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AnimatedCollapsible } from "@/components/common/AnimatedCollapsible";

function mockAnimationFrame() {
	let nextFrame = 1;
	const callbacks = new Map<number, FrameRequestCallback>();

	const requestAnimationFrame = vi
		.spyOn(window, "requestAnimationFrame")
		.mockImplementation((callback) => {
			const id = nextFrame;
			nextFrame += 1;
			callbacks.set(id, callback);
			return id;
		});
	const cancelAnimationFrame = vi
		.spyOn(window, "cancelAnimationFrame")
		.mockImplementation((id) => {
			callbacks.delete(id);
		});

	return {
		cancelAnimationFrame,
		flushFrame: () => {
			const [id, callback] = callbacks.entries().next().value ?? [];
			if (id == null || callback == null) {
				return false;
			}
			callbacks.delete(id);
			callback(performance.now());
			return true;
		},
		requestAnimationFrame,
	};
}

describe("AnimatedCollapsible", () => {
	it("does not render children while closed and not mounted", () => {
		render(
			<AnimatedCollapsible open={false}>
				<div>Filters</div>
			</AnimatedCollapsible>,
		);

		expect(screen.queryByText("Filters")).not.toBeInTheDocument();
	});

	it("renders children and marks the container visible when open", () => {
		const { container } = render(
			<AnimatedCollapsible open>
				<div>Filters</div>
			</AnimatedCollapsible>,
		);

		expect(screen.getByText("Filters")).toBeInTheDocument();
		expect(container.firstElementChild).toHaveAttribute("aria-hidden", "false");
		expect(container.firstElementChild).toHaveClass("overflow-hidden");
	});

	it("applies custom classes to the container and content wrapper", () => {
		const { container } = render(
			<AnimatedCollapsible
				open
				className="panel-container"
				contentClassName="panel-content"
			>
				<div>Filters</div>
			</AnimatedCollapsible>,
		);

		expect(container.firstElementChild).toHaveClass("panel-container");
		expect(screen.getByText("Filters").parentElement).toHaveClass(
			"panel-content",
		);
	});

	it("keeps children mounted while closing and removes them after the collapse timer", () => {
		vi.useFakeTimers();
		const { rerender } = render(
			<AnimatedCollapsible open>
				<div>Filters</div>
			</AnimatedCollapsible>,
		);

		rerender(
			<AnimatedCollapsible open={false}>
				<div>Filters</div>
			</AnimatedCollapsible>,
		);

		expect(screen.getByText("Filters")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(159);
		});
		expect(screen.getByText("Filters")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(1);
		});
		expect(screen.queryByText("Filters")).not.toBeInTheDocument();
	});

	it("uses zero-duration motion when reduced motion is preferred", () => {
		vi.mocked(window.matchMedia).mockImplementation((query: string) => ({
			matches: query === "(prefers-reduced-motion: reduce)",
			media: query,
			onchange: null,
			addEventListener: vi.fn(),
			removeEventListener: vi.fn(),
			addListener: vi.fn(),
			removeListener: vi.fn(),
			dispatchEvent: vi.fn(),
		}));

		const { container } = render(
			<AnimatedCollapsible open>
				<div>Filters</div>
			</AnimatedCollapsible>,
		);

		expect(container.firstElementChild).toHaveStyle({
			transitionDuration: "0ms",
		});
	});

	it("cleans up queued animation frames when unmounted during an opening transition", () => {
		const frames = mockAnimationFrame();
		const { unmount } = render(
			<AnimatedCollapsible open>
				<div>Filters</div>
			</AnimatedCollapsible>,
		);

		unmount();

		expect(frames.requestAnimationFrame).toHaveBeenCalled();
		expect(frames.cancelAnimationFrame).toHaveBeenCalled();
	});
});
