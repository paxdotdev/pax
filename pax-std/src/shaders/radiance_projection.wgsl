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
    let depth_squared = depth * depth;
    let depth_over_volume = depth / volume;
    let integral_scale = depth_over_volume * radius * step;
    var radiance = 0.0;
    // Integrate expanded aperture slices about one shared point light. Uniform
    // screen-distance sampling bounds the stencil step even far from the light;
    // uniform depth steps would skip thin strokes and produce repeating bands.
    for (var i = 0u; i < 96u; i = i + 1u) {
        let distance = (f32(i) + 0.5) * step;
        let q = p - direction * distance;
        let source = sample_input0(q);
        // Empty stencil samples cannot contribute. Avoid evaluating the volume's
        // powers and divisions over the transparent majority of the capture.
        if source.a > 0.0 {
            let transmitted_aperture = mix(aperture(source), source.a, parameters.transmission);
            if transmitted_aperture > 0.0 {
                let remaining = max(radius - distance, 0.0001);
                let incidence = pow(depth_squared / (depth_squared + dot(q - light, q - light)), 0.6);
                var attenuation = 1.0 - distance / parameters.reach;
                // The default linear falloff needs no general exponentiation.
                if parameters.falloff != 1.0 {
                    attenuation = pow(attenuation, parameters.falloff);
                }
                let fade = 1.0 - depth_over_volume * distance / remaining;
                // Change of variables preserves the depth integral while sampling q evenly.
                let weight = fade / remaining;
                radiance += transmitted_aperture * attenuation * incidence * weight * weight;
            }
        }
    }
    let alpha = (1.0 - exp(-parameters.brightness * radiance * integral_scale)) * parameters.light_color.a;
    return vec4<f32>(parameters.light_color.rgb * alpha, alpha);
}
