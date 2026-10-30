/** One owner, one module installation. All callers share an attempt and its
 * cancellation. Retrying a failed download/compile is safe; an install failure
 * poisons this owner because instantiation may already have changed memory. */
export function createSplitLoader({url, getImports, fetcher = (...args) => fetch(...args), decodedSize = null}) {
  const previewMs = Math.max(0, Math.min(60000, Number(new URLSearchParams(globalThis.location?.search ?? '').get('pax_loader_ms')) || 0));
  let pending, installed, poisoned, controller, observers = new Set(), latest;
  const emit = value => {
    latest = Object.freeze(value);
    // UI instrumentation must not change the result of a memory installation.
    for (const observer of observers) {
      try { observer(latest); } catch (error) { console.error('Loading observer failed', error); }
    }
  };
  function load() {
    if (installed) return Promise.resolve(installed);
    if (poisoned) return Promise.reject(poisoned);
    if (pending) return pending;
    controller = new AbortController();
    const signal = controller.signal;
    const attempt = Promise.resolve().then(async () => {
      let phase = 'downloading', received = 0, reader;
      const started = performance.now();
      try {
        emit({phase, received, total: decodedSize, started});
        signal.throwIfAborted();
        const response = await fetcher(url, {signal});
        if (!response.ok) throw new Error(`HTTP ${response.status} loading ${url}`);
        if (!response.body) throw new Error('The module response has no body');
        // Response streams expose decoded bytes. Content-Length is usable only
        // without content encoding; a build manifest can provide decoded size.
        const length = response.headers.get('Content-Length');
        const encoded = response.headers.get('Content-Encoding');
        const headerSize = !encoded && length !== null && /^\d+$/.test(length) ? Number(length) : null;
        const total = decodedSize ?? headerSize;
        reader = response.body.getReader();
        let previewRemainder;
        const body = new ReadableStream({
          async pull(target) {
            try {
              signal.throwIfAborted();
              let next = previewRemainder ? {done:false,value:previewRemainder} : await reader.read();
              previewRemainder = undefined;
              if (previewMs && total > 0 && !next.done) {
                const limit = Math.max(1, Math.ceil(total / 60));
                if (next.value.byteLength > limit) {
                  previewRemainder = next.value.subarray(limit);
                  next = {done:false, value:next.value.subarray(0, limit)};
                }
              }
              signal.throwIfAborted();
              if (next.done) { target.close(); return; }
              if (previewMs && total > 0) {
                await new Promise((resolve, reject) => {
                  const abort = () => { clearTimeout(timer); reject(signal.reason); };
                  const timer = setTimeout(() => { signal.removeEventListener('abort', abort); resolve(); }, previewMs * next.value.byteLength / total);
                  signal.addEventListener('abort', abort, {once:true});
                });
                signal.throwIfAborted();
              }
              received += next.value.byteLength;
              emit({phase:'downloading', received, total, started});
              target.enqueue(next.value);
            } catch (error) { target.error(error); }
          },
          cancel(reason) { return reader.cancel(reason); },
        });
        // Keep streaming compilation, but separate it from installation so an
        // aborted compile can never install into the running bootstrap memory.
        const module = await WebAssembly.compileStreaming(new Response(body, {headers:{'Content-Type':'application/wasm'}}));
        phase = 'compiled';
        emit({phase, received, total, started, elapsedMs:performance.now()-started});
        signal.throwIfAborted();
        if (decodedSize !== null && received !== decodedSize) throw new Error(`Module size mismatch: ${received} != ${decodedSize}`);
        const imports = getImports();
        signal.throwIfAborted();
        phase = 'installing';
        emit({phase, received, total, started});
        signal.throwIfAborted();
        const installStart = performance.now();
        // JS runs to completion here. Cancellation cannot race memory/table writes.
        try { installed = new WebAssembly.Instance(module, imports); }
        catch (error) { poisoned = new Error('Module installation failed; recreate the runtime before retrying', {cause:error}); throw poisoned; }
        emit({phase:'ready', received, total, started, installMs:performance.now()-installStart, elapsedMs:performance.now()-started});
        return installed;
      } catch (error) {
        // Safari may surface an aborted streaming compile as TypeError. The
        // owner's signal is authoritative; preserve other compile failures.
        const reason = signal.aborted ? signal.reason : error;
        // Also stop the underlying network if compilation rejects early.
        controller.abort();
        if (reader) await reader.cancel(error).catch(() => {});
        emit({phase:reason?.name === 'AbortError' ? 'cancelled' : 'error', failedPhase:phase,
          error:String(reason), retryable:!poisoned, received, started});
        throw reason;
      }
    });
    pending = attempt;
    // Register reset before exposing the promise, so a caller can retry directly
    // from its rejection handler. Avoid an unobserved promise from finally().
    attempt.then(() => {pending=undefined;controller=undefined;}, () => {pending=undefined;controller=undefined;});
    return attempt;
  }
  return {
    load,
    cancel() { if (!installed) controller?.abort(); },
    observe(callback) { observers.add(callback); if(latest) callback(latest); return () => observers.delete(callback); },
    get state() { return latest; },
    // Rust's fallible wrapper owns callback data. Settle each invocation once,
    // including deduplicated calls; error detail stays available in state.
    callback(getTable) {
      return (index, data) => {
        load().then(() => getTable().get(index)(data, true), () => getTable().get(index)(data, false));
      };
    },
  };
}
