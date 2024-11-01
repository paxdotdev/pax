struct Globals {
    resolution: vec2<f32>,
    dpr: u32,
    _pad2: u32,
};


@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var texture: texture_2d<f32>;
@group(0)@binding(2) var texture_sampler: sampler;

struct TextureVertex {
    @location(0) position: vec2<f32>,
    @location(1) texture_coord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coord: vec2<f32>,
};

@vertex
fn vs_main(
    model: TextureVertex,
) -> VertexOutput {
	var out: VertexOutput;
    var pos = model.position;
    pos /= globals.resolution;
    pos *= 2.0;
    pos -= 1.0;
    pos.y *= -1.0;
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.texture_coord = model.texture_coord;
    return out;
}
// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = textureSample(texture, texture_sampler, in.texture_coord);
    return vec4<f32>(t.x + in.texture_coord.x/1000.0, t.y + in.texture_coord.y/1000.0, t.z, t.w);
}
