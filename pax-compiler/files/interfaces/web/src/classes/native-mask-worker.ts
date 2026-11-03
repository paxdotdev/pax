import {nativeMaskRasterBounds, paintNativeMaskCoverage} from './native-mask-raster';
import type {MaskRasterRequest} from './native-mask-raster';

// The host dispatches one job at a time, bounding temporary raster allocations
// even when many native surfaces change together. No DOM or CSS ownership lives
// here; a completed Blob still has to decode before the host publishes it.
self.onmessage = async (event: MessageEvent<{id: number; request: MaskRasterRequest}>) => {
    const {id, request} = event.data;
    let canvas: OffscreenCanvas | undefined;
    try {
        if (typeof OffscreenCanvas === 'undefined' || typeof Path2D === 'undefined') {
            self.postMessage({id, unavailable: true, error: 'Native mask worker canvas unavailable'});
            return;
        }
        const bounds = nativeMaskRasterBounds(request);
        if (!bounds) {
            self.postMessage({id, result: {}});
            return;
        }
        canvas = new OffscreenCanvas(
            Math.round((bounds.right - bounds.left) * request.scale),
            Math.round((bounds.bottom - bounds.top) * request.scale));
        const ctx = canvas.getContext('2d');
        if (!ctx || typeof canvas.convertToBlob !== 'function') {
            self.postMessage({id, unavailable: true, error: 'Native mask worker context unavailable'});
            return;
        }
        paintNativeMaskCoverage(ctx, request, bounds);
        const blob = await canvas.convertToBlob({type: 'image/png'});
        self.postMessage({id, result: {blob, bounds}});
    } catch (error) {
        self.postMessage({id, error: String(error)});
    } finally {
        if (canvas) canvas.width = canvas.height = 0;
    }
};
