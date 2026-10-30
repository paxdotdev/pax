struct Globals {
    resolution: vec2<f32>,
    dpr: vec2<f32>,
    origin: vec2<f32>,
    padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> stencil_transforms: Transforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) transform_id: u32,
) -> VertexOutput {
	var out: VertexOutput;
    let stencil_transform = stencil_transforms.transforms[transform_id % TRANSFORMS_PER_PAGE];
    let t_p_x = position.x * stencil_transform.xx + position.y * stencil_transform.yx + stencil_transform.zx;
    let t_p_y = position.x * stencil_transform.xy + position.y * stencil_transform.yy + stencil_transform.zy;
    var pos = vec2<f32>(t_p_x, t_p_y);
    pos = (pos - globals.origin) / globals.resolution;
    pos *= 2.0;
    pos -= 1.0;
    pos.y *= -1.0;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}                    
