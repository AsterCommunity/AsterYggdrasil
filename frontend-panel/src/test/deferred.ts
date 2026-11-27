/**
 * Manually controlled promise for asserting pending-then-settled behavior
 * (stale responses, loading states) without timers.
 */
export function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (reason: unknown) => void;
	const promise = new Promise<T>((yes, no) => {
		resolve = yes;
		reject = no;
	});
	return { promise, resolve, reject };
}
