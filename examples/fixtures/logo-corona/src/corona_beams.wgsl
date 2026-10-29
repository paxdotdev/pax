fn boundary_distance(p: vec2<f32>, angle: f32, low_radius: f32, high_radius: f32) -> f32 {
    let direction = vec2<f32>(cos(angle), sin(angle));
    if aperture(sample_input0(p + direction * high_radius)) <= 0.45 { return 1000.0; }
    var low = low_radius;
    var high = high_radius;
    for (var refine = 0; refine < 6; refine += 1) {
        let middle = 0.5 * (low + high);
        if aperture(sample_input0(p + direction * middle)) > 0.45 {
            high = middle;
        } else {
            low = middle;
        }
    }
    return high;
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    if parameters.emission < 0.001 || parameters.reach < 0.01 || aperture(source) > 0.999 {
        return vec4<f32>(0.0);
    }

    let reach = parameters.reach;
    var nearest = p;
    var distance = reach + 1.0;
    var nearest_angle = 0.0;
    var found = false;
    // Search the rendered stencil, not a settled logo or a duplicated outline.
    // All directions have equal reach, including into painted letter regions:
    // the emitting color defines the aperture, not the source's opaque silhouette.
    for (var ring = 1; ring <= 12; ring += 1) {
        let radius = reach * f32(ring) / 12.0;
        for (var ray = 0; ray < 24; ray += 1) {
            let angle = 6.283185307 * f32(ray) / 24.0;
            let point = p + vec2<f32>(cos(angle), sin(angle)) * radius;
            if aperture(sample_input0(point)) > 0.45 {
                // Refine every intersecting direction before choosing the nearest.
                // Taking the first hit makes straight edges jump sideways at each ring.
                let high = boundary_distance(p, angle, max(0.0, radius - reach / 12.0), radius);
                if high < distance {
                    nearest_angle = angle;
                    distance = high;
                }
                found = true;
            }
        }
        if found { break; }
    }
    if !found { return vec4<f32>(0.0); }

    // Resolve between the coarse directions instead of leaving angular wedges
    // around corners and curved edges. The search remains history-free and bounded.
    var angle_step = 6.283185307 / 48.0;
    for (var refine = 0; refine < 4; refine += 1) {
        let center_angle = nearest_angle;
        let radius = min(reach, distance + reach / 12.0);
        for (var side = -1; side <= 1; side += 2) {
            let angle = center_angle + f32(side) * angle_step;
            let candidate = boundary_distance(p, angle, 0.0, radius);
            if candidate < distance {
                distance = candidate;
                nearest_angle = angle;
            }
        }
        angle_step *= 0.5;
    }
    nearest = p + vec2<f32>(cos(nearest_angle), sin(nearest_angle)) * (distance + 1.0);

    let field = corona_field(nearest);
    let length = reach * mix(1.0, 0.28 + 0.72 * field, parameters.amplitude);
    let attenuation = pow(max(0.0, 1.0 - distance / max(length, 0.01)), parameters.falloff);
    let energy = clamp(parameters.emission * attenuation * (0.08 + 1.8 * field), 0.0, 0.95);
    return vec4<f32>(parameters.foreground.rgb * energy, energy);
}
