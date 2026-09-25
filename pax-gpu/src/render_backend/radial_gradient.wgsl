// Shared by color and alpha paints. The outer circle is a unit circle; the
// affine basis maps it to the surface. -1 denotes an unpainted point, including
// singular transforms and the region outside an external focal point's cone.
fn radial_coordinate(delta: vec2<f32>, axis: vec2<f32>, off: vec2<f32>, focal: vec2<f32>) -> f32 {
    // Rescaling the basis keeps the determinant independent of absolute radius.
    let scale = max(max(abs(axis.x), abs(axis.y)), max(abs(off.x), abs(off.y)));
    if scale == 0.0 { return -1.0; }
    let a_axis = axis / scale;
    let b_axis = off / scale;
    let d = delta / scale;
    let determinant = a_axis.x * b_axis.y - a_axis.y * b_axis.x;
    if determinant == 0.0 { return -1.0; }
    let uv = vec2<f32>(d.x * b_axis.y - d.y * b_axis.x,
        a_axis.x * d.y - a_axis.y * d.x) / determinant;
    if all(focal == vec2<f32>(0.0)) { return length(uv); }

    // |(uv - focal) + t*focal|^2 = t^2 describes a circle growing
    // from the focus (radius 0) to the outer circle (radius 1).
    let p = uv - focal;
    let a = dot(focal, focal) - 1.0;
    let b = 2.0 * dot(p, focal);
    let c = dot(p, p);
    // The exact focus has a defined limiting color only inside the outer
    // circle. Canvas leaves this point unpainted for boundary/external foci.
    if c == 0.0 { return select(-1.0, 0.0, a < 0.0); }
    if a == 0.0 {
        if b == 0.0 { return -1.0; }
        let t = -c / b;
        return select(-1.0, t, t >= 0.0);
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 { return -1.0; }
    // Avoid subtracting nearly equal values near the focus or circle boundary.
    let q = -0.5 * (b + select(-sqrt(discriminant), sqrt(discriminant), b >= 0.0));
    if q == 0.0 { return 0.0; }
    let t = max(q / a, c / q);
    return select(-1.0, t, t >= 0.0);
}
