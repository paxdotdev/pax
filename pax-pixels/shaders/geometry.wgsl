struct Globals {
    resolution: vec2<f32>,
    dpr: u32,
    _pad2: u32,
};

struct Transform {
    // OBS transform: mat3x2<f32> has different alignment on WebGL vs for
    // example metal (don't change this unless you know what you are doing)
    xx: f32,
    xy: f32,
    yx: f32,
    yy: f32,
    zx: f32,
    zy: f32,
    _pad1: u32,
    _pad2: u32,
}

// TODO change type to support radius/solid etc
struct MeshMetadata {
    colors: array<vec4<f32>, 8>,
    stops_set1: vec4<f32>,
    stops_set2: vec4<f32>,
    position: vec2<f32>,
    main_axis: vec2<f32>,
    off_axis: vec2<f32>,
    stop_count: u32,
    type_id: u32,
    _pad0: array<vec4<u32>, 4>,
}


@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> transform: Transform;

@group(1) @binding(0) var<uniform> mesh_metadata: MeshMetadata;

struct GpuVertex {
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) prim_id: u32, 
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    model: GpuVertex,
) -> VertexOutput {
	var out: VertexOutput;
    var p = model.position;
    // apply transform
    let m = transform;
    let t_p_x = p.x * m.xx + p.y * m.yx + m.zx;
    let t_p_y = p.x * m.xy + p.y * m.yy + m.zy;
    var pos = vec2<f32>(t_p_x, t_p_y);
    pos /= globals.resolution;
    pos *= 2.0;
    pos -= 1.0;
    pos.y *= -1.0;

    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    //color/gradient
    let type_id = mesh_metadata.type_id;
    var color: vec4<f32>;
    if type_id == 0u {
        color = mesh_metadata.colors[0];
    } else {
        let p = in.clip_position.xy;
        color = gradient(p);
    }
    return color;
}


fn gradient(coord: vec2<f32>) -> vec4<f32> {
    let gradient = mesh_metadata;
    // Calculate color space position
    let g_p = gradient.position * f32(globals.dpr);
    let g_a = gradient.main_axis * f32(globals.dpr);
    let p_t = coord - g_p;

    // TODO check type id here, and calculate color_space (0.0 to 1.0) using
    // distance instead of projection to do radial gradients!
    let m_a_l = length(g_a);
    let n = g_a / m_a_l;
    let color_space = dot(p_t, n);

    let s1 = gradient.stops_set1;
    let s2 = gradient.stops_set2;
    let stops = array<f32, 8>(s1[0], s1[1], s1[2], s1[3], s2[0], s2[1], s2[2], s2[3]);
    
    // Find the appropriate stop segment
    var left_idx = 0u;
    var right_idx = 1u;
    
    // Handle edge cases first
    if color_space <= stops[0] {
        return gradient.colors[0];
    }
    if color_space >= stops[gradient.stop_count - 1u] {
        return gradient.colors[gradient.stop_count - 1u];
    }

    // Find the segment using a fixed loop
    for (var i = 0u; i < 7u; i++) {
        if i >= gradient.stop_count - 1u { break; }
        if stops[i + 1u] > color_space {
            left_idx = i;
            right_idx = i + 1u;
            break;
        }
    }

    let left_stop = stops[left_idx];
    let right_stop = stops[right_idx];
    let left_col = gradient.colors[left_idx];
    let right_col = gradient.colors[right_idx];
    
    let t = (color_space - left_stop) / (right_stop - left_stop);
    return mix(left_col, right_col, t);
}

