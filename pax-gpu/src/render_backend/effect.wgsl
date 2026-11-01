struct EffectDomain {
    rect: vec4<f32>,
    local0: vec4<f32>,
    local1: vec4<f32>,
    local2: vec4<f32>,
    surface0: vec4<f32>,
    surface1: vec4<f32>,
    surface2: vec4<f32>,
    camera_z: vec4<f32>,
    camera_w: vec4<f32>,
    // Active source pixels and allocated capacity. All passes share this domain.
    storage: vec4<f32>,
};
@group(0) @binding(0) var<uniform> domain: EffectDomain;
@group(0) @binding(2) var input0: texture_2d<f32>;
@group(0) @binding(3) var input1: texture_2d<f32>;
@group(0) @binding(4) var input_sampler: sampler;
fn input_uv(p: vec2<f32>) -> vec2<f32> {
    let h = vec3<f32>(p, 1.0);
    let w = dot(domain.surface2.xyz, h);
    return (vec2<f32>(dot(domain.surface0.xyz, h), dot(domain.surface1.xyz, h)) / w - domain.rect.xy) / domain.rect.zw;
}
fn visible_local(p: vec2<f32>) -> bool {
    let h = vec3<f32>(p, 1.0);
    let w = dot(domain.camera_w.xyz, h);
    let z = dot(domain.camera_z.xyz, h);
    return dot(domain.surface2.xyz, h) > 0.0 && w > 0.0 &&
        (domain.camera_z.w == 0.0 || (z >= 0.0 && z <= w));
}
fn sample_input0(p: vec2<f32>) -> vec4<f32> {
    let uv = input_uv(p);
    let value = textureSampleLevel(input0, input_sampler, storage_uv(uv, textureDimensions(input0)), 0.0);
    return select(vec4<f32>(0.0), value, visible_local(p) && all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));
}
fn sample_input1(p: vec2<f32>) -> vec4<f32> {
    let uv = input_uv(p);
    let value = textureSampleLevel(input1, input_sampler, storage_uv(uv, textureDimensions(input1)), 0.0);
    return select(vec4<f32>(0.0), value, visible_local(p) && all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));
}
fn active_pixels(dimensions: vec2<u32>) -> vec2<f32> {
    return ceil(domain.storage.xy * vec2<f32>(dimensions) / domain.storage.zw);
}
fn storage_uv(uv: vec2<f32>, dimensions: vec2<u32>) -> vec2<f32> {
    // Clamp at the active image edge, never interpolate into spare/stale texels.
    return clamp(uv * active_pixels(dimensions), vec2<f32>(0.5), active_pixels(dimensions) - 0.5) / vec2<f32>(dimensions);
}
struct EffectVertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> EffectVertex {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return EffectVertex(vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0), uv);
}
var<private> pax_effect_position: vec2<f32>;
@fragment fn fs_main(v: EffectVertex) -> @location(0) vec4<f32> {
    let h = vec3<f32>(domain.rect.xy + v.uv * domain.rect.zw, 1.0);
    let w = dot(domain.local2.xyz, h);
    if w <= 0.0 { return vec4<f32>(0.0); }
    let p = vec2<f32>(dot(domain.local0.xyz, h), dot(domain.local1.xyz, h)) / w;
    if !visible_local(p) { return vec4<f32>(0.0); }
    pax_effect_position = p;
    return effect(p);
}
// Local-axis distance corresponding to one texel of input0, including scale/rotation and DPR.
fn input0_pixel_size() -> vec2<f32> {
    let density = active_pixels(textureDimensions(input0)) / domain.rect.zw;
    let h = vec3<f32>(pax_effect_position, 1.0);
    let w = dot(domain.surface2.xyz, h);
    let xy = vec2<f32>(dot(domain.surface0.xyz, h), dot(domain.surface1.xyz, h));
    let dx = (vec2<f32>(domain.surface0.x, domain.surface1.x) * w - xy * domain.surface2.x) / (w * w);
    let dy = (vec2<f32>(domain.surface0.y, domain.surface1.y) * w - xy * domain.surface2.y) / (w * w);
    return 1.0 / vec2<f32>(length(dx * density), length(dy * density));
}
