fn glass_gradient(p: vec2<f32>, radius: f32) -> vec2<f32> {
    return vec2<f32>(
        glass_coverage(sample_input1(p + vec2<f32>(radius, 0.0))) - glass_coverage(sample_input1(p - vec2<f32>(radius, 0.0))),
        glass_coverage(sample_input1(p + vec2<f32>(0.0, radius))) - glass_coverage(sample_input1(p - vec2<f32>(0.0, radius)))
    );
}

fn filtered_projection(p: vec2<f32>) -> vec4<f32> {
    var light = sample_input0(p) * 0.4;
    light += sample_input0(p + vec2<f32>(1.5, 0.0)) * 0.15;
    light += sample_input0(p - vec2<f32>(1.5, 0.0)) * 0.15;
    light += sample_input0(p + vec2<f32>(0.0, 1.5)) * 0.15;
    light += sample_input0(p - vec2<f32>(0.0, 1.5)) * 0.15;
    return light;
}

// Disperse the captured light volume, never the crisp source image. The bounded
// transverse offsets approximate three nearby spectral directions about one light.
fn projected_light(p: vec2<f32>) -> vec4<f32> {
    let neutral = filtered_projection(p);
    if parameters.prism.x <= 0.0 || parameters.prism.y <= 0.0 { return neutral; }
    let ray = p - parameters.light.xy;
    let radius = max(length(ray), 0.001);
    let tangent = vec2<f32>(-ray.y, ray.x) / radius;
    let offset = tangent * min(radius * 0.09, 36.0) * parameters.prism.x;
    let lobes = vec3<f32>(filtered_projection(p - offset).a, neutral.a, filtered_projection(p + offset).a);
    let spectrum = vec3<f32>(lobes.y + lobes.z, lobes.x + lobes.z, lobes.x + lobes.y);
    let low = min(min(spectrum.r, spectrum.g), spectrum.b);
    let high = max(max(spectrum.r, spectrum.g), spectrum.b);
    let chroma = (spectrum - vec3<f32>(low)) / max(high - low, 0.00001);
    // Remove the shared white component before applying color.
    // Equal lobes converge smoothly to neutral; bright light cannot wash out tint.
    let separation = smoothstep(0.0, 0.015, high - low) * smoothstep(0.0, 0.04, parameters.prism.x);
    let tint = mix(parameters.light_color.rgb, chroma, parameters.prism.y * separation);
    let coverage = max(max(lobes.x, lobes.y), lobes.z);
    return vec4<f32>(tint * coverage, coverage);
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input1(p);
    if parameters.brightness <= 0.0 { return source; }
    let glass = glass_coverage(source);
    var face = source.rgb;
    if glass > 0.0 && (parameters.transmission > 0.0 || parameters.polish > 0.0) {
        let to_light = parameters.light.xy - p;
        let depth = parameters.light.z;
        let source_vector = vec3<f32>(to_light, depth);
        let light_direction = normalize(source_vector).xy;
        let illumination = depth * depth / (depth * depth + dot(to_light, to_light));
        let gain = (1.0 - exp(-parameters.brightness * 0.6)) * parameters.light_color.a;

        // A small refraction offset exposes the moving beam field inside the glass.
        // Absorption keeps it dark; source coverage still defines the exact AA edge.
        var through = vec3<f32>(0.0);
        if parameters.transmission > 0.0 {
            // A short, fixed footprint gives only a contour normal. It has no nearest-
            // edge selection or distance-field midline that could bevel a letter's face.
            let gradient = glass_gradient(p, 3.0);
            let normal = -gradient / max(length(gradient), 0.001);
            let refracted_position = p - light_direction * 7.0 + normal * 2.0;
            var refracted: vec4<f32>;
            if parameters.prism.x > 0.0 && parameters.prism.y > 0.0 {
                refracted = projected_light(refracted_position);
            } else {
                refracted = sample_input0(refracted_position);
            }
            let absorption = vec3<f32>(0.82, 0.88, 1.0);
            through = (refracted.rgb * absorption - parameters.background.rgb * refracted.a) * parameters.transmission;
        }

        // Keep the polished reflection vertical throughout the light path.
        let across = p.x - parameters.light.x;
        let along = p.y - parameters.light.y;
        let ribbon = exp(-0.5 * pow(across / 23.0, 2.0) - 0.5 * pow(along / 185.0, 2.0));
        let shoulder = exp(-0.5 * pow(across / 80.0, 2.0) - 0.5 * pow(along / 230.0, 2.0));
        let sheen = parameters.polish * gain * illumination * (0.22 * ribbon + 0.025 * shoulder);
        let polished = clamp(parameters.background.rgb + through
            + parameters.light_color.rgb * vec3<f32>(0.82, 0.9, 1.0) * sheen,
            vec3<f32>(0.0), vec3<f32>(1.0));
        // Blend only the glass fraction; the aperture and original AA coverage
        // remain untouched.
        face += glass * (polished - parameters.background.rgb);
    }
    // Glass transmits this captured light field, not arbitrary scene content behind
    // the effect. Keep source coverage opaque and preserve transparent exteriors.
    var result = vec4<f32>(face, source.a);
    if source.a < 1.0 {
        // Opaque source pixels hide the exterior projection completely. Retain
        // its contribution at partial/antialiased edges without sampling it twice
        // across the opaque glass interior (up to 15 texture reads in prism mode).
        result += projected_light(p) * (1.0 - source.a);
    }
    return result;
}
