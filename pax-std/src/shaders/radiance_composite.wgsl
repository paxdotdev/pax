fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input1(p);
    if parameters.brightness <= 0.0 { return source; }
    // Reconstruct only the projected volume; never soften the source silhouette.
    var beams = sample_input0(p) * 0.4;
    beams += sample_input0(p + vec2<f32>(1.5, 0.0)) * 0.15;
    beams += sample_input0(p - vec2<f32>(1.5, 0.0)) * 0.15;
    beams += sample_input0(p + vec2<f32>(0.0, 1.5)) * 0.15;
    beams += sample_input0(p - vec2<f32>(0.0, 1.5)) * 0.15;

    let glass = glass_coverage(source);
    var face = source.rgb;
    if glass > 0.0 && (parameters.transmission > 0.0 || parameters.polish > 0.0) {
        let to_light = parameters.light.xy - p;
        let light_direction = to_light / max(length(to_light), 0.001);
        let depth = parameters.light.z;
        let illumination = depth * depth / (depth * depth + dot(to_light, to_light));
        let gain = 1.0 - exp(-parameters.brightness * 0.6);

        // A short, fixed footprint gives only a contour normal. It has no nearest-
        // edge selection or distance-field midline that could bevel a letter's face.
        let h = 3.0;
        let gradient = vec2<f32>(
            glass_coverage(sample_input1(p + vec2<f32>(h, 0.0))) - glass_coverage(sample_input1(p - vec2<f32>(h, 0.0))),
            glass_coverage(sample_input1(p + vec2<f32>(0.0, h))) - glass_coverage(sample_input1(p - vec2<f32>(0.0, h)))
        );
        let edge = min(length(gradient), 1.0);
        let normal = -gradient / max(length(gradient), 0.001);
        let rim = edge * pow(max(dot(normal, light_direction), 0.0), 3.0) * 0.28;

        // A small refraction offset exposes the moving beam field inside the glass.
        // Absorption keeps it dark; source coverage still defines the exact AA edge.
        let refracted = sample_input0(p - light_direction * 7.0 + normal * 2.0);
        let absorption = vec3<f32>(0.82, 0.88, 1.0);
        let through = (refracted.rgb * absorption - parameters.background.rgb * refracted.a) * parameters.transmission;

        // A stylized polished reflection tied to the same light, with no independent clock.
        let axis = normalize(vec2<f32>(0.9, 0.38 + (parameters.light.x - 460.0) / 2200.0));
        let across = dot(p - parameters.light.xy, axis);
        let along = dot(p - parameters.light.xy, vec2<f32>(-axis.y, axis.x));
        let ribbon = exp(-0.5 * pow(across / 23.0, 2.0) - 0.5 * pow(along / 185.0, 2.0));
        let shoulder = exp(-0.5 * pow(across / 80.0, 2.0) - 0.5 * pow(along / 230.0, 2.0));
        let sheen = parameters.polish * gain * illumination * (rim + 0.22 * ribbon + 0.025 * shoulder);
        face += glass * (through + vec3<f32>(0.82, 0.9, 1.0) * sheen);
    }
    // Glass transmits this captured light field, not arbitrary scene content behind
    // the effect. Keep source coverage opaque and preserve transparent exteriors.
    return vec4<f32>(face, source.a) + beams * (1.0 - source.a);
}
