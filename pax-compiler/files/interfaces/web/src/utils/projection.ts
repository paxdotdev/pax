/** Flat-surface presentation. Wire values are six affine or sixteen homogeneous coefficients. */
export type Homography = [number, number, number, number, number, number, number, number, number];

export function planeMapping(matrix: readonly number[]): Homography {
    if (matrix.length === 16) {
        return [matrix[0], matrix[1], matrix[3], matrix[4], matrix[5], matrix[7], matrix[12], matrix[13], matrix[15]];
    }
    return [matrix[0], matrix[1], 0, matrix[2], matrix[3], 0, matrix[4], matrix[5], 1];
}

export function projectPoint(matrix: readonly number[], point: readonly number[]): [number, number] | undefined {
    const h = planeMapping(matrix);
    const w = h[2] * point[0] + h[5] * point[1] + h[8];
    if (!(w > 0) || !Number.isFinite(w)) return undefined;
    const x = (h[0] * point[0] + h[3] * point[1] + h[6]) / w;
    const y = (h[1] * point[0] + h[4] * point[1] + h[7]) / w;
    return Number.isFinite(x) && Number.isFinite(y) ? [x, y] : undefined;
}

export function planeMatrix3d(h: Homography): number[] {
    return [h[0], h[1], 0, h[2], h[3], h[4], 0, h[5], 0, 0, 1, 0, h[6], h[7], 0, h[8]];
}

/** Invert the visible plane, preserving its positive-w chart and rejecting collapsed surfaces. */
export function inversePlane(matrix: readonly number[]): number[] | undefined {
    const input = planeMapping(matrix);
    const scale = Math.max(...input.map(Math.abs));
    if (!(scale > 0) || !Number.isFinite(scale)) return undefined;
    const m = input.map(v => v / scale);
    const c = [
        m[4]*m[8]-m[7]*m[5], m[7]*m[2]-m[1]*m[8], m[1]*m[5]-m[4]*m[2],
        m[6]*m[5]-m[3]*m[8], m[0]*m[8]-m[6]*m[2], m[3]*m[2]-m[0]*m[5],
        m[3]*m[7]-m[6]*m[4], m[6]*m[1]-m[0]*m[7], m[0]*m[4]-m[3]*m[1],
    ];
    const det = m[0]*c[0] + m[3]*c[1] + m[6]*c[2];
    if (det === 0 || !Number.isFinite(det)) return undefined;
    const inverse = c.map(v => v / det / scale) as Homography;
    return inverse.every(Number.isFinite) ? planeMatrix3d(inverse) : undefined;
}
