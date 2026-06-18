import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "@/i18n";
import {
	UsersTableHeader,
	UsersTableRow,
} from "@/components/admin/admin-users-page/UsersTable";
import type { AdminUserInfo } from "@/types/api";

function user(overrides: Partial<AdminUserInfo> = {}): AdminUserInfo {
	return {
		id: 2,
		username: "alex",
		email: "alex@example.com",
		pending_email: null,
		operator_scopes: [],
		role: "user",
		status: "active",
		must_change_password: false,
		session_version: 1,
		profile_count: 1,
		active_session_count: 2,
		profile: {
			display_name: "Display Cat",
			avatar: {
				source: "upload",
				url_512: "https://example.test/avatar-512.webp",
				url_1024: "https://example.test/avatar-1024.webp",
				version: 3,
			},
		},
		email_verified_at: "2026-06-18T00:00:00Z",
		created_at: "2026-06-18T00:00:00Z",
		updated_at: "2026-06-18T01:00:00Z",
		...overrides,
	};
}

function renderRow(target: AdminUserInfo = user()) {
	const onDelete = vi.fn();
	const onEdit = vi.fn();
	const onRevokeSessions = vi.fn();
	const view = render(
		<table>
			<tbody>
				<UsersTableRow
					user={target}
					deletingId={null}
					revokingId={null}
					onDelete={onDelete}
					onEdit={onEdit}
					onRevokeSessions={onRevokeSessions}
				/>
			</tbody>
		</table>,
	);
	return { ...view, onDelete, onEdit, onRevokeSessions };
}

describe("UsersTableRow", () => {
	it("renders the user id in the first table cell", () => {
		renderRow(user({ id: 42 }));

		const cells = screen.getAllByRole("cell");
		expect(cells[0]).toHaveTextContent("42");
	});

	it("renders display name, username, email, and uploaded avatar", () => {
		const { container } = renderRow();

		expect(screen.getByText("Display Cat")).toBeInTheDocument();
		expect(screen.getByText("@alex · alex@example.com")).toBeInTheDocument();
		const img = container.querySelector("img");
		expect(img).toHaveAttribute("src", "https://example.test/avatar-512.webp");
	});

	it("falls back to username initials when display name and avatar are missing", () => {
		renderRow(
			user({
				username: "fallback",
				email: "fallback@example.com",
				profile: {
					display_name: "   ",
					avatar: {
						source: "none",
						url_512: null,
						url_1024: null,
						version: 0,
					},
				},
			}),
		);

		expect(screen.getByText("fallback")).toBeInTheDocument();
		expect(screen.getByText("fallback@example.com")).toBeInTheDocument();
		expect(screen.getByText("FA")).toBeInTheDocument();
	});

	it("renders the operator role badge distinctly", () => {
		renderRow(user({ role: "operator" }));

		expect(screen.getByText("Operator")).toBeInTheDocument();
		expect(screen.queryByText("User")).not.toBeInTheDocument();
	});

	it("opens details from row keyboard and click interactions", () => {
		const { onEdit } = renderRow();
		const row = screen.getByRole("row");

		fireEvent.click(row);
		fireEvent.keyDown(row, { key: "Enter" });
		fireEvent.keyDown(row, { key: " " });

		expect(onEdit).toHaveBeenCalledTimes(3);
	});

	it("keeps action clicks out of row navigation", () => {
		const { onDelete, onEdit, onRevokeSessions } = renderRow();

		fireEvent.click(screen.getByRole("button", { name: "Edit user" }));
		fireEvent.click(screen.getByRole("button", { name: "Revoke sessions" }));
		fireEvent.click(screen.getByRole("button", { name: "Delete user" }));

		expect(onEdit).toHaveBeenCalledTimes(1);
		expect(onRevokeSessions).toHaveBeenCalledTimes(1);
		expect(onDelete).toHaveBeenCalledTimes(1);
	});

	it("disables revoke when no sessions exist and blocks deleting initial admin", () => {
		renderRow(
			user({
				id: 1,
				active_session_count: 0,
				username: "root",
				email: "root@example.com",
			}),
		);

		expect(
			screen.getByRole("button", { name: "Revoke sessions" }),
		).toBeDisabled();
		expect(screen.getByRole("button", { name: "Delete user" })).toBeDisabled();
	});

	it("shows deleting and revoking spinner states", () => {
		const target = user({ id: 8 });
		render(
			<table>
				<tbody>
					<UsersTableRow
						user={target}
						deletingId={8}
						revokingId={8}
						onDelete={vi.fn()}
						onEdit={vi.fn()}
						onRevokeSessions={vi.fn()}
					/>
				</tbody>
			</table>,
		);

		const row = screen.getByRole("row");
		const buttons = within(row).getAllByRole("button");
		expect(buttons[1]).toBeDisabled();
		expect(buttons[2]).toBeDisabled();
		expect(row.querySelectorAll(".animate-spin")).toHaveLength(2);
	});
});

describe("UsersTableHeader", () => {
	it("sorts by id from the leading ID column", () => {
		const onSortChange = vi.fn();
		render(
			<table>
				<UsersTableHeader
					sortBy="username"
					sortOrder="asc"
					onSortChange={onSortChange}
				/>
			</table>,
		);

		fireEvent.click(screen.getByRole("button", { name: /ID/ }));

		expect(onSortChange).toHaveBeenCalledWith("id", "asc");
	});
});
