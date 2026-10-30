// Project premultiplied source color onto its authored two-color palette.
// This selects the same geometry when the palette is reversed, including AA.
fn aperture(source: vec4<f32>) -> f32 {
    let axis = parameters.foreground.rgb - parameters.background.rgb;
    let signal = source.rgb - parameters.background.rgb * source.a;
    return clamp(dot(signal, axis) / max(dot(axis, axis), 0.0001), 0.0, source.a);
}

// A periodic, spatially coherent field: no per-frame reseeding or history.
fn corona_field(p: vec2<f32>) -> f32 {
    let phase = parameters.phase * 6.283185307;
    let q = p / 76.0;
    let warp = vec2<f32>(
        sin(q.y * 1.1 + sin(q.x * 0.8 + phase)) + 0.4 * sin(q.y * 2.3 - phase),
        cos(q.x * 0.9 - phase + 0.7 * sin(q.y * 1.2 + phase))
    );
    let w = q + warp * 0.85;
    let a = 0.5 + 0.5 * sin(w.x * 4.3 + w.y * 2.1 - phase * 2.0);
    let b = 0.5 + 0.5 * sin(w.x * 1.9 - w.y * 5.2 + phase);
    let broad = 0.5 + 0.5 * sin(q.x * 1.4 + q.y * 0.7 + phase);
    return clamp(0.12 + 0.64 * pow(a, 5.0) + 0.24 * pow(b, 9.0) + 0.14 * broad, 0.0, 1.0);
}
