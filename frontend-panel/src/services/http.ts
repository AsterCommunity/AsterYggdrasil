import type {
	AxiosInstance,
	AxiosRequestConfig,
	AxiosResponse,
	InternalAxiosRequestConfig,
} from "axios";
import axios, { AxiosHeaders } from "axios";
import { config } from "@/config/app";
import type { ApiErrorInfo, ApiResponse, AsterErrorCode } from "@/types/api";
import { getCsrfHeaderName, getCsrfToken } from "./csrf";

type FrontendTransportErrorCode = "network_error" | "request_timeout";
export type ApiClientErrorCode = AsterErrorCode | FrontendTransportErrorCode;

const client: AxiosInstance = axios.create({
	baseURL: config.apiBaseUrl,
	timeout: 15_000,
	headers: { "Content-Type": "application/json" },
	withCredentials: true,
});

const rootClient: AxiosInstance = axios.create({
	baseURL: config.rootBaseUrl,
	timeout: 15_000,
	headers: { "Content-Type": "application/json" },
	withCredentials: true,
});

export type ApiRequestConfig = Pick<
	AxiosRequestConfig,
	"data" | "headers" | "params" | "signal"
>;

type RetriableRequestConfig = InternalAxiosRequestConfig & {
	_retry?: boolean;
};

export class ApiError extends Error {
	code: ApiClientErrorCode;
	retryable?: boolean;

	constructor(
		code: ApiClientErrorCode,
		message: string,
		details: Pick<ApiErrorInfo, "retryable"> = {},
	) {
		super(message);
		this.code = code;
		this.retryable = details.retryable ?? undefined;
	}
}

function isApiEnvelope(value: unknown): value is ApiResponse<unknown> {
	if (typeof value !== "object" || value === null) return false;
	return "code" in value && "msg" in value;
}

function normalizeApiErrorInfo(
	value: unknown,
): Pick<ApiErrorInfo, "retryable"> {
	if (typeof value !== "object" || value === null) return {};
	const retryable = "retryable" in value ? value.retryable : undefined;
	return typeof retryable === "boolean" ? { retryable } : {};
}

function extractApiError(error: unknown): ApiError | null {
	if (error instanceof ApiError) return error;
	if (typeof error !== "object" || error === null) return null;
	if (isRequestCanceled(error)) return null;

	if (isRequestTimeout(error)) {
		return new ApiError("request_timeout", "Request timed out", {
			retryable: true,
		});
	}

	if (isNetworkError(error)) {
		return new ApiError("network_error", "Network error", {
			retryable: true,
		});
	}

	const response =
		"response" in error && typeof error.response === "object"
			? error.response
			: null;
	const data = response && "data" in response ? response.data : null;
	if (!isApiEnvelope(data)) return null;
	if (typeof data.code !== "string" || typeof data.msg !== "string") {
		return null;
	}

	return new ApiError(
		data.code as AsterErrorCode,
		data.msg || "API request failed",
		normalizeApiErrorInfo(data.error),
	);
}

export function isApiConnectionError(error: unknown): boolean {
	return (
		error instanceof ApiError &&
		(error.code === "network_error" || error.code === "request_timeout")
	);
}

function isTokenAuthError(error: unknown) {
	return (
		error instanceof ApiError &&
		(error.code === "auth.token_invalid" || error.code === "auth.token_expired")
	);
}

function isRequestCanceled(error: unknown): boolean {
	if (typeof axios.isCancel === "function" && axios.isCancel(error)) {
		return true;
	}
	if (typeof error !== "object" || error === null) {
		return false;
	}
	const code = "code" in error ? error.code : null;
	const name = "name" in error ? error.name : null;
	return code === "ERR_CANCELED" || name === "AbortError";
}

function getErrorCode(error: object): string | null {
	const code = "code" in error ? error.code : null;
	return typeof code === "string" ? code : null;
}

function getErrorMessage(error: object): string | null {
	const message = "message" in error ? error.message : null;
	return typeof message === "string" ? message : null;
}

function isRequestTimeout(error: object): boolean {
	const code = getErrorCode(error);
	const message = getErrorMessage(error)?.toLowerCase();
	return (
		code === "ECONNABORTED" ||
		code === "ETIMEDOUT" ||
		message?.includes("timeout") === true
	);
}

function isNetworkError(error: object): boolean {
	if (axios.isAxiosError(error) && !error.response) {
		return true;
	}

	const code = getErrorCode(error);
	if (code === "ERR_NETWORK") {
		return true;
	}

	const message = getErrorMessage(error);
	const normalizedMessage = message?.toLowerCase();
	return (
		message === "Network Error" ||
		normalizedMessage === "network error" ||
		message === "Failed to fetch" ||
		message === "Load failed"
	);
}

const SKIP_REFRESH_PATHS = [
	"/auth/refresh",
	"/auth/login",
	"/auth/register",
	"/auth/logout",
	"/auth/check",
	"/auth/setup",
	"/auth/passkeys/login/start",
	"/auth/passkeys/login/finish",
	"/auth/external-auth/providers",
];

function shouldSkipRefresh(url: string) {
	return SKIP_REFRESH_PATHS.some((path) => url.endsWith(path));
}

function isUnsafeMethod(method?: string) {
	return !["get", "head", "options", "trace"].includes(
		(method ?? "get").toLowerCase(),
	);
}

function hasHeader(
	headers: InternalAxiosRequestConfig["headers"],
	name: string,
): boolean {
	if (!headers) return false;
	if ("get" in headers && typeof headers.get === "function") {
		return headers.get(name) != null;
	}
	return Object.keys(headers).some(
		(key) => key.toLowerCase() === name.toLowerCase(),
	);
}

function setHeader(
	request: InternalAxiosRequestConfig,
	name: string,
	value: string,
) {
	if (
		request.headers &&
		"set" in request.headers &&
		typeof request.headers.set === "function"
	) {
		request.headers.set(name, value);
		return;
	}

	request.headers = AxiosHeaders.from(request.headers ?? {});
	request.headers.set(name, value);
}

function isFormDataPayload(value: unknown): value is FormData {
	return typeof FormData !== "undefined" && value instanceof FormData;
}

function suppressHeader(request: InternalAxiosRequestConfig, name: string) {
	if (
		request.headers &&
		"set" in request.headers &&
		typeof request.headers.set === "function"
	) {
		request.headers.set(name, false);
		return;
	}

	request.headers = AxiosHeaders.from(request.headers ?? {});
	request.headers.set(name, false);
}

export function formatUnknownError(error: unknown): string {
	if (error instanceof Error && error.message) return error.message;
	if (typeof error === "string" && error.trim()) return error;
	return "Request failed";
}

async function unwrap<T>(
	promise: Promise<AxiosResponse<ApiResponse<T>>>,
): Promise<T> {
	try {
		const axiosResponse = await promise;
		const { data: response } = axiosResponse;
		if (axiosResponse.status === 204) {
			return undefined as T;
		}
		if (response === undefined || response === null) {
			return undefined as T;
		}
		if (!isApiEnvelope(response)) {
			throw new ApiError(
				"internal_server_error",
				"Invalid API response envelope",
			);
		}
		if (response.code !== "success") {
			throw new ApiError(
				response.code,
				response.msg || "API request failed",
				normalizeApiErrorInfo(response.error),
			);
		}
		return response.data as T;
	} catch (error) {
		throw extractApiError(error) ?? error;
	}
}

function createApi(axiosClient: AxiosInstance) {
	return {
		get: <T>(url: string, requestConfig?: ApiRequestConfig) =>
			unwrap<T>(axiosClient.get<ApiResponse<T>>(url, requestConfig)),
		post: <T, TBody = never>(
			url: string,
			data?: TBody,
			requestConfig?: ApiRequestConfig,
		) => unwrap<T>(axiosClient.post<ApiResponse<T>>(url, data, requestConfig)),
		put: <T, TBody = never>(
			url: string,
			data?: TBody,
			requestConfig?: ApiRequestConfig,
		) => unwrap<T>(axiosClient.put<ApiResponse<T>>(url, data, requestConfig)),
		patch: <T, TBody = never>(
			url: string,
			data?: TBody,
			requestConfig?: ApiRequestConfig,
		) => unwrap<T>(axiosClient.patch<ApiResponse<T>>(url, data, requestConfig)),
		delete: <T>(url: string, requestConfig?: ApiRequestConfig) =>
			unwrap<T>(axiosClient.delete<ApiResponse<T>>(url, requestConfig)),
		client: axiosClient,
	};
}

function attachCsrfHeader(request: InternalAxiosRequestConfig) {
	const csrfToken = getCsrfToken();
	if (!csrfToken || !isUnsafeMethod(request.method)) return request;
	const csrfHeaderName = getCsrfHeaderName();
	if (!hasHeader(request.headers, csrfHeaderName)) {
		setHeader(request, csrfHeaderName, csrfToken);
	}
	return request;
}

function prepareRequest(request: InternalAxiosRequestConfig) {
	if (isFormDataPayload(request.data)) {
		suppressHeader(request, "Content-Type");
	}
	return attachCsrfHeader(request);
}

client.interceptors.request.use(prepareRequest);
rootClient.interceptors.request.use(prepareRequest);

let isRefreshing = false;
let refreshPromise: Promise<void> | null = null;

client.interceptors.response.use(
	(response) => response,
	async (error) => {
		if (isRequestCanceled(error)) {
			return Promise.reject(error);
		}

		const original = error.config as RetriableRequestConfig | undefined;
		const url = original?.url ?? "";
		const apiError = extractApiError(error);
		if (
			error.response?.status === 401 &&
			original &&
			!original._retry &&
			!shouldSkipRefresh(url) &&
			isTokenAuthError(apiError)
		) {
			original._retry = true;
			if (!isRefreshing) {
				isRefreshing = true;
				refreshPromise = (async () => {
					const { useAuthStore } = await import("@/stores/authStore");
					await useAuthStore.getState().refresh();
				})().finally(() => {
					isRefreshing = false;
					refreshPromise = null;
				});
			}
			try {
				await refreshPromise;
				return client(original);
			} catch (refreshError) {
				const apiRefreshError = extractApiError(refreshError) ?? refreshError;
				if (!isApiConnectionError(apiRefreshError)) {
					const { useAuthStore } = await import("@/stores/authStore");
					useAuthStore.getState().clear();
				}
				return Promise.reject(apiRefreshError);
			}
		}

		return Promise.reject(apiError ?? error);
	},
);

const versionedApi = createApi(client);

export const api = {
	...versionedApi,
	root: createApi(rootClient),
	client,
	rootClient,
};
