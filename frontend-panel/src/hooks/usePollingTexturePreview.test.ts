import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { usePollingTexturePreview } from "./usePollingTexturePreview";

type MockResponse = {
	status: number;
	headers: Map<string, string>;
	blob: () => Promise<Blob>;
};

const objectUrls: string[] = [];
let objectUrlCounter = 0;

function jsonResponse(
	status: number,
	options: { retryAfter?: string; body?: string } = {},
): MockResponse {
	return {
		status,
		headers: new Map(
			options.retryAfter === undefined
				? []
				: [["retry-after", options.retryAfter]],
		),
		blob: async () =>
			new Blob([options.body ?? "png-bytes"], { type: "image/png" }),
	};
}

function stubFetch(
	impl: (url: string) => MockResponse | Promise<MockResponse>,
) {
	return vi
		.spyOn(globalThis, "fetch")
		.mockImplementation((input: RequestInfo | URL) =>
			Promise.resolve(impl(String(input))),
		);
}

beforeEach(() => {
	vi.useFakeTimers();
	objectUrls.length = 0;
	vi.stubGlobal("URL", {
		...URL,
		createObjectURL: vi.fn(() => {
			objectUrlCounter += 1;
			const url = `blob:test-${objectUrlCounter}`;
			objectUrls.push(url);
			return url;
		}),
		revokeObjectURL: vi.fn((url: URL | string) => {
			const index = objectUrls.indexOf(String(url));
			if (index >= 0) {
				objectUrls.splice(index, 1);
			}
		}),
	});
});

afterEach(() => {
	vi.unstubAllGlobals();
	vi.useRealTimers();
});

function currentObjectUrl(): string {
	expect(objectUrls.length).toBeGreaterThan(0);
	return objectUrls[objectUrls.length - 1] as string;
}

describe("usePollingTexturePreview", () => {
	it("reports the source texture without fetching when no preview url is given", async () => {
		const fetchSpy = stubFetch((_url) => {
			throw new Error("must not fetch when previewUrl is absent");
		});
		const { result } = renderHook(() =>
			usePollingTexturePreview(null, "/textures/skin.png"),
		);

		await act(async () => {});
		expect(result.current).toEqual({
			kind: "source",
			src: "/textures/skin.png",
		});
		expect(fetchSpy).not.toHaveBeenCalled();
	});

	it("resolves a 200 immediately to a preview blob url", async () => {
		const fetchSpy = stubFetch(() => jsonResponse(200));
		const { result } = renderHook(() =>
			usePollingTexturePreview(
				"/api/v1/texture-previews/hash/a.png",
				"/textures/skin.png",
			),
		);

		await act(async () => {});
		await act(async () => {});
		expect(result.current.kind === "preview" ? result.current.src : "").toMatch(
			/^blob:/,
		);
		expect(fetchSpy).toHaveBeenCalledTimes(1);
	});

	it("polls 202 with retry-after delays until a 200 arrives", async () => {
		let calls = 0;
		const fetchSpy = stubFetch(() => {
			calls += 1;
			if (calls <= 2) {
				return jsonResponse(202, { retryAfter: "3" });
			}
			return jsonResponse(200);
		});
		const { result } = renderHook(() =>
			usePollingTexturePreview(
				"/api/v1/texture-previews/hash/a.png",
				"/textures/skin.png",
			),
		);

		await act(async () => {});
		expect(result.current).toEqual({
			kind: "source",
			src: "/textures/skin.png",
		});
		expect(fetchSpy).toHaveBeenCalledTimes(1);

		await act(async () => {
			await vi.advanceTimersByTimeAsync(3000);
		});
		expect(fetchSpy).toHaveBeenCalledTimes(2);

		await act(async () => {
			await vi.advanceTimersByTimeAsync(3000);
		});
		await act(async () => {});
		expect(fetchSpy).toHaveBeenCalledTimes(3);
		expect(result.current.kind === "preview" ? result.current.src : "").toMatch(
			/^blob:/,
		);
	});

	it("schedules a refresh round after exhausting 202 retries instead of giving up", async () => {
		const fetchSpy = stubFetch(() => jsonResponse(202, { retryAfter: "1" }));
		const { result } = renderHook(() =>
			usePollingTexturePreview(
				"/api/v1/texture-previews/hash/a.png",
				"/textures/skin.png",
			),
		);

		// Exhaust the default round budget (5 attempts).
		await act(async () => {});
		for (const _ of [2, 3, 4, 5]) {
			await act(async () => {
				await vi.advanceTimersByTimeAsync(1000);
			});
		}
		expect(fetchSpy).toHaveBeenCalledTimes(5);
		expect(result.current).toEqual({
			kind: "source",
			src: "/textures/skin.png",
		});

		// The refresh timer schedules a new round after one more Retry-After.
		await act(async () => {
			await vi.advanceTimersByTimeAsync(1000);
		});
		await act(async () => {});
		expect(fetchSpy).toHaveBeenCalledTimes(6);
		expect(result.current).toEqual({
			kind: "source",
			src: "/textures/skin.png",
		});
	});

	it.each([
		[undefined, 2000, "missing header defaults to 2s"],
		["abc", 2000, "non-numeric header defaults to 2s"],
		["-1", 2000, "negative header defaults to 2s"],
		["5", 5000, "numeric header is honored"],
		["999", 30000, "oversized header clamps to 30s"],
	])("honors retry-after boundary %s", async (header, expectedDelay) => {
		let calls = 0;
		stubFetch(() => {
			calls += 1;
			return calls === 1
				? jsonResponse(202, { retryAfter: header })
				: jsonResponse(200);
		});
		renderHook(() =>
			usePollingTexturePreview(
				"/api/v1/texture-previews/hash/a.png",
				"/textures/skin.png",
			),
		);

		await act(async () => {});
		expect(fetch).toHaveBeenCalledTimes(1);
		await act(async () => {
			await vi.advanceTimersByTimeAsync(expectedDelay - 1);
		});
		expect(fetch).toHaveBeenCalledTimes(1);
		await act(async () => {
			await vi.advanceTimersByTimeAsync(1);
		});
		await act(async () => {});
	});

	it("falls back to the source texture on 404 without further polling", async () => {
		const fetchSpy = stubFetch(() => jsonResponse(404));
		const { result } = renderHook(() =>
			usePollingTexturePreview(
				"/api/v1/texture-previews/hash/a.png",
				"/textures/skin.png",
			),
		);

		await act(async () => {});
		expect(result.current).toEqual({
			kind: "source",
			src: "/textures/skin.png",
		});
		expect(fetchSpy).toHaveBeenCalledTimes(1);
		await act(async () => {
			await vi.advanceTimersByTimeAsync(60_000);
		});
		expect(fetchSpy).toHaveBeenCalledTimes(1);
	});

	it.each([500, 503])(
		"falls back to the source texture on http %i without further polling",
		async (status) => {
			const fetchSpy = stubFetch(() => jsonResponse(status));
			const { result } = renderHook(() =>
				usePollingTexturePreview(
					"/api/v1/texture-previews/hash/a.png",
					"/textures/skin.png",
				),
			);

			await act(async () => {});
			expect(result.current).toEqual({
				kind: "source",
				src: "/textures/skin.png",
			});
			expect(fetchSpy).toHaveBeenCalledTimes(1);
			await act(async () => {
				await vi.advanceTimersByTimeAsync(60_000);
			});
			expect(fetchSpy).toHaveBeenCalledTimes(1);
		},
	);

	it("falls back to the source texture on a network failure without retrying", async () => {
		const fetchSpy = stubFetch((_url) => {
			throw new TypeError("network down");
		});
		const { result } = renderHook(() =>
			usePollingTexturePreview(
				"/api/v1/texture-previews/hash/a.png",
				"/textures/skin.png",
			),
		);

		await act(async () => {});
		expect(result.current).toEqual({
			kind: "source",
			src: "/textures/skin.png",
		});
		expect(fetchSpy).toHaveBeenCalledTimes(1);
		await act(async () => {
			await vi.advanceTimersByTimeAsync(60_000);
		});
		expect(fetchSpy).toHaveBeenCalledTimes(1);
	});

	it("shows texture B's source during its 202 round and never texture A's preview", async () => {
		let calls = 0;
		stubFetch((url) => {
			calls += 1;
			if (url.includes("a.png")) {
				return jsonResponse(200);
			}
			// Texture B: first round still generating, then done.
			return calls <= 2
				? jsonResponse(202, { retryAfter: "1" })
				: jsonResponse(200);
		});
		const { result, rerender } = renderHook(
			({ previewUrl }: { previewUrl: string }) =>
				usePollingTexturePreview(previewUrl, "/textures/b.png"),
			{ initialProps: { previewUrl: "/preview/a.png" } },
		);

		await act(async () => {});
		await act(async () => {});
		expect(result.current).toEqual({
			kind: "preview",
			src: currentObjectUrl(),
		});

		// Switch to B: the stale preview of A must be dropped immediately,
		// both on the first 202 and before the eventual preview lands.
		await act(async () => {
			rerender({ previewUrl: "/preview/b.png" });
		});
		await act(async () => {});
		expect(result.current).toEqual({
			kind: "source",
			src: "/textures/b.png",
		});

		const previewBeforeSwitch =
			result.current.kind === "preview" ? result.current.src : "";

		await act(async () => {
			await vi.advanceTimersByTimeAsync(1000);
		});
		await act(async () => {});
		expect(result.current.kind).toBe("preview");
		// The new preview is texture B's blob, never the stale A blob.
		expect(
			result.current.kind === "preview" ? result.current.src : "",
		).not.toBe(previewBeforeSwitch);
	});

	it("revokes the previous object url and refetches when the preview url changes", async () => {
		stubFetch(() => jsonResponse(200));
		const { result, rerender } = renderHook(
			({ previewUrl }: { previewUrl: string }) =>
				usePollingTexturePreview(previewUrl, "/textures/skin.png"),
			{ initialProps: { previewUrl: "/preview/a.png" } },
		);

		await act(async () => {});
		await act(async () => {});
		expect(result.current.kind).toBe("preview");
		const firstUrl = currentObjectUrl();

		await act(async () => {
			rerender({ previewUrl: "/preview/b.png" });
		});
		await act(async () => {});
		expect(result.current.kind).toBe("preview");
		expect(objectUrls).not.toContain(firstUrl);
		expect(currentObjectUrl()).not.toBe(firstUrl);
	});
});
