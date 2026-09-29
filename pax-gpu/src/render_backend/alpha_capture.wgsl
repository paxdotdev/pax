@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(3) var<uniform> crop: vec4<f32>;
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Output {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return Output(vec4<f32>(p * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0), p);
}
@fragment fn fs_main(input: Output) -> @location(0) vec4<f32> {
    // Captured RGBA is premultiplied; coverage is its alpha, independent of source RGB.
    let uv = (input.uv - crop.xy) / crop.zw;
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) { return vec4<f32>(0.0); }
    return vec4<f32>(textureSampleLevel(source, source_sampler, uv, 0.0).a);
}
