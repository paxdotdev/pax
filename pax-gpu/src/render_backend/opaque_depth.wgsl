struct PlanarDepth {
    homogeneous: vec4<f32>,
    sample_plane: vec4<f32>,
};
@group(0) @binding(8) var<uniform> depth_plane: PlanarDepth;
@group(0) @binding(9) var<uniform> sample_locations: array<vec4<f32>, 4>;

fn vector_depth(h: vec4<f32>) -> f32 {
    return dot(depth_plane.homogeneous, vec4<f32>(h.xy, h.w, h.z));
}

struct DepthOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
    @builtin(sample_mask) mask: u32,
};

@fragment
fn fs_main(in: VertexOutput, @builtin(sample_index) sample: u32) -> DepthOutput {
    var out: DepthOutput;
    out.color = shade_vector(in);
    // All members of a coplanar class evaluate the same equation at the same
    // sample. Interpolated vertex depth can differ with each decal's transform
    // or triangulation and defeat LessEqual's authored tie order.
    let local = (floor(in.clip_position.xy) + sample_locations[sample].xy) / globals.dpr;
    out.depth = clamp(dot(depth_plane.sample_plane.xyz, vec3<f32>(local, 1.0)), 0.0, 1.0);
    // Force sample-frequency evaluation so MSAA intersections retain geometric
    // depth at each sample instead of using one pixel-center depth for all four.
    out.mask = 1u << sample;
    return out;
}
