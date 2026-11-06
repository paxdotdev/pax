@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var parent: texture_2d<f32>;
struct Capture { crop: vec4<f32>, parent_mapping: vec4<f32> };
@group(0) @binding(3) var<uniform> capture: Capture;
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Output {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return Output(vec4<f32>(p * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0), p);
}
@fragment fn fs_main(input: Output) -> @location(0) vec4<f32> {
    // Captured RGBA is premultiplied; coverage is its alpha, independent of source RGB.
    let uv = (input.uv - capture.crop.xy) / capture.crop.zw;
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) { return vec4<f32>(0.0); }
    let parent_uv = input.uv * capture.parent_mapping.xy + capture.parent_mapping.zw;
    if any(parent_uv < vec2<f32>(0.0)) || any(parent_uv > vec2<f32>(1.0)) { return vec4<f32>(0.0); }
    return vec4<f32>(textureSampleLevel(source, source_sampler, uv, 0.0).a *
        textureSampleLevel(parent, source_sampler, parent_uv, 0.0).r);
}
