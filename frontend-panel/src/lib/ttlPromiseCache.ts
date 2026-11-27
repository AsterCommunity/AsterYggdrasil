export type TtlPromiseCacheEntry<Value> = {
	expiresAt: number;
	promise: Promise<Value>;
};

/**
 * Small TTL cache for in-flight/fulfilled promises with in-flight
 * deduplication: concurrent `run` calls for the same key share one promise,
 * and a failed promise evicts itself so the next call retries. Entries live
 * for `ttlMs` unless refreshed with `force`.
 */
export class TtlPromiseCache<Value> {
	private readonly ttlMs: number;
	private readonly entries = new Map<string, TtlPromiseCacheEntry<Value>>();

	constructor(ttlMs: number) {
		this.ttlMs = ttlMs;
	}

	has(key: string): boolean {
		const entry = this.entries.get(key);
		return entry !== undefined && entry.expiresAt > Date.now();
	}

	run(key: string, promiseFactory: () => Promise<Value>, force = false) {
		if (!force) {
			const cached = this.entries.get(key);
			if (cached && cached.expiresAt > Date.now()) {
				return cached.promise;
			}
		}

		const promise = promiseFactory().catch((error: unknown) => {
			if (this.entries.get(key)?.promise === promise) {
				this.entries.delete(key);
			}
			throw error;
		});
		this.entries.set(key, {
			expiresAt: Date.now() + this.ttlMs,
			promise,
		});
		return promise;
	}

	clear() {
		this.entries.clear();
	}
}
