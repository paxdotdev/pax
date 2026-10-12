fn paint_pointillist(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    var paint = ground(source,parameters.point_ground);
    let spacing = parameters.size * parameters.point_spacing;
    let cell = vec2<i32>(floor(p / spacing));
    var colors = vec4<f32>(0.0);
    var coverage = 0.0;
    // The wider neighborhood covers stagger, jitter, rotation, and overlapping dabs.
    for (var y = -2; y <= 2; y += 1) {
        for (var x = -2; x <= 2; x += 1) {
            let id = cell + vec2<i32>(x,y);
            let r = random(id,220u);
            let stagger = select(0.0,0.43,(id.y & 1) != 0);
            let anchor = (vec2<f32>(id) + vec2<f32>(0.5 + stagger,0.5) +
                vec2<f32>(r - 0.5,random(id,221u) - 0.5) * parameters.point_jitter) * spacing;
            if (length(p - anchor) > spacing * 1.55) { continue; }
            let group = noise(anchor / (parameters.size * 0.85),225u);
            var angle = (group - 0.5) * parameters.point_direction.x +
                (random(id,222u) - 0.5) * 0.5;
            if (parameters.point_direction.y > 0.0) {
                let field = sample_input1(anchor);
                let edge = tangent(field);
                let turn = atan2(edge.y,edge.x) - angle;
                // An axis is unoriented: choose the nearest equivalent contour direction.
                angle += 0.5 * atan2(sin(2.0 * turn),cos(2.0 * turn)) *
                    smoothstep(0.025,0.22,field.z) * parameters.point_direction.y;
            }
            let axis = vec2<f32>(cos(angle),sin(angle));
            let center = moving_mark(anchor,axis,spacing,group * 2.0,
                parameters.point_motion,0.8);
            let d = p - center;
            if (length(d) > spacing * 1.25) { continue; }
            let breath = 1.0 + 0.035 * sin(parameters.phase * 0.75 + r * 6.283185);
            let extent = spacing * parameters.point_extent *
                (vec2<f32>(0.8) + 0.4 * vec2<f32>(r,random(id,223u)));
            let along = dot(d,axis) / (extent.x * breath);
            let across = dot(d,vec2<f32>(-axis.y,axis.x)) / (extent.y / breath);
            let rectangle = max(abs(along),abs(across)) +
                0.045 * sin(along * 13.0 + r * 5.0) * sin(across * 9.0);
            let shape = mix(rectangle,length(vec2<f32>(along,across)),parameters.point_roundness);
            let mask = brush_edge(shape,spacing);
            if (mask <= 0.0) { continue; }
            let sample = sample_input0(center);
            let luma = dot(straight_color(sample),vec3<f32>(0.2126,0.7152,0.0722));
            let variation = (random(id,224u) - 0.5) * parameters.point_pigment.y;
            var dab = pigment(sample,variation,parameters.point_pigment.x,0.01);
            // Pale pigment occupies marks, not a continuous white grid between marks.
            let pale = select(0.0,parameters.point_white.y * mix(0.3,1.0,luma),
                random(id,226u) < parameters.point_white_chance);
            dab = ground(dab,parameters.point_white.x + pale);
            let ridge = parameters.point_relief * cos(across * 8.0 + r * 4.0);
            dab = vec4<f32>(clamp(dab.rgb + vec3<f32>(ridge * dab.a),vec3<f32>(0.0),vec3<f32>(dab.a)),dab.a);
            colors += dab * mask;
            coverage += mask;
        }
    }
    paint = mix(paint,colors / max(coverage,0.0001),min(coverage,1.0));
    return paint;
}
