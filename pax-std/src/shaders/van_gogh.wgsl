fn curling_flow(p: vec2<f32>) -> vec2<f32> {
    let q = p / (parameters.size * 5.0) + vec2<f32>(
        random(vec2<i32>(0), 170u),random(vec2<i32>(1),171u)) * 6.283185;
    // Unequal wave speeds move and deform the whorls, rather than translating a
    // fixed pattern. Phase owns this motion independently of source animation.
    let t = parameters.phase * 0.22;
    let v = vec2<f32>(sin(q.y + 0.65 * sin(q.x * 0.7 + t * 0.67) + t * 0.85),
        cos(q.x + 0.65 * cos(q.y * 0.7 - t * 0.53) - t * 0.72));
    return v / max(length(v),0.001);
}
fn paint_van_gogh(p: vec2<f32>) -> vec4<f32> {
    var paint = pigment(sample_input0(p),-0.15,1.22,-0.025);
    for (var layer = 0u; layer < 2u; layer += 1u) {
        let spacing = parameters.size * select(0.82,0.43,layer == 1u);
        let cell = vec2<i32>(floor(p / spacing));
        var colors = vec4<f32>(0.0);
        var coverage = 0.0;
        for (var y = -2; y <= 2; y += 1) {
            for (var x = -2; x <= 2; x += 1) {
                let id = cell + vec2<i32>(x,y);
                let r = random(id,120u + layer);
                let anchor = (vec2<f32>(id) + vec2<f32>(0.5) +
                    vec2<f32>(r - 0.5,random(id,133u) - 0.5) * 0.55) * spacing;
                if (length(p - anchor) > spacing * 1.9) { continue; }
                let field = sample_input1(anchor);
                let flow = curling_flow(anchor);
                var edge_axis = tangent(field);
                edge_axis *= select(-1.0,1.0,dot(flow,edge_axis) >= 0.0);
                let direction = mix(flow,edge_axis,clamp(field.z * 2.2,0.0,0.8));
                let angle = atan2(direction.y,direction.x) + (r - 0.5) * 0.22;
                let axis = vec2<f32>(cos(angle),sin(angle));
                let normal = vec2<f32>(-axis.y,axis.x);
                let center = moving_mark(anchor,axis,spacing,r * 0.8,vec2<f32>(0.42,0.13),0.8);
                let d = p - center;
                let breath = 1.0 + 0.1 * sin(parameters.phase * 0.8 + r * 6.283185);
                let along = dot(d,axis) / (spacing * 1.02 * breath);
                let curve = dot(curling_flow(anchor + axis * spacing) -
                    curling_flow(anchor - axis * spacing),normal);
                let across = dot(d,normal) / (spacing * 0.28 / breath) -
                    (0.5 * curve + 0.35 * (r - 0.5)) * along * along;
                let shape = along * along + across * across;
                let mask = brush_edge(shape,spacing);
                if (mask <= 0.0) { continue; }
                let sample = sample_input0(center);
                // Paired highlight and shadow bands suggest raised impasto, not opacity holes.
                let ridge = across * 8.0 + sin(along * 4.0 + r * 6.0 - parameters.phase * 1.2) * 0.65;
                let frequency = 8.0 * max(input0_pixel_size().x,input0_pixel_size().y) / (spacing * 0.28);
                let fine = 1.0 - smoothstep(1.8,3.14,frequency * 2.1);
                let coarse = 1.0 - smoothstep(1.8,3.14,frequency);
                let relief = 0.095 * sin(ridge) * coarse + 0.03 * sin(ridge * 2.1) * fine -
                    0.04 * abs(across) + 0.07 * (r - 0.5);
                let color = pigment(sample,(r - 0.5) * 1.2,1.3,relief);
                let detail = select(1.0,0.48 + field.z * 0.5,layer == 1u);
                colors += color * mask * detail;
                coverage += mask * detail;
            }
        }
        paint = mix(paint,colors / max(coverage,0.0001),min(coverage,1.0) * 0.96);
    }
    return paint;
}
