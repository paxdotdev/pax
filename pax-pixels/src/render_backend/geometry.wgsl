struct Globals {
    resolution: vec2<f32>,
    dpr: f32,
    _pad2: f32,
};

struct Primitive {
    fill_id_and_type: u32,
    z_index: i32,
    clipping_id: u32, //not used atm
    transform_id: u32,
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
    opacity: f32,
    _pad1: f32,
}

struct Transforms {
    transforms: array<Transform, 1024>,
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
@group(0) @binding(2) var<uniform> transforms: Transforms;
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
    var p = model.position;

    // apply transform
    let primitive = u_primitives.primitives[model.prim_id];
    let m = transforms.transforms[primitive.transform_id];

    let t_p_x = p.x * m.xx + p.y * m.yx + m.zx;
    let t_p_y = p.x * m.xy + p.y * m.yy + m.zy;

    var pos = vec2<f32>(t_p_x, t_p_y);

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
    color.a *= transforms.transforms[primitive.transform_id].opacity;
    return color;
}


fn gradient(fill_id: u32, coord: vec2<f32>) -> vec4<f32> {
    let gradient = gradients.gradients[fill_id];
    
    // Calculate color space position
    let g_p = gradient.position * globals.dpr;
    let g_a = gradient.main_axis * globals.dpr;
    let p_t = coord - g_p;
    let m_a_l = length(g_a);
    let n = g_a / m_a_l;
    let color_space = dot(p_t, n);

    // Find the appropriate stop segment
    var left_idx = 0u;
    var right_idx = 1u;
    
    // Handle edge cases first
    if color_space <= gradient_stop(gradient, 0u) {
        return gradient_color(gradient, 0u);
    }
    if color_space >= gradient_stop(gradient, gradient.stop_count - 1u) {
        return gradient_color(gradient, gradient.stop_count - 1u);
    }

    // Find the segment using a fixed loop
    for (var i = 0u; i < 7u; i++) {
        if i >= gradient.stop_count - 1u { break; }
        if gradient_stop(gradient, i + 1u) > color_space {
            left_idx = i;
            right_idx = i + 1u;
            break;
        }
    }

    let left_stop = gradient_stop(gradient, left_idx);
    let right_stop = gradient_stop(gradient, right_idx);
    let left_col = gradient_color(gradient, left_idx);
    let right_col = gradient_color(gradient, right_idx);
    
    let t = (color_space - left_stop) / (right_stop - left_stop);
    return mix(left_col, right_col, t);
}

fn gradient_stop(gradient: Gradient, index: u32) -> f32 {
    switch index {
        case 0u: { return gradient.stops_set1[0]; }
        case 1u: { return gradient.stops_set1[1]; }
        case 2u: { return gradient.stops_set1[2]; }
        case 3u: { return gradient.stops_set1[3]; }
        case 4u: { return gradient.stops_set2[0]; }
        case 5u: { return gradient.stops_set2[1]; }
        case 6u: { return gradient.stops_set2[2]; }
        default: { return gradient.stops_set2[3]; }
    }
}

fn gradient_color(gradient: Gradient, index: u32) -> vec4<f32> {
    switch index {
        case 0u: { return gradient.colors[0]; }
        case 1u: { return gradient.colors[1]; }
        case 2u: { return gradient.colors[2]; }
        case 3u: { return gradient.colors[3]; }
        case 4u: { return gradient.colors[4]; }
        case 5u: { return gradient.colors[5]; }
        case 6u: { return gradient.colors[6]; }
        default: { return gradient.colors[7]; }
    }
}
