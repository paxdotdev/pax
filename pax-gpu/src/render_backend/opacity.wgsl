@group(0) @binding(0) var content: texture_2d<f32>;
@group(0) @binding(1) var content_sampler: sampler;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
};
@vertex fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) opacity: f32) -> Output {
    return Output(vec4<f32>(position, 0.0, 1.0), uv, opacity);
}
@fragment fn fs_main(input: Output) -> @location(0) vec4<f32> {
    // The render attachment already contains premultiplied color in the backend's blending
    // color space. Scale color and alpha together; applying straight-alpha blending again
    // would multiply source alpha twice, especially at antialiased edges.
    return textureSample(content, content_sampler, input.uv) * input.opacity;
}
