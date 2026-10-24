struct Paint {
    stops: array<vec4<f32>, 8>,
    axis: vec4<f32>,
    off_axis: vec4<f32>,
    params: vec4<f32>,
};
struct Globals { resolution: vec2<f32>, dpr: vec2<f32>, origin: vec2<f32>, padding: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> paint_range: vec4<u32>;
@group(0) @binding(2) var<storage, read> paints: array<Paint>;
@group(1) @binding(0) var mesh_paints: texture_2d_array<f32>;
@group(1) @binding(1) var mesh_sampler: sampler;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec2<f32>,
    @location(1) camera_clip: vec2<f32>,
};
@vertex fn vs_main(@location(0) point: vec4<f32>, @location(1) paint: vec2<f32>, @location(2) camera_clip: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    let p = (point.xy - globals.origin * point.w) / globals.resolution * 2.0 - vec2<f32>(point.w);
    out.position = vec4<f32>(p.x, -p.y, point.z, point.w);
    out.world = paint;
    out.camera_clip = camera_clip;
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.camera_clip.y <= 0.0 || in.camera_clip.x < 0.0 || in.camera_clip.x > in.camera_clip.y { discard; }
    var alpha = 0.0;
    for (var i = 0u; i < paint_range.y; i++) {
        alpha += sample_alpha(paint_range.x + i, in.world);
    }
    return vec4<f32>(clamp(alpha, 0.0, 1.0));
}
// Keep stop-array reads in storage rather than copying an array-valued function
// argument. This also avoids invalid array-copy lowering in Vulkan-to-Metal drivers.
fn sample_alpha(paint_id: u32, world: vec2<f32>) -> f32 {
    let params = paints[paint_id].params;
    let axis_data = paints[paint_id].axis;
    let off_axis = paints[paint_id].off_axis;
    if params.w > 1.5 {
        let delta = world - axis_data.xy;
        let a = axis_data.zw;
        let b = off_axis.xy;
        let determinant = a.x*b.y - a.y*b.x;
        if abs(determinant) < 0.000001 { return 0.0; }
        let local = vec2<f32>(delta.x*b.y - delta.y*b.x, a.x*delta.y - a.y*delta.x) / determinant;
        let domain = paints[paint_id].stops[0];
        let uv = (local - domain.xy) / domain.zw;
        if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) { return 0.0; }
        return textureSampleLevel(mesh_paints, mesh_sampler, uv, i32(params.x), 0.0).a * params.y;
    }
    var alpha = params.x;
    let count = u32(params.z);
    if count > 0u {
        let delta = world - axis_data.xy;
        let axis = axis_data.zw;
        var coordinate = dot(delta, axis) / max(length(axis), 0.0001);
        let off = off_axis.xy;
        let determinant = axis.x * off.y - axis.y * off.x;
        if params.w < 0.5 && abs(determinant) > 0.0001 {
            coordinate = (delta.x * off.y - delta.y * off.x) / determinant * length(axis);
        }
        if params.w > 0.5 {
            coordinate = radial_coordinate(delta, axis, off, off_axis.zw);
            if coordinate < 0.0 { return 0.0; }
        }
        alpha = paints[paint_id].stops[0].y;
        for (var i = 1u; i < 8u; i++) {
            if i >= count { break; }
            let a = paints[paint_id].stops[i - 1u];
            let b = paints[paint_id].stops[i];
            if coordinate >= b.x { alpha = b.y; }
            else if coordinate > a.x {
                alpha = mix(a.y, b.y, clamp((coordinate - a.x) / max(b.x - a.x, 0.0001), 0.0, 1.0));
                break;
            } else { break; }
        }
    }
    return alpha * params.y;
}
