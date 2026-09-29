struct Control {
    position: vec4<f32>,
    color: vec4<f32>,
};
struct Patch {
    controls: array<Control, 16>,
};
struct Domain {
    origin: vec2<f32>,
    size: vec2<f32>,
    subdivision: u32,
    _padding0: u32,
    _padding1: u32,
    _padding2: u32,
};
@group(0) @binding(0) var<storage, read> patches: array<Patch>;
@group(0) @binding(1) var<uniform> domain: Domain;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) premultiplied: vec4<f32>,
};

fn bernstein(t: f32) -> vec4<f32> {
    let s = 1.0 - t;
    return vec4<f32>(s*s*s, 3.0*s*s*t, 3.0*s*t*t, t*t*t);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) patch_index: u32) -> Vertex {
    // All patches reuse indexed parameter geometry, including the same row-major diagonal.
    let side = domain.subdivision + 1u;
    let grid = vec2<u32>(vertex % side, vertex / side);
    let uv = vec2<f32>(grid) / f32(domain.subdivision);
    let bu = bernstein(uv.x);
    let bv = bernstein(uv.y);
    var point = vec2<f32>(0.0);
    var color = vec4<f32>(0.0);
    for (var y = 0u; y < 4u; y += 1u) {
        for (var x = 0u; x < 4u; x += 1u) {
            let control = patches[patch_index].controls[y * 4u + x];
            let weight = bu[x] * bv[y];
            point += control.position.xy * weight;
            color += vec4<f32>(control.position.zw, control.color.xy) * weight;
        }
    }
    let unit = (point - domain.origin) / domain.size;
    var result: Vertex;
    result.position = vec4<f32>(unit.x * 2.0 - 1.0, 1.0 - unit.y * 2.0, 0.0, 1.0);
    result.premultiplied = color;
    return result;
}

@fragment
fn fs_main(vertex: Vertex) -> @location(0) vec4<f32> {
    let alpha = clamp(vertex.premultiplied.a, 0.0, 1.0);
    return vec4<f32>(clamp(vertex.premultiplied.rgb, vec3<f32>(0.0), vec3<f32>(alpha)), alpha);
}
