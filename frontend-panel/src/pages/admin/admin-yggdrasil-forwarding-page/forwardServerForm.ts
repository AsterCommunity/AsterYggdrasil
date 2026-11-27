import type {
	AdminYggdrasilSessionForwardServerInfo,
	CreateYggdrasilSessionForwardServerRequest,
	UpdateYggdrasilSessionForwardServerRequest,
	YggdrasilSessionForwardServerSortBy,
} from "@/types/api";

export const PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
export const DEFAULT_PAGE_SIZE = 20 as const;
export const SORT_OPTIONS = [
	"call_order",
	"id",
] as const satisfies readonly YggdrasilSessionForwardServerSortBy[];
export const DEFAULT_SORT = "call_order" satisfies ForwardSort;

export type ForwardSort = YggdrasilSessionForwardServerSortBy;

export type ForwardForm = {
	baseUrl: string;
	displayName: string;
	enabled: boolean;
	priority: string;
	textureForwardEnabled: boolean;
	timeoutMs: string;
	weight: string;
};

export function emptyForm(): ForwardForm {
	return {
		baseUrl: "",
		displayName: "",
		enabled: true,
		priority: "100",
		textureForwardEnabled: false,
		timeoutMs: "1500",
		weight: "1",
	};
}

export function formFromServer(
	server: AdminYggdrasilSessionForwardServerInfo,
): ForwardForm {
	return {
		baseUrl: server.base_url ?? "",
		displayName: server.display_name,
		enabled: server.enabled,
		priority: String(server.priority),
		textureForwardEnabled: server.texture_forward_enabled,
		timeoutMs: String(server.timeout_ms),
		weight: String(server.weight),
	};
}

function numberFromField(value: string, fallback: number) {
	const parsed = Number.parseInt(value, 10);
	return Number.isFinite(parsed) ? parsed : fallback;
}

export function parseForwardSort(value: string | null): ForwardSort {
	return SORT_OPTIONS.includes(value as ForwardSort)
		? (value as ForwardSort)
		: DEFAULT_SORT;
}

export function createPayload(
	form: ForwardForm,
): CreateYggdrasilSessionForwardServerRequest {
	return {
		base_url: form.baseUrl.trim(),
		display_name: form.displayName.trim(),
		enabled: form.enabled,
		priority: numberFromField(form.priority, 100),
		texture_forward_enabled: form.textureForwardEnabled,
		timeout_ms: numberFromField(form.timeoutMs, 1500),
		weight: numberFromField(form.weight, 1),
	};
}

export function updatePayload(
	form: ForwardForm,
	server: AdminYggdrasilSessionForwardServerInfo,
): UpdateYggdrasilSessionForwardServerRequest {
	// `server` is the snapshot captured when the edit session opened. Only
	// fields the admin actually changed from that baseline are sent: the row
	// can legitimately change behind the dialog (e.g. an inline disable
	// completing mid-edit), and resubmitting untouched fields would silently
	// revert it. Every field of the update request is optional and applied
	// only when provided.
	const nextPriority = numberFromField(form.priority, server.priority);
	const nextTimeoutMs = numberFromField(form.timeoutMs, server.timeout_ms);
	const nextWeight = numberFromField(form.weight, server.weight);
	const nextTextureForward = server.local ? false : form.textureForwardEnabled;
	return {
		...(form.displayName.trim() !== server.display_name
			? { display_name: form.displayName.trim() }
			: {}),
		...(!server.local && form.baseUrl.trim() !== (server.base_url ?? "")
			? { base_url: form.baseUrl.trim() }
			: {}),
		...(form.enabled !== server.enabled ? { enabled: form.enabled } : {}),
		...(nextPriority !== server.priority ? { priority: nextPriority } : {}),
		...(nextTextureForward !== server.texture_forward_enabled
			? { texture_forward_enabled: nextTextureForward }
			: {}),
		...(nextTimeoutMs !== server.timeout_ms
			? { timeout_ms: nextTimeoutMs }
			: {}),
		...(nextWeight !== server.weight ? { weight: nextWeight } : {}),
	};
}
