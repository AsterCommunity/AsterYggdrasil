import { expect } from "@playwright/test";

/**
 * Success marker of the project API envelope
 * (`{ "code": "success", "msg": "", "data": ... }`). Yggdrasil protocol
 * endpoints intentionally do not use this envelope; protocol suites keep
 * their raw bodies and assert wire shapes directly.
 */
export const E2E_API_SUCCESS_CODE = "success";

export type E2eApiResponse<T = unknown> = {
	code: unknown;
	data?: T;
	error?: unknown;
	msg?: string;
};

export function expectApiSuccess(payload: E2eApiResponse) {
	expect(payload.code, payload.msg ?? "API response code").toBe(
		E2E_API_SUCCESS_CODE,
	);
}

export interface E2eApiCallResult {
	status: number;
	body: unknown;
}

/**
 * Parses an Aster-envelope API response and asserts both the HTTP status and
 * the envelope success code before handing `data` back to the caller.
 */
export function parseApiSuccess<T>(
	status: number,
	text: string,
	expectedStatus = 200,
): T {
	expect(status, `unexpected HTTP status for ${text.slice(0, 200)}`).toBe(
		expectedStatus,
	);
	const payload = JSON.parse(text) as E2eApiResponse<T>;
	expectApiSuccess(payload);
	return payload.data as T;
}
