struct EffectDomain {
    rect: vec4<f32>,
    local0: vec4<f32>,
    local1: vec4<f32>,
    surface0: vec4<f32>,
    surface1: vec4<f32>,
};
@group(0) @binding(0) var<uniform> domain: EffectDomain;
@group(0) @binding(2) var input0: texture_2d<f32>;
@group(0) @binding(3) var input1: texture_2d<f32>;
@group(0) @binding(4) var input_sampler: sampler;
fn input_uv(p: vec2<f32>) -> vec2<f32> {
    let h = vec3<f32>(p, 1.0);
    return (vec2<f32>(dot(domain.surface0.xyz, h), dot(domain.surface1.xyz, h)) - domain.rect.xy) / domain.rect.zw;
}
fn sample_input0(p: vec2<f32>) -> vec4<f32> {
    let uv = input_uv(p);
    let value = textureSampleLevel(input0, input_sampler, uv, 0.0);
    return select(vec4<f32>(0.0), value, all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));
}
fn sample_input1(p: vec2<f32>) -> vec4<f32> {
    let uv = input_uv(p);
    let value = textureSampleLevel(input1, input_sampler, uv, 0.0);
    return select(vec4<f32>(0.0), value, all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));
}
struct EffectVertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> EffectVertex {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return EffectVertex(vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0), uv);
}
@fragment fn fs_main(v: EffectVertex) -> @location(0) vec4<f32> {
    let h = vec3<f32>(domain.rect.xy + v.uv * domain.rect.zw, 1.0);
    return effect(vec2<f32>(dot(domain.local0.xyz, h), dot(domain.local1.xyz, h)));
}
// Local-axis distance corresponding to one texel of input0, including scale/rotation and DPR.
fn input0_pixel_size() -> vec2<f32> {
    let density = vec2<f32>(textureDimensions(input0)) / domain.rect.zw;
    return vec2<f32>(1.0 / length(vec2<f32>(domain.surface0.x, domain.surface1.x) * density),
        1.0 / length(vec2<f32>(domain.surface0.y, domain.surface1.y) * density));
}
