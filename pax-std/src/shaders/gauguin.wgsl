fn paint_gauguin(p: vec2<f32>) -> vec4<f32> {
    var paint = pigment(sample_input0(p), 0.12, 1.16, 0.0);
    // Long, overlapping sweeps build planes rather than a field of tiny dabs.
    for (var layer = 0u; layer < 2u; layer += 1u) {
        let spacing = parameters.size * select(1.35, 0.72, layer == 1u);
        let cell = vec2<i32>(floor(p / spacing));
        var colors = vec4<f32>(0.0);
        var coverage = 0.0;
        for (var y = -2; y <= 2; y += 1) {
            for (var x = -2; x <= 2; x += 1) {
                let id = cell + vec2<i32>(x,y);
                let r = random(id, 80u + layer);
                let anchor = (vec2<f32>(id) + vec2<f32>(0.5) +
                    vec2<f32>(r - 0.5, random(id,92u) - 0.5) * 0.35) * spacing;
                if (length(p - anchor) > spacing * 1.75) { continue; }
                let field = sample_input1(anchor);
                let sweep = -0.45 + noise(anchor / (parameters.size * 5.0), 87u) * 1.8;
                let gesture = vec2<f32>(cos(sweep), sin(sweep));
                var edge_axis = tangent(field);
                edge_axis *= select(-1.0, 1.0, dot(edge_axis,gesture) >= 0.0);
                let direction = mix(gesture, edge_axis, clamp(field.z * 3.0, 0.0, 0.85));
                let axis = direction / max(length(direction), 0.001);
                let center = moving_mark(anchor,axis,spacing,r * 0.7,vec2<f32>(0.2,0.07),0.45);
                if (length(p - center) > spacing * 1.5) { continue; }
                let d = p - center;
                let breath = 1.0 + 0.06 * sin(parameters.phase * 0.65 + r * 6.283185);
                let along = dot(d,axis) / (spacing * 1.32 * breath);
                let across = dot(d,vec2<f32>(-axis.y,axis.x)) / (spacing * 0.43 / breath);
                let bend = across + 0.24 * (r - 0.5) * along * along;
                let tip = abs(along) + 0.08 * sin(bend * 17.0 + r * 8.0);
                let shape = max(tip, abs(bend) + 0.12 * along * along);
                let mask = brush_edge(shape, spacing);
                if (mask <= 0.0) { continue; }
                let sample = sample_input0(center) * 0.6 +
                    (sample_input0(center - axis * spacing * 0.25) +
                     sample_input0(center + axis * spacing * 0.25)) * 0.2;
                let flat = floor(straight_color(sample) * 18.0 + 0.5) / 18.0;
                let bristles = 0.016 * sin(bend * 57.0 + sin(along * 5.0) + r * 9.0);
                let color = pigment(vec4<f32>(flat * sample.a,sample.a),
                    (r - 0.5) * 0.55, 1.12, bristles);
                let detail = select(1.0, 0.18 + field.z * 0.65, layer == 1u);
                colors += color * mask * detail;
                coverage += mask * detail;
            }
        }
        paint = mix(paint,colors / max(coverage,0.0001),min(coverage,1.0) * 0.94);
    }
    return paint;
}
