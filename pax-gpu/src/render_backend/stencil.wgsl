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
    @location(0) camera_clip: vec2<f32>,
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
    let h = projected_position(stencil_transform, position, vec2<f32>(t_p_x, t_p_y));
    let xy = (h.xy - globals.origin * h.w) / globals.resolution * 2.0 - vec2<f32>(h.w);
    out.clip_position = vec4<f32>(xy.x, -xy.y, 0.0, h.w);
    out.camera_clip = projected_camera_clip(stencil_transform, position);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.camera_clip.y <= 0.0 || in.camera_clip.x < 0.0 || in.camera_clip.x > in.camera_clip.y { discard; }
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}                    
