import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AdminFilterToolbar } from "@/components/common/AdminFilterToolbar";

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (key: string) => key,
	}),
}));

function filterTrigger() {
	return screen.getByRole("button", { name: /admin\.(show|hide)Filters/ });
}

describe("AdminFilterToolbar", () => {
	it("keeps filter controls collapsed until the filter button is clicked", () => {
		render(
			<AdminFilterToolbar activeFilterCount={0}>
				<input aria-label="keyword" />
			</AdminFilterToolbar>,
		);

		expect(screen.queryByLabelText("keyword")).not.toBeInTheDocument();
		expect(filterTrigger()).toHaveTextContent("admin.showFilters");
		expect(filterTrigger()).toHaveAttribute("aria-expanded", "false");

		fireEvent.click(filterTrigger());

		expect(filterTrigger()).toHaveTextContent("admin.hideFilters");
		expect(filterTrigger()).toHaveAttribute("aria-expanded", "true");
		expect(screen.getByLabelText("keyword")).toBeInTheDocument();
	});

	it("keeps the animated panel mounted during close and unmounts it after the collapse duration", () => {
		vi.useFakeTimers();

		render(
			<AdminFilterToolbar activeFilterCount={0} defaultOpen>
				<input aria-label="keyword" />
			</AdminFilterToolbar>,
		);

		expect(screen.getByLabelText("keyword")).toBeInTheDocument();

		fireEvent.click(filterTrigger());

		expect(filterTrigger()).toHaveAttribute("aria-expanded", "false");
		expect(screen.getByLabelText("keyword")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(159);
		});
		expect(screen.getByLabelText("keyword")).toBeInTheDocument();

		act(() => {
			vi.advanceTimersByTime(1);
		});
		expect(screen.queryByLabelText("keyword")).not.toBeInTheDocument();
	});

	it("shows active filter state and clears filters without expanding controls", () => {
		const onResetFilters = vi.fn();

		render(
			<AdminFilterToolbar activeFilterCount={2} onResetFilters={onResetFilters}>
				<input aria-label="keyword" />
			</AdminFilterToolbar>,
		);

		expect(screen.queryByLabelText("keyword")).not.toBeInTheDocument();
		expect(screen.getByText("admin.filtersActive")).toBeInTheDocument();
		expect(screen.getByText("2")).toBeInTheDocument();
		expect(filterTrigger()).toHaveClass("h-8");

		fireEvent.click(screen.getByRole("button", { name: "admin.clearFilters" }));

		expect(onResetFilters).toHaveBeenCalledTimes(1);
		expect(screen.queryByLabelText("keyword")).not.toBeInTheDocument();
	});

	it("omits the reset action when filters are active but no reset callback is provided", () => {
		render(
			<AdminFilterToolbar activeFilterCount={1}>
				<input aria-label="keyword" />
			</AdminFilterToolbar>,
		);

		expect(screen.getByText("admin.filtersActive")).toBeInTheDocument();
		expect(
			screen.queryByRole("button", { name: "admin.clearFilters" }),
		).toBeNull();
	});

	it("can render controls expanded by default", () => {
		render(
			<AdminFilterToolbar activeFilterCount={0} defaultOpen>
				<input aria-label="keyword" />
			</AdminFilterToolbar>,
		);

		expect(filterTrigger()).toHaveTextContent("admin.hideFilters");
		expect(filterTrigger()).toHaveAttribute("aria-expanded", "true");
		expect(screen.getByLabelText("keyword")).toBeInTheDocument();
	});

	it("applies inline layout and custom classes", () => {
		const { container } = render(
			<AdminFilterToolbar
				activeFilterCount={0}
				className="toolbar-root"
				contentClassName="toolbar-content"
				inline
			>
				<input aria-label="keyword" />
			</AdminFilterToolbar>,
		);

		expect(container.firstElementChild).toHaveClass("contents", "toolbar-root");

		fireEvent.click(filterTrigger());

		const content = screen.getByLabelText("keyword").parentElement;
		expect(content).toHaveClass("toolbar-content");
		expect(content?.parentElement?.parentElement).toHaveClass("basis-full");
	});

	it("handles a zero active filter count without rendering status copy or badge", () => {
		render(
			<AdminFilterToolbar activeFilterCount={0}>
				<input aria-label="keyword" />
			</AdminFilterToolbar>,
		);

		expect(screen.queryByText("admin.filtersActive")).toBeNull();
		expect(screen.queryByText("0")).toBeNull();
		expect(
			screen.queryByRole("button", { name: "admin.clearFilters" }),
		).toBeNull();
	});
});
