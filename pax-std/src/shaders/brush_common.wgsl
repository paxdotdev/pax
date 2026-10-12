fn random(cell: vec2<i32>, layer: u32) -> f32 {
    var h = bitcast<u32>(cell.x) * 374761393u + bitcast<u32>(cell.y) * 668265263u + parameters.seed * 2246822519u + layer * 3266489917u;
    h = (h ^ (h >> 13u)) * 1274126177u;
    return f32(h ^ (h >> 16u)) / 4294967295.0;
}

fn straight_color(c: vec4<f32>) -> vec3<f32> {
    return c.rgb / max(c.a, 0.00001);
}
fn pigment(c: vec4<f32>, variation: f32, saturation: f32, light: f32) -> vec4<f32> {
    let rgb = straight_color(c);
    let luma = dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let hue = vec3<f32>(0.11, 0.025, -0.09) * variation;
    let paint = mix(vec3<f32>(luma), rgb, saturation) + hue + vec3<f32>(light);
    return vec4<f32>(clamp(paint, vec3<f32>(0.0), vec3<f32>(1.0)) * c.a, c.a);
}
fn ground(c: vec4<f32>, amount: f32) -> vec4<f32> {
    // White pigment is mixed inside source coverage; transparent areas stay transparent.
    return vec4<f32>(mix(c.rgb, vec3<f32>(0.98, 0.965, 0.92) * c.a, amount), c.a);
}
fn noise(p: vec2<f32>, salt: u32) -> f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (vec2<f32>(3.0) - 2.0 * f);
    return mix(mix(random(cell,salt),random(cell + vec2<i32>(1,0),salt),u.x),
        mix(random(cell + vec2<i32>(0,1),salt),random(cell + vec2<i32>(1,1),salt),u.x),u.y);
}
fn brush_edge(distance: f32, spacing: f32) -> f32 {
    let pixel = max(input0_pixel_size().x, input0_pixel_size().y);
    let aa = clamp(pixel / spacing, 0.025, 0.3);
    return 1.0 - smoothstep(1.0 - aa, 1.0 + aa, distance);
}
fn tangent(field: vec4<f32>) -> vec2<f32> {
    let v = field.xy * 2.0 - vec2<f32>(1.0);
    return v / max(length(v), 0.001);
}

fn moving_mark(anchor: vec2<f32>, axis: vec2<f32>, spacing: f32,
    phase_offset: f32, travel: vec2<f32>, rate: f32) -> vec2<f32> {
    // Bounded, reversible orbits retain each seeded mark's identity. A spatial
    // phase wave carries motion through flat regions as well as around vortices.
    let t = parameters.phase * rate + phase_offset +
        dot(anchor,vec2<f32>(0.7,0.45)) / (parameters.size * 2.5);
    let offset = spacing * travel * vec2<f32>(sin(t),cos(t));
    return anchor + axis * offset.x + vec2<f32>(-axis.y,axis.x) * offset.y;
}
