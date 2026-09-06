@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var parent: texture_2d<f32>;
struct Filter { axis: vec2<f32>, sigma: f32, multiply_parent: f32 };
@group(0) @binding(3) var<uniform> blur: Filter;
struct Output { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Output {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: Output;
    out.position = vec4<f32>(p * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = p;
    return out;
}
fn sample_alpha(uv: vec2<f32>) -> f32 {
    let value = textureSampleLevel(source, source_sampler, uv, 0.0).r;
    return select(0.0, value, all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0)));
}
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
    var alpha = sample_alpha(in.uv);
    if blur.sigma > 0.01 {
        var total = 0.0;
        alpha = 0.0;
        for (var i = -12; i <= 12; i++) {
            let t = f32(i) * 0.25;
            let weight = exp(-0.5 * t * t);
            alpha += sample_alpha(in.uv + blur.axis * blur.sigma * t) * weight;
            total += weight;
        }
        alpha /= total;
    }
    if blur.multiply_parent > 0.5 {
        alpha *= textureSampleLevel(parent, source_sampler, in.uv, 0.0).r;
    }
    return vec4<f32>(alpha);
}
