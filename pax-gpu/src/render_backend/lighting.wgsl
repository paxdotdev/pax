struct Material {
    coefficients: vec4<f32>,
    emissive: vec4<f32>,
}


struct SceneLight {
    position: vec4<f32>,
    direction: vec4<f32>,
    color: vec4<f32>,
    params: vec4<f32>,
}

struct SceneLighting {
    surface_to_world_x: vec4<f32>,
    surface_to_world_y: vec4<f32>,
    view: vec4<f32>,
    ambient: vec4<f32>,
    flags: vec4<u32>,
    lights: array<SceneLight, 8>,
}



fn apply_lighting(
    color: vec4<f32>,
    material: Material,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    light_mask: u32,
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
    let surface_position = world_position;
    let to_view = scene_lighting.view.xyz - world_position * scene_lighting.view.w;
    let view_dir = to_view / max(length(to_view), 0.0001);
    var lighting = scene_lighting.ambient.rgb * scene_lighting.ambient.a * ambient_coeff;

    for (var i = 0u; i < 8u; i++) {
        if i >= light_count {
            break;
        }
        if (eligible_light_mask & (1u << i)) == 0u {
            continue;
        }

        let light = scene_lighting.lights[i];
        var light_dir: vec3<f32>;
        var attenuation = 1.0;
        if light.params.x < 0.5 {
            let to_light = light.position.xyz - surface_position;
            let distance = max(length(to_light), 0.0001);
            let radius = max(light.params.y, 0.0001);
            let falloff = max(1.0 - distance / radius, 0.0);
            attenuation = falloff * falloff;
            light_dir = to_light / distance;
        } else {
            let direction_len = max(length(light.direction.xyz), 0.0001);
            light_dir = -light.direction.xyz / direction_len;
        }

        let diffuse = max(dot(normal, light_dir), 0.0);
        let half_sum = light_dir + view_dir;
        let half_dir = half_sum / max(length(half_sum), 0.0001);
        let shininess = mix(96.0, 8.0, roughness);
        let specular = pow(max(dot(normal, half_dir), 0.0), shininess);
        lighting += light.color.rgb * light.color.a * attenuation *
            ((diffuse_coeff * diffuse) + (specular_coeff * specular));
    }

    let lit_rgb = color.rgb * clamp(lighting, vec3<f32>(0.0), vec3<f32>(8.0))
        + material.emissive.rgb * material.emissive.a;
    return vec4<f32>(lit_rgb, color.a);
}


