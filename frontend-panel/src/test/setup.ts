import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, vi } from "vitest";
import { server } from "@/test/server";

// MSW runs globally with `onUnhandledFrame: "error"`: any request that
// escapes `vi.mock` service boundaries and has no handler fails loudly
// instead of silently leaking to the network. See src/test/README.md for
// the vi.mock (component boundary) vs MSW (network boundary) rules.

class ResizeObserverMock {
	observe() {}
	unobserve() {}
	disconnect() {}
}

class IntersectionObserverMock {
	root = null;
	rootMargin = "";
	thresholds = [];

	disconnect() {}
	observe() {}
	takeRecords() {
		return [];
	}
	unobserve() {}
}

Object.defineProperty(window, "matchMedia", {
	writable: true,
	value: vi.fn().mockImplementation((query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addEventListener: vi.fn(),
		removeEventListener: vi.fn(),
		addListener: vi.fn(),
		removeListener: vi.fn(),
		dispatchEvent: vi.fn(),
	})),
});

Object.defineProperty(navigator, "language", {
	configurable: true,
	value: "en-US",
});

Object.defineProperty(navigator, "languages", {
	configurable: true,
	value: ["en-US"],
});

Object.defineProperty(window, "ResizeObserver", {
	writable: true,
	value: ResizeObserverMock,
});

Object.defineProperty(window, "IntersectionObserver", {
	writable: true,
	value: IntersectionObserverMock,
});

Object.defineProperty(window, "scrollTo", {
	writable: true,
	value: vi.fn(),
});

Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
	writable: true,
	value: vi.fn(),
});

Object.defineProperty(Element.prototype, "getAnimations", {
	writable: true,
	value: vi.fn(() => []),
});

beforeAll(() => {
	server.listen({ onUnhandledFrame: "error" });
});

afterEach(() => {
	cleanup();
	server.resetHandlers();
	localStorage.clear();
	sessionStorage.clear();
	document.documentElement.className = "";
	document.documentElement.removeAttribute("data-theme");
	vi.useRealTimers();
});

afterAll(() => {
	server.close();
});
