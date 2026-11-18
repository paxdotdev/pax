@group(0) @binding(0) var<storage, read_write> locations: array<vec4<f32>, 4>;
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(linear, sample) location: vec2<f32>,
};
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Output {
    let point = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0))[index];
    var out: Output;
    out.position = vec4<f32>(point.x, -point.y, 0.0, 1.0);
    out.location = (point + vec2<f32>(1.0)) * 0.5;
    return out;
}
@fragment fn fs_main(in: Output, @builtin(sample_index) sample: u32) -> @location(0) vec4<f32> {
    locations[sample] = vec4<f32>(in.location, 0.0, 0.0);
    return vec4<f32>(0.0);
}
