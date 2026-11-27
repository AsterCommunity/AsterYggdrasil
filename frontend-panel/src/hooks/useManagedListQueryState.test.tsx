import { act, render, screen } from "@testing-library/react";
import { MemoryRouter, useLocation, useSearchParams } from "react-router-dom";
import { describe, expect, it } from "vitest";
import {
	managedEnumField,
	managedPageSizeField,
	managedRemovedField,
	managedStringField,
	useManagedListQueryState,
} from "./useManagedListQueryState";

type UsersQuery = {
	keyword: string;
	pageSize: 10 | 20 | 50;
	role: "all" | "admin" | "user";
};

const usersSchema = {
	keyword: managedStringField("keyword"),
	pageSize: managedPageSizeField([10, 20, 50] as const, 20),
	role: managedEnumField("role", ["all", "admin", "user"] as const, "all"),
};

function Probe({ onQuery }: { onQuery?: (query: UsersQuery) => void }) {
	const [searchParams, setSearchParams] = useSearchParams();
	const location = useLocation();
	const { query, setQuery } = useManagedListQueryState<UsersQuery>({
		schema: {
			...usersSchema,
			legacy: managedRemovedField("offset"),
		},
		searchParams,
		setSearchParams,
	});
	onQuery?.(query);
	return (
		<div>
			<span data-testid="query">{JSON.stringify(query)}</span>
			<span data-testid="search">{location.search}</span>
			<button
				type="button"
				onClick={() => {
					setQuery({ keyword: "nova", role: "admin" });
				}}
			>
				set-keyword
			</button>
			<button
				type="button"
				onClick={() => {
					setQuery((current) => ({
						pageSize: current.pageSize === 20 ? 50 : 20,
					}));
				}}
			>
				toggle-size
			</button>
		</div>
	);
}

function renderProbe(
	initialUrl: string,
	onQuery?: (query: UsersQuery) => void,
) {
	return render(
		<MemoryRouter initialEntries={[initialUrl]}>
			<Probe onQuery={onQuery} />
		</MemoryRouter>,
	);
}

describe("useManagedListQueryState", () => {
	it("parses managed params into typed query values", () => {
		renderProbe("/?keyword=nova&pageSize=50&role=admin");
		const query = JSON.parse(screen.getByTestId("query").textContent ?? "");
		expect(query).toMatchObject({
			keyword: "nova",
			pageSize: 50,
			role: "admin",
		});
	});

	it("falls back to defaults for missing or invalid params", () => {
		renderProbe("/?pageSize=999&role=superuser");
		const query = JSON.parse(screen.getByTestId("query").textContent ?? "");
		expect(query).toMatchObject({
			keyword: "",
			pageSize: 20,
			role: "all",
		});
	});

	it("serializes non-default values and preserves unmanaged params", () => {
		renderProbe("/?flag=1");
		act(() => {
			screen.getByRole("button", { name: "set-keyword" }).click();
		});
		expect(screen.getByTestId("search").textContent).toBe(
			"?flag=1&keyword=nova&role=admin",
		);
	});

	it("keeps params for a function update and drops defaults it writes", () => {
		renderProbe("/?pageSize=50");
		act(() => {
			screen.getByRole("button", { name: "toggle-size" }).click();
		});
		// 20 is the default size, so the param is omitted entirely.
		expect(screen.getByTestId("search").textContent).toBe("");
	});

	it("normalizes invalid params away on mount and scrubs legacy keys", () => {
		renderProbe("/?offset=40&pageSize=999&role=suspended");
		expect(screen.getByTestId("search").textContent).toBe("");
	});
});
