struct SceneRim {
    lobes: vec3<f32>,
    incidence: f32,
    focused_share: f32,
    travel: f32,
}

// Probe a shallow bevel in the container's plane. The angular offsets separate
// the three colors along a straight contour as well as around a curved corner.
fn scene_rim_lobes(p: vec2<f32>, normal: vec2<f32>, tangent: vec2<f32>, dispersion: f32, span: f32) -> SceneRim {
    let bevel = normal * 0.065;
    let split = tangent * (0.16 * dispersion);
    let a = scene_world_normal(vec3<f32>(bevel - split, 1.0));
    let b = scene_world_normal(vec3<f32>(bevel, 1.0));
    let c = scene_world_normal(vec3<f32>(bevel + split, 1.0));
    let view = scene_view_direction(p);
    let origin = scene_world_position(vec2<f32>(0.0));
    let axis = scene_world_position(tangent) - origin;
    let edge_scale = max(length(axis), 0.0001);
    let edge_axis = axis / edge_scale;
    let width = length(scene_world_position(vec2<f32>(domain.local_size.x, 0.0)) - origin);
    let half_span = 0.5 * width * span;
    var lobes = vec3<f32>(0.0);
    var incidence_sum = 0.0;
    var focused_energy = 0.0;
    var mask = scene_light_mask();
    while mask != 0u {
        let light = scene_incident_light(p, firstTrailingBit(mask));
        mask &= mask - 1u;
        var energy = dot(light.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        var incidence = 0.0;
        if half_span > 0.0 && light.distance >= 0.0 {
            // Focus along the edge, not across it: a pointer in the middle
            // of a button can still catch both the upper and lower bevels.
            incidence = dot(light.direction, edge_axis) * light.distance / half_span;
            energy *= 1.0 - smoothstep(0.0, 1.0, abs(incidence));
        }
        let h = light.direction + view;
        let half_dir = h / max(length(h), 0.0001);
        let response = energy * pow(max(vec3<f32>(dot(a, half_dir), dot(b, half_dir), dot(c, half_dir)),
                                        vec3<f32>(0.0)), vec3<f32>(16.0));
        lobes += response;
        if half_span > 0.0 && light.distance >= 0.0 {
            incidence_sum += incidence * response.y;
            focused_energy += response.y;
        }
    }
    let incidence = incidence_sum / max(focused_energy, 0.0001);
    // Project back from this sample towards the glint's center along the edge.
    // Driving phase from the center (rather than the pixel) rolls the colors as
    // the light travels without painting extra repeats into a stationary glint.
    let center = p + tangent * (incidence * half_span / edge_scale);
    let travel = (center.x + center.y) / max(domain.local_size.x, 1.0);
    return SceneRim(lobes, incidence, focused_energy / max(lobes.y, 0.0001), travel);
}

fn cmy_palette(turns: f32) -> vec3<f32> {
    let centers = vec3<f32>(0.0, 1.0 / 3.0, 2.0 / 3.0);
    let distance = 3.0 * abs(fract(vec3<f32>(turns + 0.5) - centers) - vec3<f32>(0.5));
    return smoothstep(vec3<f32>(0.18), vec3<f32>(0.82), distance);
}

fn focused_spectrum(p: vec2<f32>, scene: SceneRim, peak: f32, threshold: f32, cycles: f32, travel: f32) -> vec3<f32> {
    // Measure the fraction of the falloff traversed *before the threshold*.
    // A dim pointer and a bright strip then each fit the same number of bands.
    let loss = smoothstep(0.0, 1.0, abs(scene.incidence));
    let visible_loss = 1.0 - (1.0 - loss) * threshold / peak;
    let u = sign(scene.incidence) * sqrt(clamp(loss / max(visible_loss, 0.0001), 0.0, 1.0));
    // The bend vanishes at both ends; hue_cycles sets density and hue_travel
    // offsets the whole palette, independent of the light's intensity or age.
    let bend = 0.08 * (1.0 - u * u) * sin(dot(p, vec2<f32>(0.023, 0.017)));
    return cmy_palette(1.0 / 3.0 + travel * scene.travel + 0.5 * cycles * (u + bend));
}

fn scene_cmy(p: vec2<f32>, tangent: vec2<f32>, lobes: vec3<f32>, peak: f32) -> vec3<f32> {
    // Opposing contours reverse both the tangent and the side-probe order.
    // Project their difference onto the palette's travel direction so the
    // bands keep the same spacing on both sides of a centered light.
    let travel = vec2<f32>(1.0, 0.25);
    let band = 2.6 * (lobes.z - lobes.x) / peak * dot(tangent, travel);
    let phase = band + dot(p, travel) / 80.0;
    // Each pigment removes one RGB channel. Hold its pure color at the center
    // of each band and blend neighboring pigments across the transition.
    return cmy_palette(phase / 3.0);
}

fn rim_tint(lobes: vec3<f32>, peak: f32, chroma: vec3<f32>, rim: vec4<f32>,
            light_color: vec4<f32>, exposure: f32, opacity: f32, edge: f32) -> vec4<f32> {
    let spread = peak - min(min(lobes.x, lobes.y), lobes.z);
    // Fade smoothly to white when lobes coincide; zero dispersion stays neutral.
    let separation = smoothstep(0.0, 0.25, rim.z) * smoothstep(0.0, 0.01, spread);
    let tint = mix(light_color.rgb, chroma, rim.w * separation);
    // Keep tint and tone-mapped coverage separate. Adding emitted RGB to a white
    // glass face would clip every channel and erase the dispersed color.
    let energy = 1.0 - exp(-4.0 * rim.x * exposure * peak * edge);
    return vec4<f32>(tint, opacity * energy);
}

fn scene_glint(p: vec2<f32>, normal: vec2<f32>, tangent: vec2<f32>, edge: f32,
               rim: vec4<f32>, light_color: vec4<f32>, span: f32, threshold: f32, cycles: f32, travel: f32) -> vec4<f32> {
    let dispersion = rim.z;
    var chroma: vec3<f32>;
    let scene = scene_rim_lobes(p, normal, tangent, dispersion, span);
    let lobes = scene.lobes;
    let peak = max(max(lobes.x, lobes.y), lobes.z);
    if peak <= threshold { return vec4<f32>(0.0); }
    if scene.focused_share >= 1.0 {
        chroma = focused_spectrum(p, scene, peak, threshold, cycles, travel);
    } else {
        chroma = scene_cmy(p, tangent, lobes, peak);
        if scene.focused_share > 0.0 {
            chroma = mix(chroma, focused_spectrum(p, scene, peak, threshold, cycles, travel), scene.focused_share);
        }
    }
    let exposure = smoothstep(threshold, threshold + 0.08, peak);
    return rim_tint(lobes, peak, chroma, rim, light_color, exposure, light_color.a, edge);
}
