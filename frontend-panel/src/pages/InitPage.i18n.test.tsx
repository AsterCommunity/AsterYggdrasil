import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { useInitStatusStore } from "@/stores/initStatusStore";
import InitPage from "./InitPage";

/**
 * Rendered-locale coverage for InitPage (real i18n pipeline, no
 * react-i18next mock). Behavioral tests live in InitPage.test.tsx.
 */

function renderInitPage() {
	return render(
		<MemoryRouter initialEntries={["/init"]}>
			<InitPage />
		</MemoryRouter>,
	);
}

describe("InitPage rendered locale", () => {
	beforeEach(() => {
		useInitStatusStore.setState({
			checking: false,
			initialized: false,
			error: null,
		});
	});

	it("renders the init form on the standalone init route", async () => {
		renderInitPage();

		expect(
			await screen.findByRole("heading", {
				level: 1,
				name: "Finish initial setup",
			}),
		).toBeInTheDocument();
		expect(screen.getByLabelText("Username")).toBeInTheDocument();
		expect(screen.getByLabelText("Email")).toBeInTheDocument();
		expect(screen.getByLabelText("Password")).toBeInTheDocument();
		expect(screen.getByLabelText("Public site URL")).toBeInTheDocument();
	});
});
