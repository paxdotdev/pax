// The aperture is the foreground-colored coverage of the current rendered
// subtree, including masks and partial reveals; luminance is not a geometry test.
fn aperture(source: vec4<f32>) -> f32 {
    let axis = parameters.foreground.rgb - parameters.background.rgb;
    let signal = source.rgb - parameters.background.rgb * source.a;
    return clamp(dot(signal, axis) / max(dot(axis, axis), 0.0001), 0.0, source.a);
}

// Source alpha distinguishes painted dark glass from transparent exterior.
fn glass_coverage(source: vec4<f32>) -> f32 {
    return max(source.a - aperture(source), 0.0);
}

