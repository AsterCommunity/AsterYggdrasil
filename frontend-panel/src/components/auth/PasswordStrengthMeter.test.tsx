import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PasswordStrengthMeter } from "./PasswordStrengthMeter";

function activeSegments(container: HTMLElement) {
	return Array.from(
		container.querySelectorAll("span.rounded-full.transition-colors"),
	);
}

describe("PasswordStrengthMeter", () => {
	it.each([
		{
			expectedClass: "bg-red-400",
			expectedTextClass: "text-red-700",
			score: 0,
			value: "Weak",
		},
		{
			expectedClass: "bg-red-400",
			expectedTextClass: "text-red-700",
			score: 1,
			value: "Weak",
		},
		{
			expectedClass: "bg-amber-400",
			expectedTextClass: "text-amber-700",
			score: 2,
			value: "Medium",
		},
		{
			expectedClass: "bg-amber-400",
			expectedTextClass: "text-amber-700",
			score: 3,
			value: "Medium",
		},
		{
			expectedClass: "bg-emerald-400",
			expectedTextClass: "text-emerald-700",
			score: 4,
			value: "Strong",
		},
		{
			expectedClass: "bg-emerald-400",
			expectedTextClass: "text-emerald-700",
			score: 5,
			value: "Strong",
		},
	])(
		"uses $expectedClass for active segments at score $score",
		({ expectedClass, expectedTextClass, score, value }) => {
			const { container } = render(
				<PasswordStrengthMeter
					label="Password strength"
					value={value}
					score={score}
				/>,
			);

			const segments = activeSegments(container);
			const activeCount = Math.min(score, 4);

			expect(screen.getByText(value)).toHaveClass(expectedTextClass);
			for (const segment of segments.slice(0, activeCount)) {
				expect(segment).toHaveClass(expectedClass);
			}
			for (const segment of segments.slice(activeCount)) {
				expect(segment).toHaveClass("bg-black/10", "dark:bg-white/8");
				expect(segment).not.toHaveClass(expectedClass);
			}
		},
	);
});
