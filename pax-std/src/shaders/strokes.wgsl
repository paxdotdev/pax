fn random(cell: vec2<i32>, layer: u32) -> f32 {
    var h = bitcast<u32>(cell.x) * 374761393u + bitcast<u32>(cell.y) * 668265263u + parameters.seed * 2246822519u + layer * 3266489917u;
    h = (h ^ (h >> 13u)) * 1274126177u;
    return f32(h ^ (h >> 16u)) / 4294967295.0;
}
fn effect(p: vec2<f32>) -> vec4<f32> {
    var paint = sample_input0(p);
    for (var layer = 0u; layer < 3u; layer += 1u) {
        let spacing = parameters.size * pow(0.56, f32(layer));
        let cell = vec2<i32>(floor(p / spacing));
        var colors = vec4<f32>(0.0);
        var coverage = 0.0;
        for (var y = -1; y <= 1; y += 1) {
            for (var x = -1; x <= 1; x += 1) {
                let id = cell + vec2<i32>(x,y);
                let r = random(id,layer);
                let jitter = vec2<f32>(r - 0.5, random(id,layer + 19u) - 0.5) * 0.55;
                let center = (vec2<f32>(id) + vec2<f32>(0.5) + jitter) * spacing;
                let field = sample_input1(center);
                let angle = atan2(field.y * 2.0 - 1.0, field.x * 2.0 - 1.0) + (r - 0.5) * 0.8;
                let axis = vec2<f32>(cos(angle),sin(angle));
                let delta = p - center;
                let breath = 1.0 + 0.12 * sin(parameters.phase * 0.8 + r * 6.283185);
                let along = dot(delta,axis) / (spacing * 0.75 * breath);
                let across = dot(delta,vec2<f32>(-axis.y,axis.x)) / (spacing * 0.26 / breath);
                let edge = along * along + across * across;
                let bristle = 0.83 + 0.17 * cos(across * 22.0 + r * 5.0);
                let mark = (1.0 - smoothstep(0.65,1.0,edge)) * bristle;
                let sample = sample_input0(center);
                let tint = vec3<f32>(1.0 + (r - 0.5) * 0.22, 1.0 + (r - 0.5) * 0.12, 1.0 - (r - 0.5) * 0.08);
                let color = vec4<f32>(clamp(sample.rgb * tint, vec3<f32>(0.0), vec3<f32>(sample.a)), sample.a);
                // Fine marks concentrate around details, while coarse marks establish the field.
                let detail = select(1.0,0.32 + field.z * 0.68,layer > 0u);
                colors += color * mark * detail;
                coverage += mark * detail;
            }
        }
        let opacity = min(coverage,1.0) * 0.9;
        paint = mix(paint, colors / max(coverage,0.0001),opacity);
    }
    return paint;
}
