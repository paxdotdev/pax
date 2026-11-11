struct Globals {
    resolution: vec2<f32>,
    dpr: vec2<f32>,
    origin: vec2<f32>,
    padding: vec2<f32>,
};

struct Primitive {
    fill_id_and_type: u32,
    z_index: i32,
    material_id: u32,
    transform_id: u32,
    draw_range: vec4<f32>,
    light_mask: u32,
    paint_count: u32,
    _padding_1: u32,
    _padding_2: u32,
};

struct Primitives {
    primitives: array<Primitive, VECTOR_PRIMITIVES_PER_BANK>,
};


struct Colors {
    colors: array<vec4<f32>, VECTOR_TABLE_CAPACITY>,
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
    weight: f32,
    _pad0: u32,
    focal_point: vec2<f32>,
    _pad: array<vec4<u32>, 3>,
}

struct Gradients {
    gradients: array<Gradient>,
}

struct Materials {
    materials: array<Material, VECTOR_MATERIALS_PER_BANK>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> u_primitives: Primitives;
@group(0) @binding(2) var<uniform> u_primitives_upper: Primitives;
@group(3) @binding(0) var<uniform> transforms: Transforms;
@group(0) @binding(3) var<uniform> colors: Colors;
@group(0) @binding(4) var<storage, read> gradients: Gradients;
@group(0) @binding(5) var<uniform> materials: Materials;
@group(0) @binding(6) var<uniform> scene_lighting: SceneLighting;
@group(0) @binding(7) var<uniform> materials_upper: Materials;

fn scene_material(index: u32) -> Material {
    if index < VECTOR_MATERIALS_PER_BANK { return materials.materials[index]; }
    return materials_upper.materials[index - VECTOR_MATERIALS_PER_BANK];
}
@group(1) @binding(0) var alpha_mask: texture_2d<f32>;
@group(1) @binding(1) var alpha_sampler: sampler;
@group(1) @binding(2) var<uniform> alpha_domain: vec4<f32>;
@group(2) @binding(0) var mesh_paints: texture_2d_array<f32>;
@group(2) @binding(1) var mesh_sampler: sampler;

struct GpuVertex {
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) prim_id: u32,
    @location(3) path_progress: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) @interpolate(flat) prim_id: u32,
    @location(1) world_position: vec3<f32>,
    @location(2) path_progress: f32,
    @location(3) local_position: vec2<f32>,
    @location(4) camera_clip: vec2<f32>,
};

@vertex
fn vs_main(
    model: GpuVertex,
) -> VertexOutput {
	var out: VertexOutput;
    var p = model.position;

    // apply transform
    let primitive = scene_primitive(model.prim_id);
    let m = scene_transform(primitive.transform_id);

    let t_p_x = p.x * m.xx + p.y * m.yx + m.zx;
    let t_p_y = p.x * m.xy + p.y * m.yy + m.zy;

    let affine = vec2<f32>(t_p_x, t_p_y);
    let h = projected_position(m, p, affine);
    let xy = (h.xy - globals.origin * h.w) / globals.resolution * 2.0 - vec2<f32>(h.w);
    out.prim_id = model.prim_id;
    out.world_position = lighting_position(m, p, affine, scene_lighting.surface_to_world_x.xyz, scene_lighting.surface_to_world_y.xyz);
    out.path_progress = model.path_progress;
    out.local_position = p;
    out.camera_clip = projected_camera_clip(m, p);
    out.clip_position = vec4<f32>(xy.x, -xy.y, 0.0, h.w);
    return out;
}

fn scene_primitive(id: u32) -> Primitive {
    if id < VECTOR_PRIMITIVES_PER_BANK {
        return u_primitives.primitives[id];
    }
    return u_primitives_upper.primitives[id - VECTOR_PRIMITIVES_PER_BANK];
}

// Identity is shared by every page and never aliases an allocated scene transform.
fn scene_transform(id: u32) -> Transform {
    if id == 0u {
        return identity_transform();
    }
    return transforms.transforms[id % TRANSFORMS_PER_PAGE];
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let grain_size = max(0.7, max(length(dpdx(in.local_position)), length(dpdy(in.local_position))));

    let primitive = scene_primitive(in.prim_id);
    if primitive.draw_range.z > 0.5 {
        let draw_start = clamp(primitive.draw_range.x, 0.0, 1.0);
        let draw_end = clamp(primitive.draw_range.y, 0.0, 1.0);
        if draw_start >= draw_end || in.path_progress < draw_start || in.path_progress > draw_end {
            discard;
        }
    }
    if in.camera_clip.y <= 0.0 || in.camera_clip.x < 0.0 || in.camera_clip.x > in.camera_clip.y { discard; }
    //color/gradient
    let fill_id_and_type = primitive.fill_id_and_type;
    //clipping rectangle
    let fill_id = fill_id_and_type & 0xFFFFu;
    let fill_type = fill_id_and_type >> 16u;
    var color: vec4<f32>;
    var paint_position = in.clip_position.xy + globals.origin * globals.dpr;
    if projection_enabled(scene_transform(primitive.transform_id)) {
        paint_position = in.local_position * globals.dpr;
    }
    if fill_type == 0u {
        color = colors.colors[fill_id];
    } else if fill_type == 1u {
        color = gradient(fill_id, paint_position);
    } else {
        var premultiplied = vec4<f32>(0.0);
        for (var i = 0u; i < primitive.paint_count; i++) {
            let sample = gradient(fill_id + i, paint_position);
            let weight = gradients.gradients[fill_id + i].weight;
            premultiplied += vec4<f32>(sample.rgb * sample.a, sample.a) * weight;
        }
        color = vec4<f32>(0.0);
        if premultiplied.a > 0.0 {
            color = vec4<f32>(premultiplied.rgb / premultiplied.a, premultiplied.a);
        }
    }
    color.a *= scene_transform(primitive.transform_id).opacity;
    let bounded_mask = all(alpha_domain.zw > vec2<f32>(0.0));
    let mask_uv = (in.clip_position.xy / globals.dpr + globals.origin - alpha_domain.xy) /
        select(vec2<f32>(1.0), alpha_domain.zw, bounded_mask);
    let mask_alpha = textureSampleLevel(alpha_mask, alpha_sampler, mask_uv, 0.0).r;
    color.a *= select(0.0, mask_alpha, !bounded_mask ||
        (all(mask_uv >= vec2<f32>(0.0)) && all(mask_uv <= vec2<f32>(1.0))));
    color = apply_lighting(
        color,
        scene_material(primitive.material_id),
        in.world_position,
        lighting_normal(scene_transform(primitive.transform_id)),
        primitive.light_mask,
        vec3<f32>(in.local_position / grain_size, 0.0),
    );
    return color;
}

fn gradient(fill_id: u32, coord: vec2<f32>) -> vec4<f32> {
    let gradient = gradients.gradients[fill_id];
    if gradient.type_id == 2u { return gradient.colors[0]; }
    if gradient.type_id == 3u {
        let delta = coord / globals.dpr - gradient.position;
        let a = gradient.main_axis;
        let b = gradient.off_axis;
        let determinant = a.x*b.y - a.y*b.x;
        if abs(determinant) < 0.000001 { return vec4<f32>(0.0); }
        let local = vec2<f32>(delta.x*b.y - delta.y*b.x, a.x*delta.y - a.y*delta.x) / determinant;
        let domain = gradient.colors[0];
        let uv = (local - domain.xy) / domain.zw;
        if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) { return vec4<f32>(0.0); }
        let value = textureSampleLevel(mesh_paints, mesh_sampler, uv, i32(gradient._pad0), 0.0);
        if value.a <= 0.0 { return vec4<f32>(0.0); }
        return vec4<f32>(value.rgb / value.a, value.a);
    }
    if gradient.stop_count == 0u { return vec4<f32>(0.0); }
    
    // Calculate color space position
    let g_p = gradient.position * globals.dpr;
    let g_a = gradient.main_axis * globals.dpr;
    let p_t = coord - g_p;
    let m_a_l = max(length(g_a), 0.0001);
    let n = g_a / m_a_l;
    let stop_scale = m_a_l / max(length(gradient.main_axis), 0.0001);
    var color_space = dot(p_t, n) / stop_scale;
    let delta = coord / globals.dpr - gradient.position;
    let axis = gradient.main_axis;
    let off = gradient.off_axis;
    let determinant = axis.x * off.y - axis.y * off.x;
    if gradient.type_id == 0u && abs(determinant) > 0.0001 {
        color_space = (delta.x * off.y - delta.y * off.x) / determinant * length(axis);
    }
    if gradient.type_id == 1u {
        color_space = radial_coordinate(coord / globals.dpr - gradient.position,
            gradient.main_axis, gradient.off_axis, gradient.focal_point);
        if color_space < 0.0 { return vec4<f32>(0.0); }
    }

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
