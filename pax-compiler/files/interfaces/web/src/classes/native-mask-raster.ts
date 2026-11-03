import type { NativeMaskEntry } from './messages/native-mask-update-patch';

export type MaskBounds = {left: number; top: number; right: number; bottom: number};
export type MaskRasterRequest = {
    entries: NativeMaskEntry[]; width: number; height: number; scale: number; key: string;
};
export type PreparedMask = {url?: string; bounds?: MaskBounds};
export type MaskPreparation = {ready: Promise<PreparedMask>; dispose(): void};
export type EncodedMask = {blob?: Blob; bounds?: MaskBounds};
export type MaskEncoding = {ready: Promise<EncodedMask>; dispose(): void};

// The native-mask protocol emits absolute M/L/Q/C/Z paths. Bezier control hulls
// conservatively contain their curves. Unknown syntax must never narrow a mask.
export function nativeMaskPathBounds(path: string): MaskBounds | undefined {
    const tokens: (string | number)[] = [];
    const token = /[MLCQZ]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g;
    let end = 0;
    let match: RegExpExecArray | null;
    while ((match = token.exec(path))) {
        if (!/^[\s,]*$/.test(path.slice(end, match.index))) return;
        const value = /^[MLCQZ]$/.test(match[0]) ? match[0] : Number(match[0]);
        if (typeof value === 'number' && !Number.isFinite(value)) return;
        tokens.push(value);
        end = match.index + match[0].length;
    }
    if (!/^[\s,]*$/.test(path.slice(end))) return;
    const bounds = {left: Infinity, top: Infinity, right: -Infinity, bottom: -Infinity};
    let command = '', index = 0;
    while (index < tokens.length) {
        if (typeof tokens[index] === 'string') command = tokens[index++] as string;
        if (command === 'Z') {command = ''; continue;}
        const count = command === 'C' ? 6 : command === 'Q' ? 4 : command === 'M' || command === 'L' ? 2 : 0;
        if (!count || index + count > tokens.length) return;
        const points = tokens.slice(index, index + count);
        if (points.some(value => typeof value !== 'number')) return;
        for (let n = 0; n < count; n += 2) {
            bounds.left = Math.min(bounds.left, points[n] as number);
            bounds.top = Math.min(bounds.top, points[n + 1] as number);
            bounds.right = Math.max(bounds.right, points[n] as number);
            bounds.bottom = Math.max(bounds.bottom, points[n + 1] as number);
        }
        index += count;
        if (command === 'M') command = 'L';
    }
    return Number.isFinite(bounds.left) ? bounds : undefined;
}

function intersect(a: MaskBounds, b: MaskBounds): MaskBounds {
    return {left: Math.max(a.left, b.left), top: Math.max(a.top, b.top),
        right: Math.min(a.right, b.right), bottom: Math.min(a.bottom, b.bottom)};
}

export function maskOpacity(opacity: number | undefined): number {
    return Number.isFinite(opacity) ? Math.min(1, Math.max(0, opacity!)) : 1;
}

export function nativeMaskRasterBounds(request: MaskRasterRequest): MaskBounds | undefined {
    const surface = {left: 0, top: 0, right: request.width, bottom: request.height};
    let union: MaskBounds | undefined;
    for (const entry of request.entries) {
        if (maskOpacity(entry.opacity) === 0) continue;
        let bounds = intersect(nativeMaskPathBounds(entry.path) ?? surface, surface);
        for (const clip of entry.clips) {
            const clipBounds = nativeMaskPathBounds(clip);
            if (clipBounds) bounds = intersect(bounds, clipBounds);
        }
        if (bounds.right <= bounds.left || bounds.bottom <= bounds.top) continue;
        union = union ? {left: Math.min(union.left, bounds.left), top: Math.min(union.top, bounds.top),
            right: Math.max(union.right, bounds.right), bottom: Math.max(union.bottom, bounds.bottom)} : bounds;
    }
    if (!union) return;
    // A physical-pixel guard retains antialias coverage at conservative bounds.
    const s = request.scale;
    return {left: Math.max(0, Math.floor(union.left * s) - 1) / s,
        top: Math.max(0, Math.floor(union.top * s) - 1) / s,
        right: Math.min(Math.ceil(request.width * s), Math.ceil(union.right * s) + 1) / s,
        bottom: Math.min(Math.ceil(request.height * s), Math.ceil(union.bottom * s) + 1) / s};
}

export function paintNativeMaskCoverage(ctx: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D, request: MaskRasterRequest, bounds: MaskBounds) {
    ctx.setTransform(request.scale, 0, 0, request.scale, -bounds.left * request.scale, -bounds.top * request.scale);
    ctx.fillStyle = 'white';
    // Source-over combines occluder alpha as 1 - product(1 - opacity), matching
    // black-over-white SVG luminance coverage after subtracting this bitmap.
    for (const entry of request.entries) {
        ctx.save();
        for (const clip of entry.clips) ctx.clip(new Path2D(clip));
        ctx.globalAlpha = maskOpacity(entry.opacity);
        ctx.fill(new Path2D(entry.path));
        ctx.restore();
    }
}

// Compatibility path for hosts without worker-side 2D canvas, or whose CSP
// disallows workers. Safari's callback-based toBlob can encode synchronously.
export function encodeNativeMaskOnMainThread(request: MaskRasterRequest): MaskEncoding {
    let cancelled = false;
    let canvas: HTMLCanvasElement | undefined;
    const dispose = () => {
        cancelled = true;
        if (canvas) canvas.width = canvas.height = 0;
        canvas = undefined;
    };
    const ready = (async (): Promise<EncodedMask> => {
        const bounds = nativeMaskRasterBounds(request);
        if (!bounds) return {};
        try {
            canvas = document.createElement('canvas');
            canvas.width = Math.round((bounds.right - bounds.left) * request.scale);
            canvas.height = Math.round((bounds.bottom - bounds.top) * request.scale);
            const ctx = canvas.getContext('2d');
            if (!ctx) throw new Error('Native mask raster context unavailable');
            paintNativeMaskCoverage(ctx, request, bounds);
            const blob = await new Promise<Blob | null>(resolve => canvas!.toBlob(resolve, 'image/png'));
            if (cancelled) throw new Error('Native mask preparation cancelled');
            canvas.width = canvas.height = 0;
            canvas = undefined;
            if (!blob) throw new Error('Native mask encoding failed');
            return {blob, bounds};
        } finally {
            dispose();
        }
    })();
    return {ready, dispose};
}

export function prepareNativeMaskRaster(request: MaskRasterRequest,
    encode = encodeNativeMaskOnMainThread): MaskPreparation {
    let cancelled = false;
    let image: HTMLImageElement | undefined;
    let url: string | undefined;
    const encoding = encode(request);
    const dispose = () => {
        cancelled = true;
        encoding.dispose();
        image?.removeAttribute('src');
        image = undefined;
        if (url) URL.revokeObjectURL(url);
        url = undefined;
    };
    const ready = (async (): Promise<PreparedMask> => {
        try {
            const {blob, bounds} = await encoding.ready;
            if (cancelled) throw new Error('Native mask preparation cancelled');
            if (!blob || !bounds) return {};
            url = URL.createObjectURL(blob);
            image = new Image();
            image.src = url;
            // SVG decode() need not rasterize; this waits for a real bitmap.
            // Retain it alongside the CSS URL until replacement or teardown.
            await image.decode();
            if (cancelled) throw new Error('Native mask preparation cancelled');
            return {url, bounds};
        } catch (error) {
            dispose();
            throw error;
        }
    })();
    return {ready, dispose};
}
