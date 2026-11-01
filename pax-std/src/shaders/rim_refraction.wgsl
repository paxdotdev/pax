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

fn cmy_rim(p: vec2<f32>, to_light: vec3<f32>, light_exposure: f32) -> vec4<f32> {
    if parameters.rim.x <= 0.0 || parameters.rim.y <= 0.0 { return vec4<f32>(0.0); }
    // A bounded contour footprint cannot introduce a face-wide medial ridge.
    // Clip the result to the selected surface coverage in the composite below.
    let gradient = surface_gradient(p, parameters.rim.y);
    let edge = min(length(gradient), 1.0);
    if edge < 0.001 { return vec4<f32>(0.0); }
    let normal = -gradient / max(length(gradient), 0.001);
    let angle = parameters.rim.z * 0.6981317;
    let tangent = vec2<f32>(-normal.y, normal.x);
    let cyan_normal = normal * cos(angle) - tangent * sin(angle);
    let yellow_normal = normal * cos(angle) + tangent * sin(angle);
    let lobes = vec3<f32>(rim_source(cyan_normal, to_light),
        rim_source(normal, to_light), rim_source(yellow_normal, to_light));
    // Separate chromaticity from exposure: summing the three broad CMY lobes
    // directly washed out their color and halved the already narrow highlight.
    let spectrum = vec3<f32>(lobes.y + lobes.z, lobes.x + lobes.z, lobes.x + lobes.y);
    let low = min(min(spectrum.r, spectrum.g), spectrum.b);
    let high = max(max(spectrum.r, spectrum.g), spectrum.b);
    let chroma = (spectrum - vec3<f32>(low)) / max(high - low, 0.0001);
    // Fade smoothly to white when lobes coincide; zero dispersion stays neutral.
    let separation = smoothstep(0.0, 0.25, parameters.rim.z) * smoothstep(0.0, 0.01, high - low);
    let tint = mix(parameters.light_color.rgb, chroma, parameters.rim.w * separation);
    let facing = max(max(lobes.x, lobes.y), lobes.z);
    // Keep tint and tone-mapped coverage separate. Adding emitted RGB to a white
    // glass face would clip every channel and erase the dispersed color.
    let energy = 1.0 - exp(-4.0 * parameters.rim.x * light_exposure * facing * edge);
    return vec4<f32>(tint, energy);
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    let coverage = surface_coverage(source);
    if coverage <= 0.0 || parameters.brightness <= 0.0 { return source; }
    let to_light = vec3<f32>(parameters.light.xy - p, parameters.light.z);
    let illumination = to_light.z * to_light.z / dot(to_light, to_light);
    let gain = (1.0 - exp(-parameters.brightness * 0.6)) * parameters.light_color.a;
    let rim = cmy_rim(p, to_light, gain * illumination);
    // Preserve the source alpha and the unselected color at mixed AA pixels.
    if parameters.select_color != 0u {
        return vec4<f32>(source.rgb + coverage * rim.a * (rim.rgb - parameters.surface_color.rgb), source.a);
    }
    return vec4<f32>(mix(source.rgb, rim.rgb * source.a, rim.a), source.a);
}
