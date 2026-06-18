import { useReducer } from "react";
import {
	parseOffsetSearchParam,
	parsePageSizeSearchParam,
	parseSortOrderSearchParam,
	parseSortSearchParam,
	type SortOrder,
} from "@/lib/pagination";
import type {
	AdminUserInfo,
	AdminUserSortBy,
	UserRole,
	UserStatus,
} from "@/types/api";

export const USER_PAGE_SIZE_OPTIONS = [10, 20, 50] as const;
export const DEFAULT_USER_PAGE_SIZE = 20 as const;
export const USER_SORT_BY_OPTIONS = [
	"id",
	"username",
	"email",
	"role",
	"status",
	"created_at",
	"updated_at",
] as const satisfies readonly AdminUserSortBy[];
export const DEFAULT_SORT_BY = "created_at" as const satisfies AdminUserSortBy;
export const DEFAULT_SORT_ORDER = "desc" as const satisfies SortOrder;

export type UserFilterValue<T extends string> = "__all__" | T;

export type AdminUsersPageState = {
	createDialogOpen: boolean;
	debouncedKeyword: string;
	deletingId: number | null;
	deletingUser: AdminUserInfo | null;
	items: AdminUserInfo[];
	keyword: string;
	loading: boolean;
	offset: number;
	pageSize: (typeof USER_PAGE_SIZE_OPTIONS)[number];
	revokingId: number | null;
	revokingUser: AdminUserInfo | null;
	role: UserFilterValue<UserRole>;
	sortBy: AdminUserSortBy;
	sortOrder: SortOrder;
	status: UserFilterValue<UserStatus>;
	submitting: boolean;
	total: number;
};

export type AdminUsersPageAction =
	| { type: "createDialogOpen"; value: boolean }
	| { type: "debouncedKeyword"; value: string }
	| { type: "deletingId"; value: number | null }
	| { type: "deletingUser"; value: AdminUserInfo | null }
	| { type: "keyword"; value: string }
	| { type: "loadStart" }
	| { type: "loadSuccess"; items: AdminUserInfo[]; total: number }
	| { type: "loading"; value: boolean }
	| { type: "offset"; value: number | ((current: number) => number) }
	| { type: "pageSize"; value: (typeof USER_PAGE_SIZE_OPTIONS)[number] }
	| { type: "resetFilters" }
	| { type: "revokingId"; value: number | null }
	| { type: "revokingUser"; value: AdminUserInfo | null }
	| { type: "role"; value: UserFilterValue<UserRole> }
	| { type: "sort"; sortBy: AdminUserSortBy; sortOrder: SortOrder }
	| { type: "status"; value: UserFilterValue<UserStatus> }
	| { type: "submitting"; value: boolean };

function reducer(
	state: AdminUsersPageState,
	action: AdminUsersPageAction,
): AdminUsersPageState {
	switch (action.type) {
		case "createDialogOpen":
			return { ...state, createDialogOpen: action.value };
		case "debouncedKeyword":
			return { ...state, debouncedKeyword: action.value };
		case "deletingId":
			return { ...state, deletingId: action.value };
		case "deletingUser":
			return { ...state, deletingUser: action.value };
		case "keyword":
			return { ...state, keyword: action.value, offset: 0 };
		case "loadStart":
			return { ...state, loading: true };
		case "loadSuccess":
			return {
				...state,
				items: action.items,
				loading: false,
				total: action.total,
			};
		case "loading":
			return { ...state, loading: action.value };
		case "offset":
			return {
				...state,
				offset:
					typeof action.value === "function"
						? action.value(state.offset)
						: action.value,
			};
		case "pageSize":
			return { ...state, pageSize: action.value, offset: 0 };
		case "resetFilters":
			return {
				...state,
				debouncedKeyword: "",
				keyword: "",
				offset: 0,
				role: "__all__",
				status: "__all__",
			};
		case "revokingId":
			return { ...state, revokingId: action.value };
		case "revokingUser":
			return { ...state, revokingUser: action.value };
		case "role":
			return { ...state, role: action.value, offset: 0 };
		case "sort":
			return {
				...state,
				offset: 0,
				sortBy: action.sortBy,
				sortOrder: action.sortOrder,
			};
		case "status":
			return { ...state, status: action.value, offset: 0 };
		case "submitting":
			return { ...state, submitting: action.value };
	}
}

export function useAdminUsersPageState(searchParams: URLSearchParams) {
	return useReducer(reducer, searchParams, (params): AdminUsersPageState => {
		const keyword = params.get("keyword") ?? "";
		return {
			createDialogOpen: false,
			debouncedKeyword: keyword,
			deletingId: null,
			deletingUser: null,
			items: [],
			keyword,
			loading: true,
			offset: parseOffsetSearchParam(params.get("offset")),
			pageSize: parsePageSizeSearchParam(
				params.get("pageSize"),
				USER_PAGE_SIZE_OPTIONS,
				DEFAULT_USER_PAGE_SIZE,
			),
			revokingId: null,
			revokingUser: null,
			role: parseRole(params.get("role")),
			sortBy: parseSortSearchParam(
				params.get("sortBy"),
				USER_SORT_BY_OPTIONS,
				DEFAULT_SORT_BY,
			),
			sortOrder: parseSortOrderSearchParam(
				params.get("sortOrder"),
				DEFAULT_SORT_ORDER,
			),
			status: parseStatus(params.get("status")),
			submitting: false,
			total: 0,
		};
	});
}

function parseRole(value: string | null): UserFilterValue<UserRole> {
	return value === "admin" || value === "operator" || value === "user"
		? value
		: "__all__";
}

function parseStatus(value: string | null): UserFilterValue<UserStatus> {
	return value === "active" || value === "disabled" ? value : "__all__";
}
