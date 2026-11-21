// Alpha outlines work for arbitrary source colors. Palette selection also finds
// internal contours in two-color artwork, including antialiased boundaries.
fn surface_coverage(source: vec4<f32>) -> f32 {
    if parameters.select_color == 0u { return source.a; }
    let axis = parameters.surface_color.rgb - parameters.contrast_color.rgb;
    let signal = source.rgb - parameters.contrast_color.rgb * source.a;
    return clamp(dot(signal, axis) / max(dot(axis, axis), 0.0001), 0.0, source.a);
}

fn surface_gradient(p: vec2<f32>, radius: f32) -> vec2<f32> {
    return vec2<f32>(
        surface_coverage(sample_input0(p + vec2<f32>(radius, 0.0))) - surface_coverage(sample_input0(p - vec2<f32>(radius, 0.0))),
        surface_coverage(sample_input0(p + vec2<f32>(0.0, radius))) - surface_coverage(sample_input0(p - vec2<f32>(0.0, radius)))
    );
}

// Sample the round source image, including its soft spherical edge.
fn source_disc(distance_squared: f32, radius: f32) -> f32 {
    let r2 = distance_squared / (radius * radius);
    return sqrt(max(1.0 - r2, 0.0)) * (1.0 - smoothstep(0.85, 1.0, r2));
}

fn rim_source(normal: vec2<f32>, to_light: vec3<f32>) -> f32 {
    // A stylized contour facet redirects a viewing ray toward the light plane.
    // Intersect that ray with the finite spherical source: an entire straight
    // edge cannot stay lit merely because the source is on its facing side.
    let ray = normalize(vec3<f32>(normal * 0.85, 1.0));
    let along = dot(to_light, ray);
    let miss_squared = max(dot(to_light, to_light) - along * along, 0.0);
    return select(0.0, source_disc(miss_squared, parameters.light_radius), along > 0.0);
}

// Probe a shallow bevel in the container's plane. The angular offsets separate
// the three colors along a straight contour as well as around a curved corner.
fn scene_lobe(p: vec2<f32>, slope: vec2<f32>) -> f32 {
    // A broad specular lobe lets an interior light catch opposing contours.
    let rgb = scene_specular(p, normalize(vec3<f32>(slope, 1.0)), 16.0);
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
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
    let centers = vec3<f32>(0.0, 1.0, 2.0);
    let turns = (vec3<f32>(phase) - centers) / 3.0;
    let distance = 3.0 * abs(fract(turns + vec3<f32>(0.5)) - vec3<f32>(0.5));
    return smoothstep(vec3<f32>(0.18), vec3<f32>(0.82), distance);
}

fn cmy_rim(p: vec2<f32>) -> vec4<f32> {
    if parameters.rim.x <= 0.0 || parameters.rim.y <= 0.0 { return vec4<f32>(0.0); }
    // A bounded contour footprint cannot introduce a face-wide medial ridge.
    // Clip the result to the selected surface coverage in the composite below.
    let gradient = surface_gradient(p, parameters.rim.y);
    let edge = min(length(gradient), 1.0);
    if edge < 0.001 { return vec4<f32>(0.0); }
    let normal = -gradient / max(length(gradient), 0.001);
    let tangent = vec2<f32>(-normal.y, normal.x);
    var lobes: vec3<f32>;
    var exposure: f32;
    var opacity = 1.0;
    var chroma: vec3<f32>;
    if parameters.scene_mode != 0u {
        let bevel = normal * 0.065;
        let split = tangent * (0.16 * parameters.rim.z);
        lobes = vec3<f32>(scene_lobe(p, bevel - split), scene_lobe(p, bevel), scene_lobe(p, bevel + split));
        let peak = max(max(lobes.x, lobes.y), lobes.z);
        if peak <= parameters.threshold { return vec4<f32>(0.0); }
        chroma = scene_cmy(p, tangent, lobes, peak);
        exposure = smoothstep(parameters.threshold, parameters.threshold + 0.08, peak);
        opacity = parameters.light_color.a;
    } else {
        let to_light = vec3<f32>(parameters.light.xy - p, parameters.light.z);
        let angle = parameters.rim.z * 0.6981317;
        let cyan_normal = normal * cos(angle) - tangent * sin(angle);
        let yellow_normal = normal * cos(angle) + tangent * sin(angle);
        lobes = vec3<f32>(rim_source(cyan_normal, to_light), rim_source(normal, to_light), rim_source(yellow_normal, to_light));
        let spectrum = vec3<f32>(lobes.y + lobes.z, lobes.x + lobes.z, lobes.x + lobes.y);
        let low = min(min(spectrum.r, spectrum.g), spectrum.b);
        let high = max(max(spectrum.r, spectrum.g), spectrum.b);
        chroma = (spectrum - vec3<f32>(low)) / max(high - low, 0.0001);
        let illumination = to_light.z * to_light.z / dot(to_light, to_light);
        exposure = (1.0 - exp(-parameters.brightness * 0.6)) * parameters.light_color.a * illumination;
    }
    let peak = max(max(lobes.x, lobes.y), lobes.z);
    let spread = peak - min(min(lobes.x, lobes.y), lobes.z);
    // Fade smoothly to white when lobes coincide; zero dispersion stays neutral.
    let separation = smoothstep(0.0, 0.25, parameters.rim.z) * smoothstep(0.0, 0.01, spread);
    let tint = mix(parameters.light_color.rgb, chroma, parameters.rim.w * separation);
    // Keep tint and tone-mapped coverage separate. Adding emitted RGB to a white
    // glass face would clip every channel and erase the dispersed color.
    let energy = 1.0 - exp(-4.0 * parameters.rim.x * exposure * peak * edge);
    return vec4<f32>(tint, opacity * energy);
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    let coverage = surface_coverage(source);
    if coverage <= 0.0 || (parameters.scene_mode == 0u && parameters.brightness <= 0.0) { return source; }
    let rim = cmy_rim(p);
    // Preserve the source alpha and the unselected color at mixed AA pixels.
    if parameters.select_color != 0u {
        return vec4<f32>(source.rgb + coverage * rim.a * (rim.rgb - parameters.surface_color.rgb), source.a);
    }
    return vec4<f32>(mix(source.rgb, rim.rgb * source.a, rim.a), source.a);
}
