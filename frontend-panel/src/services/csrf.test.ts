import { beforeEach, describe, expect, it } from "vitest";
import {
	DEFAULT_CSRF_COOKIE_NAME,
	DEFAULT_CSRF_HEADER_NAME,
	getCsrfCookieName,
	getCsrfHeaderName,
	getCsrfToken,
	readCookie,
} from "@/services/csrf";

function setTestCookie(cookie: string) {
	// biome-ignore lint/suspicious/noDocumentCookie: jsdom tests need direct cookie mutation.
	document.cookie = cookie;
}

function setMeta(name: string, content: string) {
	const element = document.createElement("meta");
	element.name = name;
	element.content = content;
	document.head.append(element);
}

describe("csrf helpers", () => {
	beforeEach(() => {
		document.head.innerHTML = "";
		window.__ASTER_YGGDRASIL_CSRF__ = undefined;
		setTestCookie("plain=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/");
		setTestCookie(
			`${DEFAULT_CSRF_COOKIE_NAME}=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/`,
		);
		setTestCookie(
			"custom_csrf=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/",
		);
		setTestCookie(
			"space%20name=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/",
		);
	});

	it("returns the decoded cookie value for an exact name match", () => {
		setTestCookie("plain=value%20with%20spaces; path=/");

		expect(readCookie("plain")).toBe("value with spaces");
	});

	it("matches cookie names after URL encoding", () => {
		setTestCookie("space%20name=encoded-value; path=/");

		expect(readCookie("space name")).toBe("encoded-value");
	});

	it("returns null when the cookie is missing", () => {
		expect(readCookie("missing")).toBeNull();
	});

	it("reads the csrf token from the default cookie name", () => {
		setTestCookie(`${DEFAULT_CSRF_COOKIE_NAME}=csrf-token-1; path=/`);

		expect(getCsrfToken()).toBe("csrf-token-1");
	});

	it("reads csrf names from injected meta tags", () => {
		setMeta("asteryggdrasil-csrf-cookie-name", "custom_csrf");
		setMeta("asteryggdrasil-csrf-header-name", "x-custom-csrf");
		setTestCookie("custom_csrf=csrf-token-2; path=/");

		expect(getCsrfCookieName()).toBe("custom_csrf");
		expect(getCsrfHeaderName()).toBe("x-custom-csrf");
		expect(getCsrfToken()).toBe("csrf-token-2");
	});

	it("falls back to the runtime csrf object when meta tags are absent", () => {
		window.__ASTER_YGGDRASIL_CSRF__ = {
			cookieName: "custom_csrf",
			headerName: "x-runtime-csrf",
		};
		setTestCookie("custom_csrf=csrf-token-3; path=/");

		expect(getCsrfCookieName()).toBe("custom_csrf");
		expect(getCsrfHeaderName()).toBe("x-runtime-csrf");
		expect(getCsrfToken()).toBe("csrf-token-3");
	});

	it("falls back to built-in names when injected values are blank", () => {
		setMeta("asteryggdrasil-csrf-cookie-name", " ");
		setMeta("asteryggdrasil-csrf-header-name", "");

		expect(getCsrfCookieName()).toBe(DEFAULT_CSRF_COOKIE_NAME);
		expect(getCsrfHeaderName()).toBe(DEFAULT_CSRF_HEADER_NAME);
	});
});
