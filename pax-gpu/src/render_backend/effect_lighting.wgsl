struct EffectLighting {
    scene: SceneLighting,
    origin: vec4<f32>,
    tangent_x: vec4<f32>,
    tangent_y: vec4<f32>,
    normal: vec4<f32>,
    mask: vec4<u32>,
}
@group(0) @binding(5) var<uniform> effect_lighting: EffectLighting;

// The runtime supplies world coordinates and scope independently of capture pixels.
fn scene_world_position(p: vec2<f32>) -> vec3<f32> {
    return effect_lighting.origin.xyz + p.x * effect_lighting.tangent_x.xyz + p.y * effect_lighting.tangent_y.xyz;
}

// Facet slopes are measured in an orthonormal tangent frame, so scaling a glyph
// preserves its surface roughness while rotation turns its normals with the glyph.
fn scene_world_normal(normal: vec3<f32>) -> vec3<f32> {
    let x = effect_lighting.tangent_x.xyz / max(length(effect_lighting.tangent_x.xyz), 0.0001);
    let off_axis = effect_lighting.tangent_y.xyz - x * dot(effect_lighting.tangent_y.xyz, x);
    let y = off_axis / max(length(off_axis), 0.0001);
    let n = x * normal.x + y * normal.y + effect_lighting.normal.xyz * normal.z;
    return n / max(length(n), 0.0001);
}

fn scene_light_count() -> u32 {
    let slots = select(0u, min(effect_lighting.scene.flags.y, 8u), effect_lighting.scene.flags.x != 0u);
    return countOneBits(effect_lighting.mask.x & ((1u << slots) - 1u));
}

fn scene_ambient() -> vec3<f32> {
    let ambient = effect_lighting.scene.ambient;
    let eligible = scene_light_count();
    let enabled = effect_lighting.scene.flags.x != 0u && (eligible != 0u || effect_lighting.scene.flags.z != 0u);
    return select(vec3<f32>(0.0), ambient.rgb * ambient.a, enabled);
}

// Colored specular irradiance from eligible point, strip and directional lights.
// p is effect-local; normal is in the tangent frame; shininess must be positive.
fn scene_specular(p: vec2<f32>, normal: vec3<f32>, shininess: f32) -> vec3<f32> {
    let position = scene_world_position(p);
    let world_normal = scene_world_normal(normal);
    let view = scene_view_at(effect_lighting.scene.view, position);
    var energy = vec3<f32>(0.0);
    if effect_lighting.scene.flags.x == 0u { return energy; }
    for (var i = 0u; i < min(effect_lighting.scene.flags.y, 8u); i++) {
        if (effect_lighting.mask.x & (1u << i)) == 0u { continue; }
        let light = effect_lighting.scene.lights[i];
        let sampled = sample_scene_light(light, position);
        let lobe = scene_specular_lobe(sampled.xyz, view, world_normal, max(shininess, 0.0001));
        energy += light.color.rgb * light.color.a * sampled.w * lobe;
    }
    return energy;
}
