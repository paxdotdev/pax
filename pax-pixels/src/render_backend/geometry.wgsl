struct Globals {
    resolution: vec2<f32>,
    dpr: u32,
    _pad2: u32,
};

struct Primitive {
    fill_id_and_type: u32,
    z_index: i32,
    clipping_id: u32,
    transform_id: u32, //not used atm
};

struct Primitives {
    primitives: array<Primitive, 512>,
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

struct Transforms {
    transforms: array<Transform, 64>,
}


struct Colors {
    colors: array<vec4<f32>, 512>,
}

struct Gradient {
    colors: array<vec4<f32>, 8>,
    stops_set1: vec4<f32>,
    stops_set2: vec4<f32>,
    position: vec2<f32>,
    main_axis: vec2<f32>,
    off_axis: vec2<f32>,
    stop_count: u32,
    type_id: u32,
    _pad: array<vec4<u32>, 4>,
}

struct Gradients {
    gradients: array<Gradient, 64>,
}


@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> u_primitives: Primitives;
@group(0) @binding(2) var<uniform> clipping: Transforms;
@group(0) @binding(3) var<uniform> colors: Colors;
@group(0) @binding(4) var<uniform> gradients: Gradients;

struct GpuVertex {
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) prim_id: u32, 
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) @interpolate(flat) prim_id: u32,
};

@vertex
fn vs_main(
    model: GpuVertex,
) -> VertexOutput {
	var out: VertexOutput;
    var pos = model.position;
    pos /= globals.resolution;
    pos *= 2.0;
    pos -= 1.0;
    pos.y *= -1.0;

    out.prim_id = model.prim_id;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    return out;
}
// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let primitive = u_primitives.primitives[in.prim_id];

    //clipping rectangle
    let p = in.clip_position.xy;
    let m = clipping.transforms[primitive.clipping_id];
    let t_p_x = p.x * m.xx + p.y * m.yx + m.zx;
    let t_p_y = p.x * m.xy + p.y * m.yy + m.zy;
    if t_p_x > 1.0 || t_p_x < 0.0 || t_p_y > 1.0 || t_p_y < 0.0 {
        discard;
    }



    //color/gradient
    let fill_id_and_type = primitive.fill_id_and_type;
    //clipping rectangle
    let fill_id = fill_id_and_type & 0xFFFFu;
    let fill_type = fill_id_and_type >> 16u;
    var color: vec4<f32>;
    if fill_type == 0u {
        color = colors.colors[fill_id];
    } else {
        let p = in.clip_position.xy;
        color = gradient(fill_id, p);
    }
    return color;
}



fn gradient(fill_id: u32, coord: vec2<f32>) -> vec4<f32> {

    let gradient = gradients.gradients[fill_id];

    //color space for linear gradient:
    let g_p = gradient.position*f32(globals.dpr);
    let g_a = gradient.main_axis*f32(globals.dpr);
    let p_t = coord - g_p;
    let m_a_l = length(g_a);
    let n = g_a/m_a_l;
    let t_m = dot(p_t, n); //this is the ammount that p_t points in the direction of n
    let color_space = t_m/m_a_l; //This is 0.0..=1.0 and can be mapped to color space with coords

    //figure out surrounding stops in color space:
    let s1 = gradient.stops_set1;
    let s2 = gradient.stops_set2;
    let stops = array<f32, 8>(s1[0], s1[1], s1[2], s1[3], s2[0], s2[1], s2[2], s2[3]);
    let colors = gradient.colors;

    //assumed to be 2 or larger
    let len = gradient.stop_count;
    var color: vec4<f32>;


    // This is horrible, but can't use dynamic indicies in loops in wgsl. One
    // possible path forward would be to create 1d textures to sample from
    // instead.
    var left_stop: f32;
    var right_stop: f32;
    var left_col: vec4<f32>;
    var right_col: vec4<f32>;
    if stops[0] > color_space { 
        left_stop = stops[0] - 1.0;
        right_stop = stops[0];
        left_col = colors[0];
        right_col = colors[0];
    } else if stops[1] > color_space || len <= 1u {
        left_stop = stops[0];
        right_stop = stops[1];
        left_col = colors[0];
        right_col = colors[1];
    } else if stops[2] > color_space || len <= 2u {
        left_stop = stops[1];
        right_stop = stops[2];
        left_col = colors[1];
        right_col = colors[2];
    } else if stops[3] > color_space || len <= 3u {
        left_stop = stops[2];
        right_stop = stops[3];
        left_col = colors[2];
        right_col = colors[3];
    } else if stops[4] > color_space || len <= 4u {
        left_stop = stops[3];
        right_stop = stops[4];
        left_col = colors[3];
        right_col = colors[4];
    } else if stops[5] > color_space || len <= 5u {
        left_stop = stops[4];
        right_stop = stops[5];
        left_col = colors[4];
        right_col = colors[5];
    } else if stops[6] > color_space || len <= 6u {
        left_stop = stops[5];
        right_stop = stops[6];
        left_col = colors[5];
        right_col = colors[6];
    } else if stops[7] > color_space || len <= 7u {
        left_stop = stops[6];
        right_stop = stops[7];
        left_col = colors[6];
        right_col = colors[7];
    } else {
        left_stop = stops[7] - 1.0;
        right_stop = stops[7];
        left_col = colors[7];
        right_col = colors[7];
    }
    let space = (color_space - left_stop)/(right_stop - left_stop);
    color = left_col*(1.0 - space) + right_col*space;
    return color;
}