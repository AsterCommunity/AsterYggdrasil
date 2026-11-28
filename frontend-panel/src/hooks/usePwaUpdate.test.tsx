import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePwaUpdate } from "@/hooks/usePwaUpdate";

type RegisterOptions = {
	onRegistered?: (registration: unknown) => void;
	onRegisterError?: (error: unknown) => void;
};

const mockState = vi.hoisted(() => ({
	needRefresh: false,
	offlineReady: false,
	updateServiceWorker: vi.fn(),
	toastInfo: vi.fn(),
	translate: vi.fn((key: string) => `translated:${key}`),
	registerOptions: null as RegisterOptions | null,
	registration: {
		scope: "/",
		active: { scriptURL: "/sw.js" },
		waiting: { scriptURL: "/sw-waiting.js" },
		installing: null,
		update: vi.fn(),
	},
}));

vi.mock("react-i18next", () => ({
	useTranslation: () => ({
		t: mockState.translate,
	}),
}));

vi.mock("sonner", () => ({
	toast: {
		info: mockState.toastInfo,
	},
}));

vi.mock("virtual:pwa-register/react", () => ({
	useRegisterSW: (options: RegisterOptions) => {
		mockState.registerOptions = options;
		return {
			needRefresh: [mockState.needRefresh, vi.fn()],
			offlineReady: [mockState.offlineReady, vi.fn()],
			updateServiceWorker: mockState.updateServiceWorker,
		};
	},
}));

describe("usePwaUpdate", () => {
	beforeEach(() => {
		mockState.needRefresh = false;
		mockState.offlineReady = false;
		mockState.updateServiceWorker.mockReset();
		mockState.toastInfo.mockReset();
		mockState.translate.mockClear();
		mockState.registerOptions = null;
		mockState.registration.update.mockReset();
		mockState.registration.update.mockResolvedValue(mockState.registration);
	});

	it("does not show an update toast when no refresh is pending", () => {
		renderHook(() => usePwaUpdate());

		expect(mockState.toastInfo).not.toHaveBeenCalled();
	});

	it("does not apply an update without user confirmation", async () => {
		mockState.needRefresh = true;

		renderHook(() => usePwaUpdate());

		await waitFor(() => {
			expect(mockState.toastInfo).toHaveBeenCalled();
		});

		// Prompt semantics: the toast is the only path that applies the
		// update. Reaching the toast must never auto-trigger the takeover;
		// only the toast action below calls updateServiceWorker(true).
		expect(mockState.updateServiceWorker).not.toHaveBeenCalled();
	});

	it("shows an update toast and refreshes the service worker from the action", async () => {
		mockState.needRefresh = true;

		renderHook(() => usePwaUpdate());

		await waitFor(() => {
			expect(mockState.toastInfo).toHaveBeenCalledWith(
				"translated:pwa.updateAvailable",
				expect.objectContaining({
					action: expect.objectContaining({
						label: "translated:pwa.refresh",
					}),
					duration: Number.POSITIVE_INFINITY,
				}),
			);
		});

		const toastOptions = mockState.toastInfo.mock.calls[0]?.[1] as {
			action: { onClick: () => void };
		};
		toastOptions.action.onClick();

		expect(mockState.updateServiceWorker).toHaveBeenCalledWith(true);
	});

	it("checks for updates after registration and starts an hourly update poll", async () => {
		const setIntervalSpy = vi.spyOn(globalThis, "setInterval");

		renderHook(() => usePwaUpdate());

		act(() => {
			mockState.registerOptions?.onRegistered?.(mockState.registration);
		});

		await waitFor(() => {
			expect(mockState.registration.update).toHaveBeenCalledTimes(1);
		});

		expect(setIntervalSpy).toHaveBeenCalledWith(
			expect.any(Function),
			3_600_000,
		);

		const intervalCallback = setIntervalSpy.mock.calls[0]?.[0] as () => void;
		intervalCallback();

		expect(mockState.registration.update).toHaveBeenCalledTimes(2);
		setIntervalSpy.mockRestore();
	});

	it("checks for updates when the page becomes visible again", async () => {
		const visibilitySpy = vi
			.spyOn(document, "visibilityState", "get")
			.mockReturnValue("visible");

		renderHook(() => usePwaUpdate());

		act(() => {
			mockState.registerOptions?.onRegistered?.(mockState.registration);
		});

		await waitFor(() => {
			expect(mockState.registration.update).toHaveBeenCalledTimes(1);
		});

		act(() => {
			document.dispatchEvent(new Event("visibilitychange"));
		});

		expect(mockState.registration.update).toHaveBeenCalledTimes(2);
		visibilitySpy.mockRestore();
	});
});
