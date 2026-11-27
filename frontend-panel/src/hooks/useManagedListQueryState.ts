import { useCallback, useEffect, useMemo } from "react";
import {
	parsePageSizeSearchParam,
	parseSortSearchParam,
} from "@/lib/pagination";

type SetSearchParams = (
	nextInit: URLSearchParams,
	navigateOptions?: { replace?: boolean },
) => void;

export type ManagedListField<Value> = {
	/** URL parameter name. Every managed key is removed before re-serialization. */
	key: string;
	parse: (rawValue: string | null) => Value;
	/** Returns the string to write, or null to leave the parameter absent. */
	serialize: (value: Value) => string | null;
};

export type ManagedListQuerySchema<Query extends object> = {
	[Field in keyof Query]: ManagedListField<Query[Field]>;
};

export type ManagedListQueryUpdate<Query extends object> =
	| Partial<Query>
	| ((current: Readonly<Query>) => Partial<Query>);

/**
 * Declares an enum-backed URL field. Values outside `options` fall back to
 * `defaultValue`; the parameter is omitted when the value equals the default.
 */
export function managedEnumField<Value extends string>(
	key: string,
	options: readonly Value[],
	defaultValue: Value,
): ManagedListField<Value> {
	return {
		key,
		parse: (rawValue) =>
			options.includes(rawValue as Value) ? (rawValue as Value) : defaultValue,
		serialize: (value) => (value === defaultValue ? null : value),
	};
}

/**
 * Declares the pageSize field, reusing the shared pageSize param parsing so
 * every admin list accepts the same option set. Omitted at the default size.
 */
export function managedPageSizeField<Size extends number>(
	options: readonly Size[],
	defaultSize: Size,
): ManagedListField<Size> {
	return {
		key: "pageSize",
		parse: (rawValue) =>
			parsePageSizeSearchParam(rawValue, options, defaultSize),
		serialize: (value) => (value === defaultSize ? null : String(value)),
	};
}

/**
 * Declares a free-text field. The value is trimmed and empty strings are
 * omitted from the URL.
 */
export function managedStringField(key: string): ManagedListField<string> {
	return {
		key,
		parse: (rawValue) => rawValue?.trim() ?? "",
		serialize: (value) => {
			const trimmed = value.trim();
			return trimmed || null;
		},
	};
}

/**
 * Declares a sort-order field backed by parseSortSearchParam.
 */
export function managedSortField<SortBy extends string>(
	key: string,
	options: readonly SortBy[],
	defaultValue: SortBy,
): ManagedListField<SortBy> {
	return {
		key,
		parse: (rawValue) => parseSortSearchParam(rawValue, options, defaultValue),
		serialize: (value) => (value === defaultValue ? null : value),
	};
}

/**
 * Declares a field that is only ever removed from the URL. Use it to scrub
 * legacy parameter names (e.g. the old offset-based pagination) without
 * exposing them as managed query state.
 */
export function managedRemovedField(key: string): ManagedListField<null> {
	return {
		key,
		parse: () => null,
		serialize: () => null,
	};
}

function serializeQuery<Query extends object>(
	schema: ManagedListQuerySchema<Query>,
	query: Query,
): URLSearchParams {
	const params = new URLSearchParams();
	for (const field of Object.values(schema) as Array<
		ManagedListField<Query[keyof Query]>
	>) {
		const serialized = field.serialize(query[field.key as keyof Query]);
		if (serialized !== null) {
			params.set(field.key, serialized);
		}
	}
	return params;
}

/**
 * Schema-driven URL query state for managed lists. The schema declares every
 * managed parameter (name, parsing, serialization); the query object is
 * derived from the current search params and `setQuery` writes a normalized
 * parameter set back, preserving unmanaged params and dropping defaults.
 *
 * This replaces the per-page hand-rolled URL-sync effects while keeping
 * existing URL parameter names (bookmarks keep working).
 */
export function useManagedListQueryState<Query extends object>({
	schema,
	searchParams,
	setSearchParams,
}: {
	schema: ManagedListQuerySchema<Query>;
	searchParams: URLSearchParams;
	setSearchParams: SetSearchParams;
}) {
	const query = useMemo(() => {
		const nextQuery = {} as Query;
		for (const field of Object.values(schema) as Array<
			ManagedListField<Query[keyof Query]>
		>) {
			nextQuery[field.key as keyof Query] = field.parse(
				searchParams.get(field.key),
			);
		}
		return nextQuery;
	}, [schema, searchParams]);

	const managedKeys = useMemo(
		() =>
			new Set(
				(Object.values(schema) as Array<ManagedListField<unknown>>).map(
					(field) => field.key,
				),
			),
		[schema],
	);

	const mergeParams = useCallback(
		(nextManaged: URLSearchParams) => {
			const merged = new URLSearchParams(searchParams);
			for (const key of managedKeys) {
				merged.delete(key);
			}
			for (const [key, value] of nextManaged.entries()) {
				merged.set(key, value);
			}
			return merged;
		},
		[managedKeys, searchParams],
	);

	// Normalize the URL to the canonical serialization of the parsed query so
	// hand-edited or stale params (invalid enums, out-of-set page sizes,
	// legacy keys) converge on mount without a user-visible navigation.
	useEffect(() => {
		const normalized = mergeParams(serializeQuery(schema, query));
		if (normalized.toString() === searchParams.toString()) {
			return;
		}
		setSearchParams(normalized, { replace: true });
	}, [mergeParams, query, schema, searchParams, setSearchParams]);

	const setQuery = useCallback(
		(updates: ManagedListQueryUpdate<Query>) => {
			const patch = typeof updates === "function" ? updates(query) : updates;
			const nextQuery = { ...query, ...patch };
			setSearchParams(mergeParams(serializeQuery(schema, nextQuery)), {
				replace: true,
			});
		},
		[mergeParams, query, schema, setSearchParams],
	);

	return { query, setQuery };
}
