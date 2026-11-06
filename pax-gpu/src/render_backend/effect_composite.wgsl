struct Globals { resolution: vec2<f32>, dpr: vec2<f32>, origin: vec2<f32>, padding: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
struct OutputRect { rect: vec4<f32>, uv_scale: vec4<f32> };
@group(0) @binding(1) var<uniform> output: OutputRect;
@group(0) @binding(2) var output_texture: texture_2d<f32>;
@group(0) @binding(3) var output_sampler: sampler;
@group(1) @binding(0) var alpha_mask: texture_2d<f32>;
@group(1) @binding(1) var alpha_sampler: sampler;
@group(1) @binding(2) var<uniform> alpha_domain: vec4<f32>;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> Vertex {
    let coords = array<vec2<f32>, 6>(vec2<f32>(0,0), vec2<f32>(0,1), vec2<f32>(1,0), vec2<f32>(1,0), vec2<f32>(0,1), vec2<f32>(1,1));
    let uv = coords[i];
    let pos = (output.rect.xy + uv * output.rect.zw - globals.origin) / globals.resolution;
    return Vertex(vec4<f32>(pos * vec2<f32>(2,-2) + vec2<f32>(-1,1), 0, 1), uv);
}
@fragment fn fs_main(v: Vertex) -> @location(0) vec4<f32> {
    let bounded_mask = all(alpha_domain.zw > vec2<f32>(0.0));
    let mask_uv = (v.position.xy / globals.dpr + globals.origin - alpha_domain.xy) /
        select(vec2<f32>(1.0), alpha_domain.zw, bounded_mask);
    let mask_alpha = textureSampleLevel(alpha_mask, alpha_sampler, mask_uv, 0.0).r;
    let alpha = select(0.0, mask_alpha, !bounded_mask ||
        (all(mask_uv >= vec2<f32>(0.0)) && all(mask_uv <= vec2<f32>(1.0))));
    let size = vec2<f32>(textureDimensions(output_texture));
    let uv = clamp(v.uv * output.uv_scale.xy, 0.5 / size, output.uv_scale.xy - 0.5 / size);
    return textureSample(output_texture, output_sampler, uv) * alpha;
}
