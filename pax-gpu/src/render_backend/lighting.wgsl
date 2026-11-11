struct Material {
    surface: vec4<f32>,
    coefficients: vec4<f32>,
    emissive: vec4<f32>,
}


fn apply_lighting(
    color: vec4<f32>,
    material: Material,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    light_mask: u32,
    grain_position: vec3<f32>,
) -> vec4<f32> {
    if scene_lighting.flags.x == 0u {
        return color;
    }

    if material.coefficients.x < 0.0 {
        return color;
    }

    let ambient_coeff = material.coefficients.x;
    let diffuse_coeff = material.coefficients.y;
    let specular_coeff = material.coefficients.z;
    let roughness = clamp(material.coefficients.w, 0.0, 1.0);
    let light_count = min(scene_lighting.flags.y, 8u);
    let active_light_mask = (1u << light_count) - 1u;
    let eligible_light_mask = light_mask & active_light_mask;
    if eligible_light_mask == 0u && scene_lighting.flags.z == 0u {
        return color;
    }
    let view_dir = scene_view_at(scene_lighting.view, world_position);
    var frost_rgb = vec3<f32>(0.0);
    var frost_energy = 0.0;
    var lighting = scene_lighting.ambient.rgb * scene_lighting.ambient.a * ambient_coeff;

    for (var i = 0u; i < 8u; i++) {
        if i >= light_count {
            break;
        }
        if (eligible_light_mask & (1u << i)) == 0u {
            continue;
        }

        let light = scene_lighting.lights[i];
        let sampled = sample_scene_light(light, world_position);
        let light_dir = sampled.xyz;
        let attenuation = sampled.w;
        let diffuse = max(dot(normal, light_dir), 0.0);
        let specular = scene_specular_lobe(light_dir, view_dir, normal, mix(96.0, 8.0, roughness));
        let energy = light.color.a * attenuation * specular;
        frost_rgb += light.color.rgb * energy;
        frost_energy += energy;
        lighting += light.color.rgb * light.color.a * attenuation *
            ((diffuse_coeff * diffuse) + (specular_coeff * specular));
    }

    let lit_rgb = color.rgb * clamp(lighting, vec3<f32>(0.0), vec3<f32>(8.0))
        + material.emissive.rgb * material.emissive.a;
    if material.surface.x <= 0.0 { return vec4<f32>(lit_rgb, color.a); }
    // Grain is attached to the surface and band-limited to the raster footprint.
    let cell = vec3<i32>(floor(grain_position));
    var h = (bitcast<u32>(cell.x) * 1597334677u) ^ (bitcast<u32>(cell.y) * 3812015801u) ^ (bitcast<u32>(cell.z) * 2798796415u);
    h = (h ^ (h >> 16u)) * 2246822519u;
    let grain = 0.82 + 0.18 * f32(h & 65535u) / 65535.0;
    let sheen = 1.0 - exp(-clamp(material.surface.x, 0.0, 1.0) * frost_energy * grain);
    let sheen_color = frost_rgb / max(frost_energy, 0.0001);
    return vec4<f32>(mix(lit_rgb, sheen_color, sheen), color.a);
}
