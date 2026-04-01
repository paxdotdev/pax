struct Globals {
    resolution: vec2<f32>,
    dpr: f32,
    _pad2: f32,
};

struct StencilTransform {
    xx: f32,
    xy: f32,
    yx: f32,
    yy: f32,
    zx: f32,
    zy: f32,
    opacity: f32,
    _pad1: f32,
};

struct StencilTransforms {
    transforms: array<StencilTransform, 1024>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> stencil_transforms: StencilTransforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) transform_id: u32,
) -> VertexOutput {
	var out: VertexOutput;
    let stencil_transform = stencil_transforms.transforms[transform_id];
    let t_p_x = position.x * stencil_transform.xx + position.y * stencil_transform.yx + stencil_transform.zx;
    let t_p_y = position.x * stencil_transform.xy + position.y * stencil_transform.yy + stencil_transform.zy;
    var pos = vec2<f32>(t_p_x, t_p_y);
    pos /= globals.resolution;
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
