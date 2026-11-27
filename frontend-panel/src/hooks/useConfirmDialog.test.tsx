import { act, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useConfirmDialog } from "./useConfirmDialog";

type Target = { id: number; name: string };

function Probe({
	onConfirm,
	onRequestConfirm,
}: {
	onConfirm: (target: Target) => void;
	onRequestConfirm: (requestConfirm: (target: Target) => void) => void;
}) {
	const { confirmTarget, requestConfirm, dialogProps } =
		useConfirmDialog<Target>(onConfirm);
	onRequestConfirm(requestConfirm);
	return (
		<div>
			<button type="button" onClick={dialogProps.onConfirm}>
				confirm:{confirmTarget?.name}
			</button>
			<button type="button" onClick={() => dialogProps.onOpenChange(false)}>
				close
			</button>
		</div>
	);
}

describe("useConfirmDialog", () => {
	it("opens with the requested target and confirms with the same target", () => {
		let requestConfirm: (target: Target) => void = () => {};
		const onConfirm = vi.fn();
		render(
			<Probe
				onConfirm={onConfirm}
				onRequestConfirm={(value) => {
					requestConfirm = value;
				}}
			/>,
		);

		act(() => {
			requestConfirm({ id: 3, name: "nova" });
		});
		act(() => {
			screen.getByRole("button", { name: "confirm:nova" }).click();
		});

		expect(onConfirm).toHaveBeenCalledWith({ id: 3, name: "nova" });
		expect(
			screen.getByRole("button", { name: "confirm:" }),
		).toBeInTheDocument();
	});

	it("closes without running the action when dismissed", () => {
		let requestConfirm: (target: Target) => void = () => {};
		const onConfirm = vi.fn();
		render(
			<Probe
				onConfirm={onConfirm}
				onRequestConfirm={(value) => {
					requestConfirm = value;
				}}
			/>,
		);

		act(() => {
			requestConfirm({ id: 5, name: "orbit" });
		});
		act(() => {
			screen.getByRole("button", { name: "close" }).click();
		});

		expect(onConfirm).not.toHaveBeenCalled();
	});
});
