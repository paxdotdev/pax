struct Globals { resolution: vec2<f32>, dpr: vec2<f32>, origin: vec2<f32>, padding: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> rect: vec4<f32>;
@group(0) @binding(2) var output_texture: texture_2d<f32>;
@group(0) @binding(3) var output_sampler: sampler;
@group(1) @binding(0) var alpha_mask: texture_2d<f32>;
@group(1) @binding(1) var alpha_sampler: sampler;
@group(1) @binding(2) var<uniform> alpha_domain: vec4<f32>;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> Vertex {
    let coords = array<vec2<f32>, 6>(vec2<f32>(0,0), vec2<f32>(0,1), vec2<f32>(1,0), vec2<f32>(1,0), vec2<f32>(0,1), vec2<f32>(1,1));
    let uv = coords[i];
    let pos = (rect.xy + uv * rect.zw - globals.origin) / globals.resolution;
    return Vertex(vec4<f32>(pos * vec2<f32>(2,-2) + vec2<f32>(-1,1), 0, 1), uv);
}
@fragment fn fs_main(v: Vertex) -> @location(0) vec4<f32> {
    let alpha = textureSampleLevel(alpha_mask, alpha_sampler,
        (v.position.xy / globals.dpr + globals.origin - alpha_domain.xy) / alpha_domain.zw, 0.0).r;
    return textureSample(output_texture, output_sampler, v.uv) * alpha;
}
