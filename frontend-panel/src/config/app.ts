function readMetaValue(name: string) {
	if (typeof document === "undefined") return undefined;
	const value = document
		.querySelector(`meta[name="${name}"]`)
		?.getAttribute("content")
		?.trim();

	if (!value) return undefined;
	if (value.startsWith("%") && value.endsWith("%")) return undefined;
	return value;
}

export const config = {
	apiBaseUrl: import.meta.env.VITE_API_BASE_URL ?? "/api/v1",
	rootBaseUrl: import.meta.env.VITE_ROOT_BASE_URL ?? "",
	appName: "AsterYggdrasil",
	appVersion:
		readMetaValue("asteryggdrasil-version") ??
		(import.meta.env.DEV ? "dev" : "unknown"),
} as const;
