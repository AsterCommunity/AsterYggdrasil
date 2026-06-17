import { AxiosError, CanceledError } from "axios";
import { afterEach, describe, expect, it } from "vitest";
import { ApiError, api } from "@/services/http";

const originalAdapter = api.client.defaults.adapter;

describe("http transport errors", () => {
	afterEach(() => {
		api.client.defaults.adapter = originalAdapter;
	});

	it("normalizes axios network failures without a response", async () => {
		api.client.defaults.adapter = () =>
			Promise.reject(new AxiosError("Network Error", "ERR_NETWORK"));

		await expect(api.get("/offline")).rejects.toMatchObject({
			code: "network_error",
			message: "Network error",
			retryable: true,
		});
		await expect(api.get("/offline")).rejects.toBeInstanceOf(ApiError);
	});

	it("normalizes request timeouts before generic network failures", async () => {
		api.client.defaults.adapter = () =>
			Promise.reject(
				new AxiosError("timeout of 15000ms exceeded", "ECONNABORTED"),
			);

		await expect(api.get("/slow")).rejects.toMatchObject({
			code: "request_timeout",
			message: "Request timed out",
			retryable: true,
		});
	});

	it("does not rewrite user-canceled requests as network failures", async () => {
		api.client.defaults.adapter = () =>
			Promise.reject(new CanceledError("canceled by test"));

		await expect(api.get("/cancelled")).rejects.toMatchObject({
			code: "ERR_CANCELED",
			message: "canceled by test",
		});
	});

	it("keeps backend envelope errors as backend error codes", async () => {
		api.client.defaults.adapter = () =>
			Promise.resolve({
				config: {},
				data: {
					code: "auth.token_invalid",
					msg: "token invalid",
				},
				headers: {},
				status: 401,
				statusText: "Unauthorized",
			});

		await expect(api.get("/auth/me")).rejects.toMatchObject({
			code: "auth.token_invalid",
			message: "token invalid",
		});
	});
});
