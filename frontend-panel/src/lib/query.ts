type QueryPrimitive = string | number | boolean | null | undefined;
type QueryValue = QueryPrimitive | QueryPrimitive[];

export function withQuery(
	path: string,
	params: { [Key in string]?: QueryValue },
) {
	const query = new URLSearchParams();
	for (const [key, value] of Object.entries(params)) {
		if (value === null || value === undefined || value === "") continue;
		if (Array.isArray(value)) {
			const values = value
				.filter((item) => item !== null && item !== undefined && item !== "")
				.map(String);
			if (values.length > 0) {
				query.set(key, values.join(","));
			}
			continue;
		}
		query.set(key, String(value));
	}
	const rawQuery = query.toString();
	return rawQuery ? `${path}?${rawQuery}` : path;
}
