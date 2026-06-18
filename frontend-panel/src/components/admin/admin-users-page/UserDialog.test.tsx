import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "@/i18n";
import { UserDialog } from "@/components/admin/admin-users-page/UserDialog";

function renderDialog() {
	const onOpenChange = vi.fn();
	const onSubmit = vi.fn();

	render(
		<UserDialog
			open
			submitting={false}
			onOpenChange={onOpenChange}
			onSubmit={onSubmit}
		/>,
	);

	return { onOpenChange, onSubmit };
}

function fillRequiredFields() {
	fireEvent.change(screen.getByLabelText(/^Username/), {
		target: { value: "alex-1" },
	});
	fireEvent.change(screen.getByLabelText(/^Email/), {
		target: { value: "alex@example.com" },
	});
}

describe("UserDialog", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it("allows creating a user without an explicit password", () => {
		const { onSubmit } = renderDialog();

		fillRequiredFields();
		const submit = screen.getByRole("button", { name: "Create" });
		expect(submit).not.toBeDisabled();
		fireEvent.click(submit);

		expect(onSubmit).toHaveBeenCalledWith({
			email: "alex@example.com",
			password: null,
			must_change_password: false,
			role: "user",
			status: "active",
			username: "alex-1",
		});
	});

	it("trims blank create passwords into generated-password requests", () => {
		const { onSubmit } = renderDialog();

		fillRequiredFields();
		fireEvent.change(screen.getByLabelText("Password"), {
			target: { value: "   " },
		});
		fireEvent.click(screen.getByRole("button", { name: "Create" }));

		expect(onSubmit).toHaveBeenCalledWith(
			expect.objectContaining({ password: null }),
		);
	});

	it("rejects explicit passwords outside the allowed length", () => {
		const { onSubmit } = renderDialog();

		fillRequiredFields();
		fireEvent.change(screen.getByLabelText("Password"), {
			target: { value: "short" },
		});

		expect(
			screen.getByText("Password must be 8-128 characters."),
		).toBeInTheDocument();
		expect(screen.getByRole("button", { name: "Create" })).toBeDisabled();
		expect(onSubmit).not.toHaveBeenCalled();
	});

	it("submits the force password change flag", () => {
		const { onSubmit } = renderDialog();

		fillRequiredFields();
		fireEvent.click(
			screen.getByRole("switch", {
				name: "Require password change at next sign-in",
			}),
		);
		fireEvent.click(screen.getByRole("button", { name: "Create" }));

		expect(onSubmit).toHaveBeenCalledWith(
			expect.objectContaining({ must_change_password: true }),
		);
	});

	it("offers the operator role option for scoped admin access", async () => {
		renderDialog();

		fireEvent.click(screen.getAllByRole("combobox")[0]);

		expect(
			await screen.findByRole("option", { name: "Operator" }),
		).toBeInTheDocument();
	});
});
