import {encodeNativeMaskOnMainThread, prepareNativeMaskRaster} from './native-mask-raster';
import type {EncodedMask, MaskEncoding, MaskRasterRequest} from './native-mask-raster';

// Bundled into the interface so nested routes and custom cartridge URLs do not
// need an additional worker asset path. CSP-restricted hosts use the fallback.
declare const __PAX_NATIVE_MASK_WORKER_SOURCE__: string;
function createWorker(): Worker {
    if (typeof Worker === 'undefined' || typeof OffscreenCanvas === 'undefined') {
        throw new Error('Native mask workers unavailable');
    }
    const url = URL.createObjectURL(new Blob([__PAX_NATIVE_MASK_WORKER_SOURCE__], {type: 'text/javascript'}));
    try {return new Worker(url, {name: 'pax-native-masks'});}
    finally {URL.revokeObjectURL(url);}
}

type Job = {request: MaskRasterRequest; resolve(value: EncodedMask): void; reject(error: Error): void};

// One worker per mounted host, shared by its native mask owners. Owners already
// coalesce to one pending request; this queue never speculates or caches pixels.
export class NativeMaskRasterizer {
    private worker?: Worker;
    private disabled = false;
    private disposed = false;
    private nextId = 0;
    private active?: number;
    private jobs = new Map<number, Job>();

    constructor(private workerFactory = createWorker, private fallback = encodeNativeMaskOnMainThread) {}

    prepare = (request: MaskRasterRequest) => prepareNativeMaskRaster(request, request => this.encode(request));

    encode(request: MaskRasterRequest): MaskEncoding {
        let cancelled = false;
        let fallback: MaskEncoding | undefined;
        const id = ++this.nextId;
        const work = new Promise<EncodedMask>((resolve, reject) => {
            if (this.disposed || this.disabled) {
                reject(new Error('Native mask worker unavailable'));
                return;
            }
            this.jobs.set(id, {request, resolve, reject});
            this.pump();
        });
        const ready = work.catch(error => {
            if (cancelled || this.disposed || !this.disabled) throw error;
            fallback = this.fallback(request);
            return fallback.ready;
        });
        return {ready, dispose: () => {
            cancelled = true;
            fallback?.dispose();
            const job = this.jobs.get(id);
            this.jobs.delete(id);
            job?.reject(new Error('Native mask preparation cancelled'));
            // An active encode cannot be interrupted without killing unrelated
            // work. Discard its reply, then dispatch the next live owner.
        }};
    }

    dispose() {
        this.disposed = true;
        this.fail(new Error('Native mask rasterizer disposed'));
    }

    private pump() {
        if (this.active !== undefined || !this.jobs.size || this.disposed || this.disabled) return;
        try {
            if (!this.worker) {
                this.worker = this.workerFactory();
                this.worker.onmessage = event => {
                    const {id, result, error, unavailable} = event.data;
                    if (id !== this.active) return;
                    if (unavailable) {this.fail(new Error(error)); return;}
                    const job = this.jobs.get(id);
                    this.jobs.delete(id);
                    this.active = undefined;
                    // A malformed path or failed encode belongs to this owner,
                    // not every mask in the host. Keep its previous ready mask.
                    if (error) job?.reject(new Error(error));
                    else job?.resolve(result);
                    this.pump();
                };
                this.worker.onerror = event => {
                    event.preventDefault();
                    this.fail(new Error('Native mask worker failed'));
                };
                this.worker.onmessageerror = () => this.fail(new Error('Native mask worker message failed'));
            }
            const [id, job] = this.jobs.entries().next().value!;
            this.active = id;
            this.worker.postMessage({id, request: job.request});
        } catch (error) {
            this.fail(error instanceof Error ? error : new Error(String(error)));
        }
    }

    private fail(error: Error) {
        this.disabled = true;
        this.worker?.terminate();
        this.worker = undefined;
        this.active = undefined;
        const jobs = [...this.jobs.values()];
        this.jobs.clear();
        jobs.forEach(job => job.reject(error));
    }
}
