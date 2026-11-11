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



// Shared by vector materials and opt-in shader effects. xyz is the direction
// toward the emitter; w is its finite-radius attenuation.
fn sample_scene_light(light: SceneLight, position: vec3<f32>) -> vec4<f32> {
    if light.params.x >= 0.5 && light.params.x <= 1.5 {
        return vec4<f32>(-light.direction.xyz / max(length(light.direction.xyz), 0.0001), 1.0);
    }
    var emitter = light.position.xyz;
    if light.params.x > 1.5 {
        let extent = light.direction.xyz;
        let squared = dot(extent, extent);
        if squared > 0.000001 {
            emitter += extent * clamp(dot(position - emitter, extent) / squared, -1.0, 1.0);
        }
    }
    let delta = emitter - position;
    let distance = max(length(delta), 0.0001);
    let falloff = max(1.0 - distance / max(light.params.y, 0.0001), 0.0);
    return vec4<f32>(delta / distance, falloff * falloff);
}

fn scene_view_at(view: vec4<f32>, position: vec3<f32>) -> vec3<f32> {
    let delta = view.xyz - position * view.w;
    return delta / max(length(delta), 0.0001);
}

fn scene_specular_lobe(direction: vec3<f32>, view: vec3<f32>, normal: vec3<f32>, shininess: f32) -> f32 {
    let half_sum = direction + view;
    let half_dir = half_sum / max(length(half_sum), 0.0001);
    return pow(max(dot(normal, half_dir), 0.0), shininess);
}
