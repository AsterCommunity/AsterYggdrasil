import { afterEach, describe, expect, it, vi } from "vitest";
import {
	readJsonStorageItem,
	readStorageItem,
	removeStorageItem,
	STORAGE_KEYS,
	writeJsonStorageItem,
	writeStorageItem,
} from "@/lib/storage";

describe("storage helpers", () => {
	afterEach(() => {
		vi.restoreAllMocks();
		vi.unstubAllGlobals();
	});

	it("reads, writes, and removes local storage values", () => {
		expect(writeStorageItem("local", STORAGE_KEYS.themeMode, "dark")).toBe(
			true,
		);
		expect(readStorageItem("local", STORAGE_KEYS.themeMode)).toBe("dark");

		expect(removeStorageItem("local", STORAGE_KEYS.themeMode)).toBe(true);
		expect(readStorageItem("local", STORAGE_KEYS.themeMode)).toBeNull();
	});

	it("keeps session storage separate from local storage", () => {
		writeStorageItem("session", STORAGE_KEYS.authExpiresAt, "123");

		expect(readStorageItem("session", STORAGE_KEYS.authExpiresAt)).toBe("123");
		expect(readStorageItem("local", STORAGE_KEYS.authExpiresAt)).toBeNull();
	});

	it("round-trips JSON payloads", () => {
		const payload = {
			ownerId: "tab-1",
			lockId: "lock-1",
			expiresAt: 123,
		};

		expect(
			writeJsonStorageItem("local", STORAGE_KEYS.refreshLock, payload),
		).toBe(true);
		expect(readJsonStorageItem("local", STORAGE_KEYS.refreshLock)).toEqual(
			payload,
		);
	});

	it("returns null for missing or malformed JSON values", () => {
		expect(readJsonStorageItem("local", STORAGE_KEYS.refreshEvent)).toBeNull();

		localStorage.setItem(STORAGE_KEYS.refreshEvent, "{");

		expect(readJsonStorageItem("local", STORAGE_KEYS.refreshEvent)).toBeNull();
	});

	it("returns null or false when storage methods throw", () => {
		vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
			throw new Error("get blocked");
		});
		vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
			throw new Error("set blocked");
		});
		vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
			throw new Error("remove blocked");
		});

		expect(readStorageItem("local", STORAGE_KEYS.cachedUser)).toBeNull();
		expect(writeStorageItem("local", STORAGE_KEYS.cachedUser, "value")).toBe(
			false,
		);
		expect(removeStorageItem("local", STORAGE_KEYS.cachedUser)).toBe(false);
		expect(
			writeJsonStorageItem("local", STORAGE_KEYS.cachedUser, { id: 1 }),
		).toBe(false);
	});

	it("returns null or false when the browser storage object is unavailable", () => {
		const localStorageDescriptor = Object.getOwnPropertyDescriptor(
			window,
			"localStorage",
		);

		Object.defineProperty(window, "localStorage", {
			configurable: true,
			get() {
				throw new Error("local storage disabled");
			},
		});

		try {
			expect(readStorageItem("local", STORAGE_KEYS.themeMode)).toBeNull();
			expect(writeStorageItem("local", STORAGE_KEYS.themeMode, "dark")).toBe(
				false,
			);
			expect(removeStorageItem("local", STORAGE_KEYS.themeMode)).toBe(false);
		} finally {
			if (localStorageDescriptor) {
				Object.defineProperty(window, "localStorage", localStorageDescriptor);
			}
		}
	});

	it("returns null or false outside a browser window", () => {
		const currentWindow = window;
		vi.stubGlobal("window", undefined);

		try {
			expect(readStorageItem("local", STORAGE_KEYS.themeMode)).toBeNull();
			expect(writeStorageItem("local", STORAGE_KEYS.themeMode, "dark")).toBe(
				false,
			);
			expect(removeStorageItem("local", STORAGE_KEYS.themeMode)).toBe(false);
		} finally {
			vi.stubGlobal("window", currentWindow);
		}
	});
});
