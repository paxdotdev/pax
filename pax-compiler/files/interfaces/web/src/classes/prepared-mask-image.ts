import type { NativeMaskEntry } from './messages/native-mask-update-patch';
import { prepareNativeMaskRaster } from './native-mask-raster';
import type { MaskPreparation, MaskRasterRequest, PreparedMask } from './native-mask-raster';

// One displayed bitmap, one preparation job, and the latest requested geometry.
// Ready coverage is not discarded until its replacement can be presented.
export class PreparedMaskImage {
    private desired?: MaskRasterRequest;
    private applied?: MaskRasterRequest;
    private failedKey?: string;
    private retained?: MaskPreparation;
    private pending?: MaskPreparation;
    private disposed = false;

    constructor(private element: HTMLElement, private createPreparation = prepareNativeMaskRaster) {}

    update(entries: NativeMaskEntry[], width: number, height: number, scale: number) {
        if (this.disposed) return;
        // Patch objects are pooled; an asynchronous job owns its snapshot.
        const snapshot = entries.map(entry => ({...entry, clips: [...entry.clips]}));
        this.desired = {entries: snapshot, width, height, scale,
            key: JSON.stringify([width, height, scale, snapshot])};
        this.prepare();
    }

    setPixelRatio(scale: number) {
        const request = this.desired;
        if (request && request.scale !== scale) this.update(request.entries, request.width, request.height, scale);
    }

    dispose() {
        this.disposed = true;
        this.desired = this.applied = undefined;
        this.failedKey = undefined;
        this.pending?.dispose();
        this.retained?.dispose();
        this.pending = this.retained = undefined;
    }

    private prepare() {
        const request = this.desired;
        if (this.disposed || this.pending || !request
            || request.key === this.applied?.key || request.key === this.failedKey) return;
        const job = this.createPreparation(request);
        this.pending = job;
        void this.present(job, request);
    }

    private async present(job: MaskPreparation, request: MaskRasterRequest) {
        try {
            const ready = await job.ready;
            if (this.disposed) return;
            // A reversal can return to already-applied geometry while another
            // job is pending. Do not replace it with that obsolete result.
            if (this.desired?.key !== this.applied?.key) {
                this.apply(ready, request);
                this.applied = request;
                this.retained?.dispose();
                this.retained = job;
            }
            this.failedKey = undefined;
        } catch {
            if (!this.disposed) this.failedKey = request.key;
        } finally {
            if (this.retained !== job) job.dispose();
            if (this.pending === job) this.pending = undefined;
            // Publish completed work before preparing the latest request;
            // continuous animation must not starve presentation.
            this.prepare();
        }
    }

    private apply(ready: PreparedMask, request: MaskRasterRequest) {
        const style = this.element.style;
        const bounds = ready.bounds;
        const cropped = ready.url && bounds;
        style.webkitMaskImage = 'linear-gradient(white, white)' + (cropped ? `, url("${ready.url}")` : '');
        style.webkitMaskRepeat = 'no-repeat';
        style.webkitMaskPosition = '0px 0px' + (cropped ? `, ${bounds.left}px ${bounds.top}px` : '');
        style.webkitMaskSize = `${request.width}px ${request.height}px`
            + (cropped ? `, ${bounds.right - bounds.left}px ${bounds.bottom - bounds.top}px` : '');
        // White is the source, cropped occluder alpha is the destination. This
        // preserves coverage outside the crop without rasterizing that area.
        (style as any).webkitMaskComposite = 'source-out';
        style.maskComposite = 'subtract';
        style.maskMode = 'alpha';
    }
}
