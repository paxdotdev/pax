struct Globals {
    resolution: vec2<f32>,
    dpr: u32,
    _pad2: u32,
};


@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0)@binding(1)
var texture_sampler: sampler;

struct TextureVertex {
    @location(0) position: vec2<f32>,
    @location(1) texture_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    model: TextureVertex,
) -> VertexOutput {
	var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    return out;
}
// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
