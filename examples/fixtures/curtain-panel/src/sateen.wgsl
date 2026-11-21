// Each fold has its own gathered shoulder, curved centre line and tucked side.
// Carry height and both slopes together to keep billowing, shading and fiber
// orientation attached to the same cloth as the panel scales or scrolls.
fn shoulder(offset: f32, width: f32, height: f32, center_dy: f32, width_dy: f32) -> vec3<f32> {
    let q = offset / width;
    let z = height * exp(-q * q);
    let dx = -2.0 * q * z / width;
    return vec3<f32>(z, dx, -dx * (center_dy + q * width_dy));
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    let pitch = parameters.spacing;
    let x = (p.x - 0.58 * parameters.extent.x) / pitch;
    let v = clamp(p.y / parameters.extent.y, 0.0, 1.0);
    let spread = 0.82 + 0.29 * v;
    let nearest = floor(x / spread + 0.5);
    let cycle = 6.2831853 * parameters.phase;
    var field = vec3<f32>(0.0);
    for (var j = -2; j <= 2; j++) {
        let i = nearest + f32(j);
        let seed = sin(i * 2.37 + 0.38);
        let pin = i + 0.18 * seed;
        let bend = sin(i * 1.73 + 0.60);
        let bow = 0.53 * bend;
        let lag = cycle + 0.76 * seed - 2.2 * v;
        let swing = 0.17 + 0.14 * bend;
        let draft = swing * sin(lag);
        let center = pin * spread + bow * v * (2.0 - v) + v * v * draft;
        let center_dy = pin * 0.29 + 2.0 * bow * (1.0 - v)
            + 2.0 * v * draft - 2.2 * v * v * swing * cos(lag);

        let w = 0.19 + 0.04 * seed + 0.14 * v;
        let fullness = (0.31 + 0.07 * bend) * parameters.depth;
        let dx = x - center;
        field += shoulder(dx, w, fullness, center_dy, 0.14);
        field += shoulder(dx - 1.06 * w, w * 0.48, -0.33 * fullness,
                          center_dy + 1.06 * 0.14, 0.48 * 0.14);
    }
    let slope = vec2<f32>(field.y, field.z * pitch / parameters.extent.y);
    let n = normalize(vec3<f32>(-slope, 1.0));
    // A small spread across the yarn direction stretches the reflected strip.
    // Both lobes use Pax's actual light colour, attenuation, scope and view.
    let cross_yarn = normalize(vec3<f32>(1.0, 0.0, slope.x));
    let exponent = mix(72.0, 12.0, parameters.roughness);
    let traced = 0.5 * (scene_specular(p, normalize(n + 0.13 * cross_yarn), exponent)
                       + scene_specular(p, normalize(n - 0.13 * cross_yarn), exponent));
    let recess = smoothstep(-0.10, 0.29, field.x);
    let quiet = mix(0.55, 1.0, smoothstep(0.18, 0.56, p.x / parameters.extent.x));
    let energy = traced * parameters.sheen * quiet * mix(0.47, 1.0, recess);
    let sheen = (vec3<f32>(1.0) - exp(-energy)) * 0.44;
    // Pale cloth needs shallower shadows to keep dark foreground type legible.
    let tint = parameters.body_color.rgb;
    let pale = smoothstep(0.15, 0.85, dot(tint, vec3<f32>(0.2126, 0.7152, 0.0722)));
    let body = source.a * tint * mix(mix(0.40, 0.91, pale), mix(1.24, 1.02, pale), recess);
    return vec4<f32>(body + sheen * source.a, source.a);
}
