import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { AppFooter } from "@/components/layout/AppFooter";

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: (key: string, values?: Record<string, string | number>) =>
			values?.name ? `${key}:${values.name}` : key,
	}),
}));

describe("AppFooter", () => {
	it("keeps legal and account links without duplicating home or login navigation", () => {
		render(
			<MemoryRouter>
				<AppFooter />
			</MemoryRouter>,
		);

		const navigation = screen.getByRole("navigation", {
			name: "footer.navigation",
		});

		expect(
			within(navigation).queryByRole("link", { name: /nav\.home/ }),
		).not.toBeInTheDocument();
		expect(
			within(navigation).queryByRole("link", { name: /nav\.login/ }),
		).not.toBeInTheDocument();
		expect(
			within(navigation).getByRole("link", { name: /nav\.account/ }),
		).toHaveAttribute("href", "/account");
		expect(
			within(navigation).getByRole("link", { name: /nav\.tos/ }),
		).toHaveAttribute("href", "/tos");
		expect(
			within(navigation).getByRole("link", { name: /nav\.privacy/ }),
		).toHaveAttribute("href", "/privacy");
	});

	it("resets scroll when a footer navigation link is opened", () => {
		const scrollTo = vi.mocked(window.scrollTo);
		scrollTo.mockClear();

		render(
			<MemoryRouter>
				<AppFooter />
			</MemoryRouter>,
		);

		fireEvent.click(screen.getByRole("link", { name: /nav\.tos/ }));

		expect(scrollTo).toHaveBeenCalledWith({
			top: 0,
			left: 0,
			behavior: "auto",
		});
	});
});
