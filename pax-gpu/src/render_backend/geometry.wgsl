struct Globals {
    resolution: vec2<f32>,
    dpr: vec2<f32>,
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
    transforms: array<Transform, 480>,
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
    weight: f32,
    _pad0: u32,
    focal_point: vec2<f32>,
    _pad: array<vec4<u32>, 3>,
}

struct Gradients {
    gradients: array<Gradient>,
}

struct Material {
    coefficients: vec4<f32>,
    emissive: vec4<f32>,
}

struct Materials {
    materials: array<Material, 512>,
}

struct SceneLight {
    position: vec4<f32>,
    direction: vec4<f32>,
    color: vec4<f32>,
    params: vec4<f32>,
}

struct SceneLighting {
    ambient: vec4<f32>,
    flags: vec4<u32>,
    lights: array<SceneLight, 8>,
}


@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> u_primitives: Primitives;
@group(0) @binding(2) var<uniform> transforms: Transforms;
@group(0) @binding(3) var<uniform> colors: Colors;
@group(0) @binding(4) var<storage, read> gradients: Gradients;
@group(0) @binding(5) var<uniform> materials: Materials;
@group(0) @binding(6) var<uniform> scene_lighting: SceneLighting;
@group(1) @binding(0) var alpha_mask: texture_2d<f32>;
@group(1) @binding(1) var alpha_sampler: sampler;

struct GpuVertex {
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) prim_id: u32,
    @location(3) path_progress: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
	@location(0) @interpolate(flat) prim_id: u32,
    @location(1) world_position: vec2<f32>,
    @location(2) path_progress: f32,
    @location(3) local_position: vec2<f32>,
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
    out.world_position = vec2<f32>(t_p_x, t_p_y);
    out.path_progress = model.path_progress;
    out.local_position = p;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    return out;
}

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let primitive = u_primitives.primitives[in.prim_id];
    if primitive.draw_range.z > 0.5 {
        let draw_start = clamp(primitive.draw_range.x, 0.0, 1.0);
        let draw_end = clamp(primitive.draw_range.y, 0.0, 1.0);
        if draw_start >= draw_end || in.path_progress < draw_start || in.path_progress > draw_end {
            discard;
        }
    }
    //color/gradient
    let fill_id_and_type = primitive.fill_id_and_type;
    //clipping rectangle
    let fill_id = fill_id_and_type & 0xFFFFu;
    let fill_type = fill_id_and_type >> 16u;
    var color: vec4<f32>;
    if fill_type == 0u {
        color = colors.colors[fill_id];
    } else if fill_type == 1u {
        color = gradient(fill_id, in.clip_position.xy);
    } else {
        var premultiplied = vec4<f32>(0.0);
        for (var i = 0u; i < primitive.paint_count; i++) {
            let sample = gradient(fill_id + i, in.clip_position.xy);
            let weight = gradients.gradients[fill_id + i].weight;
            premultiplied += vec4<f32>(sample.rgb * sample.a, sample.a) * weight;
        }
        color = vec4<f32>(0.0);
        if premultiplied.a > 0.0 {
            color = vec4<f32>(premultiplied.rgb / premultiplied.a, premultiplied.a);
        }
    }
    color.a *= transforms.transforms[primitive.transform_id].opacity;
    color.a *= textureSampleLevel(alpha_mask, alpha_sampler,
        in.clip_position.xy / (globals.resolution * globals.dpr), 0.0).r;
    color = apply_lighting(
        color,
        materials.materials[primitive.material_id],
        in.world_position,
        primitive.light_mask,
    );
    return color;
}

fn apply_lighting(
    color: vec4<f32>,
    material: Material,
    world_position: vec2<f32>,
    light_mask: u32,
) -> vec4<f32> {
    if scene_lighting.flags.x == 0u {
        return color;
    }

    if material.coefficients.x < 0.0 {
        return color;
    }

    let ambient_coeff = material.coefficients.x;
    let diffuse_coeff = material.coefficients.y;
    let specular_coeff = material.coefficients.z;
    let roughness = clamp(material.coefficients.w, 0.0, 1.0);
    let light_count = min(scene_lighting.flags.y, 8u);
    let active_light_mask = (1u << light_count) - 1u;
    let eligible_light_mask = light_mask & active_light_mask;
    if eligible_light_mask == 0u && scene_lighting.flags.z == 0u {
        return color;
    }
    let surface_position = vec3<f32>(world_position, 0.0);
    let normal = vec3<f32>(0.0, 0.0, 1.0);
    let view_dir = vec3<f32>(0.0, 0.0, 1.0);
    var lighting = scene_lighting.ambient.rgb * scene_lighting.ambient.a * ambient_coeff;

    for (var i = 0u; i < 8u; i++) {
        if i >= light_count {
            break;
        }
        if (eligible_light_mask & (1u << i)) == 0u {
            continue;
        }

        let light = scene_lighting.lights[i];
        var light_dir: vec3<f32>;
        var attenuation = 1.0;
        if light.params.x < 0.5 {
            let to_light = light.position.xyz - surface_position;
            let distance = max(length(to_light), 0.0001);
            let radius = max(light.params.y, 0.0001);
            let falloff = max(1.0 - distance / radius, 0.0);
            attenuation = falloff * falloff;
            light_dir = to_light / distance;
        } else {
            let direction_len = max(length(light.direction.xyz), 0.0001);
            light_dir = -light.direction.xyz / direction_len;
        }

        let diffuse = max(dot(normal, light_dir), 0.0);
        let half_dir = normalize(light_dir + view_dir);
        let shininess = mix(96.0, 8.0, roughness);
        let specular = pow(max(dot(normal, half_dir), 0.0), shininess);
        lighting += light.color.rgb * light.color.a * attenuation *
            ((diffuse_coeff * diffuse) + (specular_coeff * specular));
    }

    let lit_rgb = color.rgb * clamp(lighting, vec3<f32>(0.0), vec3<f32>(8.0))
        + material.emissive.rgb * material.emissive.a;
    return vec4<f32>(lit_rgb, color.a);
}


fn gradient(fill_id: u32, coord: vec2<f32>) -> vec4<f32> {
    let gradient = gradients.gradients[fill_id];
    if gradient.type_id == 2u { return gradient.colors[0]; }
    if gradient.stop_count == 0u { return vec4<f32>(0.0); }
    
    // Calculate color space position
    let g_p = gradient.position * globals.dpr;
    let g_a = gradient.main_axis * globals.dpr;
    let p_t = coord - g_p;
    let m_a_l = max(length(g_a), 0.0001);
    let n = g_a / m_a_l;
    let stop_scale = m_a_l / max(length(gradient.main_axis), 0.0001);
    var color_space = dot(p_t, n) / stop_scale;
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
