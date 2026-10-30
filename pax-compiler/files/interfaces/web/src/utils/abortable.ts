/** Detach a caller promptly without cancelling work another owner may share. */
export function abortable<T>(pending: Promise<T>, signal?: AbortSignal): Promise<T> {
    if (!signal) return pending;
    return new Promise((resolve, reject) => {
        const cleanup = () => signal.removeEventListener('abort', abort);
        const abort = () => { cleanup(); reject(signal.reason); };
        signal.addEventListener('abort', abort, {once: true});
        // Observe completion even after cancellation so late failures are handled.
        pending.then(value => { cleanup(); resolve(value); }, error => { cleanup(); reject(error); });
        if (signal.aborted) abort();
    });
}
