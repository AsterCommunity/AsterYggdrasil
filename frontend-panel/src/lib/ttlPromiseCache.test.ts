import { describe, expect, it, vi } from "vitest";
import { TtlPromiseCache } from "./ttlPromiseCache";

describe("TtlPromiseCache", () => {
	it("reuses the cached promise within the TTL and dedupes concurrent calls", async () => {
		vi.useFakeTimers();
		try {
			const cache = new TtlPromiseCache<string>(60_000);
			const loader = vi.fn(async () => "value");
			const key = "k";

			const first = cache.run(key, loader);
			const second = cache.run(key, loader);
			expect(second).toBe(first);
			expect(loader).toHaveBeenCalledTimes(1);

			await vi.advanceTimersByTimeAsync(59_999);
			const third = cache.run(key, loader);
			expect(third).toBe(first);
			expect(loader).toHaveBeenCalledTimes(1);

			await vi.advanceTimersByTimeAsync(1);
			await cache.run(key, loader);
			expect(loader).toHaveBeenCalledTimes(2);
		} finally {
			vi.useRealTimers();
		}
	});

	it("evicts a failed promise so the next call retries", async () => {
		const cache = new TtlPromiseCache<string>(60_000);
		const loader = vi
			.fn<() => Promise<string>>()
			.mockRejectedValueOnce(new Error("boom"))
			.mockResolvedValueOnce("recovered");

		await expect(cache.run("k", loader)).rejects.toThrow("boom");
		await expect(cache.run("k", loader)).resolves.toBe("recovered");
	});

	it("bypasses a live cache entry when force is set and keeps the fresh promise", async () => {
		const cache = new TtlPromiseCache<string>(60_000);
		const loader = vi
			.fn<() => Promise<string>>()
			.mockResolvedValueOnce("stale")
			.mockResolvedValueOnce("fresh");

		await expect(cache.run("k", loader)).resolves.toBe("stale");
		await expect(cache.run("k", loader, true)).resolves.toBe("fresh");
		await expect(cache.run("k", loader)).resolves.toBe("fresh");
	});

	it("clear() drops every entry", async () => {
		const cache = new TtlPromiseCache<string>(60_000);
		const loader = vi
			.fn<() => Promise<string>>()
			.mockResolvedValueOnce("one")
			.mockResolvedValueOnce("two");

		await cache.run("k", loader);
		cache.clear();
		expect(cache.has("k")).toBe(false);
		await expect(cache.run("k", loader)).resolves.toBe("two");
	});
});
