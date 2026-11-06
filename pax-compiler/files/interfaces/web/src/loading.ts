import {requestPreparationFrame} from './utils/shared-work-budget';
import {abortable} from './utils/abortable';
import type {WebInitOptions, MountedApp, Glue, ApplicationOptions, ApplicationSurface} from './index';

type HostFactory = () => {initApplication(options: ApplicationOptions): Promise<ApplicationSurface>};

/** One loading experience owns two application lifetimes and their publication gate. */
export async function startLoadingApplication(options: WebInitOptions, glue: Glue, createHost: HostFactory): Promise<MountedApp> {
    const container = document.createElement('div');
    container.style.cssText = 'position:relative;width:100%;height:100%;min-height:0;isolation:isolate';
    options.element.appendChild(container);
    const mainSlot = document.createElement('div');
    const loaderSlot = document.createElement('div');
    for (const slot of [mainSlot, loaderSlot]) {
        slot.style.cssText = 'position:absolute;inset:0;overflow:hidden';
        container.appendChild(slot);
    }
    loaderSlot.style.zIndex = '1';
    const split = await abortable(import(new URL('./__wasm_split.js', new URL(options.cartridgeUrl, document.baseURI)).href), options.signal)
        .catch(error => { container.remove(); throw error; });
    const adapter = split.loader;
    const prepareLoader = () => createHost().initApplication({...options, glue, element: loaderSlot,
        loader: true, holdPresentation: true, yieldMount: true, pageScroll: false, documentRouting: false});
    let loader = await prepareLoader()
        .catch(error => { container.remove(); throw error; });
    let loaderExiting = false;
    let main: ApplicationSurface | undefined;
    let mountFailure: Promise<never> | undefined;
    let closed = false;
    let generation = 0;
    let received = 0;
    let total: number | undefined;
    let attempt: AbortController | undefined;
    let request: ((action: string) => void) | undefined;
    let poll = 0;
    const update = (next: string, error?: string, retry = false, cancel = false) => {
        if (closed) return;
        loader.chassis?.loading_update(next, received, total, error, retry, cancel);
        container.dataset.paxLoading = next;
    };
    const abort = () => { attempt?.abort(); };
    options.signal?.addEventListener('abort', abort, {once: true});
    const pump = () => {
        if (closed) return;
        const action = loader.chassis?.take_loading_request();
        if (action === 'cancel') attempt?.abort();
        if (action) request?.(action);
        poll = requestAnimationFrame(pump);
    };
    pump();
    const waitFrame = async (signal = options.signal) => {
        signal?.throwIfAborted();
        const frame = new Promise<void>(resolve => requestPreparationFrame(resolve));
        await abortable(mountFailure ? Promise.race([frame, mountFailure]) : frame, signal);
        signal?.throwIfAborted();
    };
    try {
        while (!loader.publish()) { await waitFrame(); options.signal?.throwIfAborted(); }
        for (;;) {
            options.signal?.throwIfAborted();
            attempt = new AbortController();
            const current = ++generation;
            // The module belongs to the shared Wasm runtime. A cancelled view
            // releases its interest; another live application may still need it.
            const release = split.acquire ? split.acquire() : () => adapter.cancel();
            attempt.signal.addEventListener('abort', release, {once: true});
            const unobserve = adapter.observe((state: any) => {
                if (current !== generation || attempt?.signal.aborted) return;
                received = state.received ?? received;
                total = state.total ?? total;
                if (state.phase === 'downloading') update('downloading', undefined, false, true);
                else if (state.phase === 'ready' || state.phase === 'compiled' || state.phase === 'installing') update('preparing');
            });
            try {
                update('downloading', undefined, false, true);
                await new Promise<void>((resolve, reject) => {
                    const signal = attempt!.signal;
                    const stop = () => reject(signal.reason);
                    signal.addEventListener('abort', stop, {once: true});
                    adapter.load().then(() => { signal.removeEventListener('abort', stop); resolve(); },
                        (error: unknown) => { signal.removeEventListener('abort', stop); reject(error); });
                });
                attempt.signal.throwIfAborted();
                update('preparing', undefined, false, true);
                main = await createHost().initApplication({...options, glue, element: mainSlot,
                    holdPresentation: true, yieldMount: true, signal: attempt.signal});
                // First-viewport readiness can precede full mounting. A later
                // failure must interrupt reveal/exit waits just like cancellation.
                mountFailure = main.mounted.then(() => new Promise<never>(() => {}));
                void mountFailure.catch(() => {});
                attempt.signal.throwIfAborted();
                await waitFrame(attempt.signal);
                // Both policies require settled native content and submitted
                // visible surfaces. Revealing also releases deferred mounting.
                while (!main.reveal()) await waitFrame(attempt.signal);
                if (options.loaderExit !== 'first_viewport') await abortable(main.mounted, attempt.signal);
                update('ready');
                await waitFrame(attempt.signal);
                update('revealing');
                loaderExiting = true;
                loader.chassis!.begin_presentation_exit();
                do { await waitFrame(attempt.signal); }
                while (!loader.chassis!.presentation_exit_complete());
                // A custom loader need not fade every element in its @out.
                // Retire its visible slot even if the remaining mounts are pending.
                loaderSlot.hidden = true;
                main.enableInput();
                await abortable(main.mounted, attempt.signal);
                while (!main.publish()) await waitFrame(attempt.signal);
                update('active');
                loader.close();
                loaderSlot.remove();
                break;
            } catch (error) {
                mountFailure = undefined;
                main?.close();
                if (main) await main.closed;
                main = undefined;
                // A handoff can fail after exit starts. Retire the exited loader
                // lifetime and prepare a fresh one for error/retry in that case.
                if (loaderExiting && !options.signal?.aborted) {
                    loader.close();
                    await loader.closed;
                    // The replacement needs layout for native measurement; its
                    // own held-presentation region prevents an unprepared flash.
                    loaderSlot.hidden = false;
                    loader = await prepareLoader();
                    while (!loader.publish()) await waitFrame();
                    loaderExiting = false;
                }
                options.signal?.throwIfAborted();
                unobserve?.(); release();
                const cancelled = attempt.signal.aborted;
                const retryable = !(error instanceof WebAssembly.RuntimeError) && adapter.state?.retryable !== false;
                update(cancelled ? 'cancelled' : 'failed', String(error), retryable);
                // A failed attempt stays visible and actionable. The caller's
                // AbortSignal still closes the entire invocation immediately.
                await new Promise<void>((resolve, reject) => {
                    const stop = () => { request = undefined; reject(options.signal?.reason); };
                    options.signal?.addEventListener('abort', stop, {once: true});
                    request = action => {
                        if (action !== 'retry' || !retryable) return;
                        options.signal?.removeEventListener('abort', stop);
                        request = undefined;
                        resolve();
                    };
                });
            } finally { unobserve?.(); release(); }
        }
    } catch (error) {
        closed = true;
        options.signal?.removeEventListener('abort', abort);
        loader.close(); main?.close(); container.remove();
        throw error;
    } finally {
        cancelAnimationFrame(poll);
    }
    void main!.closed.then(() => options.signal?.removeEventListener('abort', abort));
    return {
        closed: main!.closed.then(() => { container.remove(); }),
        request: bytes => main!.request(bytes),
        close() { if (closed) return; closed = true; main!.close(); container.remove(); },
    };
}
