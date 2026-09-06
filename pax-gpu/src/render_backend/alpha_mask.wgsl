struct Paint {
    stops: array<vec4<f32>, 8>,
    axis: vec4<f32>,
    off_axis: vec4<f32>,
    params: vec4<f32>,
};
struct Globals { resolution: vec2<f32>, dpr: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<uniform> paint: Paint;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world: vec2<f32>,
};
@vertex fn vs_main(@location(0) point: vec2<f32>) -> VertexOutput {
    var out: VertexOutput;
    let p = point / globals.resolution * 2.0 - 1.0;
    out.position = vec4<f32>(p.x, -p.y, 0.0, 1.0);
    out.world = point;
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var alpha = paint.params.x;
    let count = u32(paint.params.z);
    if count > 0u {
        let delta = in.world - paint.axis.xy;
        let axis = paint.axis.zw;
        var coordinate = dot(delta, axis) / max(length(axis), 0.0001);
        let off = paint.off_axis.xy;
        let determinant = axis.x * off.y - axis.y * off.x;
        if paint.params.w < 0.5 && abs(determinant) > 0.0001 {
            coordinate = (delta.x * off.y - delta.y * off.x) / determinant * length(axis);
        }
        if paint.params.w > 0.5 {
            let uv = vec2<f32>(delta.x * off.y - delta.y * off.x,
                axis.x * delta.y - axis.y * delta.x) / max(abs(determinant), 0.0001);
            coordinate = length(uv) * length(axis);
        }
        alpha = paint.stops[0].y;
        for (var i = 1u; i < 8u; i++) {
            if i >= count { break; }
            let a = paint.stops[i - 1u];
            let b = paint.stops[i];
            if coordinate >= b.x { alpha = b.y; }
            else if coordinate > a.x {
                alpha = mix(a.y, b.y, clamp((coordinate - a.x) / max(b.x - a.x, 0.0001), 0.0, 1.0));
                break;
            } else { break; }
        }
    }
    alpha = clamp(alpha * paint.params.y, 0.0, 1.0);
    return vec4<f32>(alpha);
}
