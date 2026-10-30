fn effect(p: vec2<f32>) -> vec4<f32> {
    if parameters.brightness <= 0.0 || parameters.reach <= 0.0 {
        return vec4<f32>(0.0);
    }
    let light = parameters.light.xy;
    let depth = parameters.light.z;
    let volume = parameters.light.w;
    let radius = max(length(p - light), 0.0001);
    let direction = (p - light) / radius;
    let limit = min(parameters.reach, radius * volume / (depth + volume));
    let step = limit / 96.0;
    var radiance = 0.0;
    // Integrate expanded aperture slices about one shared point light. Uniform
    // screen-distance sampling bounds the stencil step even far from the light;
    // uniform depth steps would skip thin strokes and produce repeating bands.
    for (var i = 0u; i < 96u; i = i + 1u) {
        let distance = (f32(i) + 0.5) * step;
        let remaining = max(radius - distance, 0.0001);
        let z = depth * distance / remaining;
        let q = p - direction * distance;
        let incidence = pow(depth * depth / (depth * depth + dot(q - light, q - light)), 0.6);
        let attenuation = pow(1.0 - distance / parameters.reach, parameters.falloff);
        let volume_fade = pow(1.0 - z / volume, 2.0);
        // Change of variables preserves the depth integral while sampling q evenly.
        let dz = depth * radius / (remaining * remaining) * step;
        let source = sample_input0(q);
        let transmitted_aperture = aperture(source) + glass_coverage(source) * parameters.transmission;
        radiance += transmitted_aperture * attenuation * incidence * volume_fade * dz / volume;
    }
    let alpha = 1.0 - exp(-parameters.brightness * radiance);
    return vec4<f32>(parameters.foreground.rgb * alpha, alpha);
}
