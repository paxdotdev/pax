import { maskOpacity, nativeMaskPathBounds } from './native-mask-raster';
import type { MaskBounds, MaskRasterRequest } from './native-mask-raster';

// Recognize geometry, not its bounding box. In particular, rounded corners,
// skewed rectangles, holes, and unsupported SVG syntax must stay on the raster path.
export function nativeMaskRectangle(path: string): MaskBounds | undefined {
    const token = /[MLCQZ]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g;
    if (!/^[\s,]*$/.test(path.replace(token, ''))) return;
    const tokens = path.match(token) ?? [];
    const vertices: number[][] = [];
    let index = 0, closed = false;
    while (index < tokens.length) {
        const command = tokens[index++];
        if (command === 'Z') {
            closed = index === tokens.length;
            break;
        }
        const count = command === 'M' || command === 'L' ? 2 : command === 'Q' ? 4 : command === 'C' ? 6 : 0;
        if (!count || index + count > tokens.length) return;
        const points = tokens.slice(index, index + count).map(Number);
        if (!points.every(Number.isFinite)) return;
        index += count;
        const current = vertices[vertices.length - 1];
        if (command === 'M') {
            if (current) return;
            vertices.push(points);
        } else if (!current) {
            return;
        } else if (command === 'L') {
            if (points[0] !== current[0] || points[1] !== current[1]) vertices.push(points);
        } else {
            // Pax emits zero-radius corners as stationary cubic segments.
            // Even collinear but nonstationary curves are deliberately not inferred.
            if (!points.every((value, n) => value === current[n % 2])) return;
        }
    }
    if (!closed || vertices.length < 4) return;
    const first = vertices[0], last = vertices[vertices.length - 1];
    if (last[0] === first[0] && last[1] === first[1]) vertices.pop();
    if (vertices.length !== 4) return;
    const xs = vertices.map(p => p[0]), ys = vertices.map(p => p[1]);
    const bounds = {left: Math.min(...xs), top: Math.min(...ys), right: Math.max(...xs), bottom: Math.max(...ys)};
    if (bounds.left === bounds.right || bounds.top === bounds.bottom) return;
    if (new Set(vertices.map(p => `${p[0]},${p[1]}`)).size !== 4) return;
    for (let n = 0; n < 4; n++) {
        const p = vertices[n], next = vertices[(n + 1) % 4];
        if ((p[0] !== bounds.left && p[0] !== bounds.right)
            || (p[1] !== bounds.top && p[1] !== bounds.bottom)
            || (p[0] !== next[0] && p[1] !== next[1])) return;
    }
    return bounds;
}

function intersect(a: MaskBounds, b: MaskBounds): MaskBounds {
    return {left: Math.max(a.left, b.left), top: Math.max(a.top, b.top),
        right: Math.min(a.right, b.right), bottom: Math.min(a.bottom, b.bottom)};
}

// One opaque rectangle may subsume other occluders (e.g. a panel's lettering).
// Interior contributors cannot change its coverage. Leave an AA-sized margin
// so overlapping edges cannot strengthen fractional coverage at the boundary.
export function rectangularNativeMaskCoverage(request: MaskRasterRequest): MaskBounds | undefined {
    if (![request.width, request.height, request.scale].every(v => Number.isFinite(v) && v > 0)) return;
    const surface = {left: 0, top: 0, right: request.width, bottom: request.height};
    const candidates = request.entries.map(entry =>
        maskOpacity(entry.opacity) === 1 && entry.clips.length === 0 ? nativeMaskRectangle(entry.path) : undefined);
    if (!candidates.some(Boolean)) return;
    const bounds = request.entries.map(entry => {
        if (maskOpacity(entry.opacity) === 0) return null;
        let bound = nativeMaskPathBounds(entry.path);
        if (!bound) return undefined;
        for (const clip of entry.clips) {
            const clipBounds = nativeMaskPathBounds(clip);
            if (!clipBounds) return undefined;
            bound = intersect(bound, clipBounds);
        }
        bound = intersect(bound, surface);
        return bound.right > bound.left && bound.bottom > bound.top ? bound : null;
    });
    const margin = 1 / request.scale;
    for (let n = 0; n < candidates.length; n++) {
        const candidate = candidates[n];
        if (!candidate) continue;
        if (!bounds.every((bound, i) => i === n || bound === null || (bound !== undefined
            && bound.left >= candidate.left + margin && bound.right <= candidate.right - margin
            && bound.top >= candidate.top + margin && bound.bottom <= candidate.bottom - margin))) continue;
        const clipped = intersect(candidate, surface);
        if (clipped.right > clipped.left && clipped.bottom > clipped.top) return clipped;
    }
}
