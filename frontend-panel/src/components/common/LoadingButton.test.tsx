import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "@/i18n";
import { LoadingButton } from "./LoadingButton";
import { RefreshButton } from "./RefreshButton";

describe("RefreshButton", () => {
	it("renders the shared refresh label with the unified refresh glyph", () => {
		render(<RefreshButton />);

		const button = screen.getByRole("button", { name: "Refresh" });
		expect(button).toBeInTheDocument();
		expect(button).toHaveAttribute("type", "button");
		expect(button.querySelector("svg")).not.toHaveClass("animate-spin");
	});

	it("renders a custom label for retry/resend variants", () => {
		render(<RefreshButton>Retry health check</RefreshButton>);

		expect(
			screen.getByRole("button", { name: "Retry health check" }),
		).toBeInTheDocument();
	});

	it("shows the spinner and disables while loading, then recovers", () => {
		const onClick = vi.fn();
		const { rerender } = render(<RefreshButton loading onClick={onClick} />);

		const button = screen.getByRole("button", { name: "Refresh" });
		expect(button).toBeDisabled();
		expect(button.querySelector("svg")).toHaveClass("animate-spin");
		fireEvent.click(button);
		expect(onClick).not.toHaveBeenCalled();

		rerender(<RefreshButton onClick={onClick} />);
		expect(screen.getByRole("button", { name: "Refresh" })).toBeEnabled();
		expect(onClick).toHaveBeenCalledTimes(0);
	});

	it("keeps caller-disabled state even when not loading", () => {
		render(<RefreshButton disabled />);

		expect(screen.getByRole("button", { name: "Refresh" })).toBeDisabled();
	});
});

describe("LoadingButton", () => {
	it("renders the idle semantic icon next to the label", () => {
		render(<LoadingButton icon="FloppyDisk">Save changes</LoadingButton>);

		const button = screen.getByRole("button", { name: "Save changes" });
		expect(button).toBeEnabled();
		expect(button.querySelector("svg")).not.toHaveClass("animate-spin");
	});

	it("renders a text-only button when no icon is given", () => {
		render(<LoadingButton loading={false}>Create</LoadingButton>);

		const button = screen.getByRole("button", { name: "Create" });
		expect(button.querySelector("svg")).toBeNull();
	});

	it("swaps in a spinner, disables re-entry, and keeps the label while loading", () => {
		const onClick = vi.fn();
		render(
			<LoadingButton loading icon="EnvelopeSimple" onClick={onClick}>
				Send invitation
			</LoadingButton>,
		);

		const button = screen.getByRole("button", { name: "Send invitation" });
		expect(button).toBeDisabled();
		expect(button.querySelector("svg")).toHaveClass("animate-spin");
		fireEvent.click(button);
		expect(onClick).not.toHaveBeenCalled();
	});

	it("shows the spinner alone for icon-less async buttons", () => {
		render(<LoadingButton loading>Delete</LoadingButton>);

		const button = screen.getByRole("button", { name: "Delete" });
		expect(button.querySelector("svg")).toHaveClass("animate-spin");
	});
});
