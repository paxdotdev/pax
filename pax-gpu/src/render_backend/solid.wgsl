struct Globals {
    resolution: vec2<f32>, dpr: vec2<f32>, origin: vec2<f32>, _padding: vec2<f32>,
}
struct Solid {
    world: mat4x4<f32>,
    surface: mat4x4<f32>,
    normal: mat3x3<f32>,
    color: vec4<f32>,
    material: Material,
    depth_range: vec4<f32>,
    lights: vec4<u32>,
}
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> scene_lighting: SceneLighting;
@group(0) @binding(2) var<uniform> solid: Solid;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
}
@vertex fn vs_main(vertex: Vertex) -> Output {
    let local = vec4<f32>(vertex.position, 1.0);
    let world = solid.world * local;
    let surface = solid.surface * local;
    let xy = ((surface.xy - globals.origin) / globals.resolution) * 2.0 - 1.0;
    var out: Output;
    out.position = vec4<f32>(xy.x, -xy.y, (solid.depth_range.y - surface.z) * solid.depth_range.z, 1.0);
    out.world = world.xyz;
    out.normal = solid.normal * vertex.normal;
    return out;
}
@fragment fn fs_main(in: Output) -> @location(0) vec4<f32> {
    return apply_lighting(solid.color, solid.material, in.world, normalize(in.normal), solid.lights.x);
}
