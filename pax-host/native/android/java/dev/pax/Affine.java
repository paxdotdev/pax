package dev.pax;

/** Decomposes a 2D affine map into two rotations and a signed scale.
 * Android View properties then carry the same transform for rendering, input,
 * and accessibility; an animation matrix only changes rendering.
 */
final class Affine {
    private Affine() {}

    // A = R(outer) * diag(scaleX, scaleY) * R(inner).
    static double[] decompose(double a, double b, double c, double d) {
        double first = Math.hypot(a,b), second = Math.hypot(c,d);
        if(first > 0 && Math.abs(a*c+b*d) <= 1e-7*first*second) {
            return new double[] {Math.toDegrees(Math.atan2(b,a)), first, (a*d-b*c)/first, 0};
        }
        double phi = 0.5 * Math.atan2(2 * (a*c + b*d), a*a + b*b - c*c - d*d);
        double x = a*Math.cos(phi) + c*Math.sin(phi);
        double y = b*Math.cos(phi) + d*Math.sin(phi);
        double sx = Math.hypot(x, y);
        double sy = sx == 0 ? 0 : (a*d - b*c) / sx;
        return new double[] {Math.toDegrees(Math.atan2(y, x)), sx, sy, -Math.toDegrees(phi)};
    }
}
