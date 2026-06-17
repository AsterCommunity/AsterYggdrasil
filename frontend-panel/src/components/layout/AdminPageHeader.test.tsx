import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AdminPageHeader } from "@/components/layout/AdminPageHeader";

describe("AdminPageHeader", () => {
	it("renders the page title and description in a plain header layout", () => {
		const { container } = render(
			<AdminPageHeader
				title="Audit log"
				description="Inspect account activity."
			/>,
		);

		expect(
			screen.getByRole("heading", { name: "Audit log" }),
		).toBeInTheDocument();
		expect(screen.getByText("Inspect account activity.")).toBeInTheDocument();
		expect(container.firstChild).toHaveClass("border-b");
		expect(container.querySelector("[class*='size-10']")).toBeNull();
	});

	it("renders actions and toolbar without introducing a card shell", () => {
		const { container } = render(
			<AdminPageHeader
				title="Settings"
				actions={<button type="button">Refresh</button>}
				toolbar={<button type="button">Filters</button>}
			/>,
		);

		expect(screen.getByRole("button", { name: "Refresh" })).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Filters" })).toBeInTheDocument();
		expect(container.firstChild).not.toHaveClass("rounded-lg");
		expect(container.firstChild).not.toHaveClass("bg-card");
	});
});
