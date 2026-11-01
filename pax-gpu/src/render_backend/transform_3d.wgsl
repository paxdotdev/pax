const TRANSFORMS_PER_PAGE: u32 = 80u;
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
    projection_mode: f32,
    projection_x: vec4<f32>,
    projection_y: vec4<f32>,
    projection_origin: vec4<f32>,
    camera_z: vec4<f32>,
    camera_w: vec4<f32>,
    lighting_x: vec4<f32>,
    lighting_y: vec4<f32>,
    lighting_origin: vec4<f32>,
    lighting_normal: vec4<f32>,
}

struct Transforms {
    transforms: array<Transform, 80>,
}


fn identity_transform() -> Transform {
    return Transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0),
        vec4<f32>(1.0, 0.0, 0.0, 0.0), vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0), vec4<f32>(0.0, 0.0, 1.0, 0.0));
}
fn lighting_position(m: Transform, p: vec2<f32>, projected: vec2<f32>, row_x: vec3<f32>, row_y: vec3<f32>) -> vec3<f32> {
    if m.lighting_normal.w != 0.0 {
        return m.lighting_origin.xyz + p.x * m.lighting_x.xyz + p.y * m.lighting_y.xyz;
    }
    return vec3<f32>(dot(row_x, vec3<f32>(projected, 1.0)), dot(row_y, vec3<f32>(projected, 1.0)), 0.0);
}
fn lighting_normal(m: Transform) -> vec3<f32> { return m.lighting_normal.xyz; }

fn projection_enabled(m: Transform) -> bool { return m.projection_mode > 0.5; }
fn projected_position(m: Transform, p: vec2<f32>, affine: vec2<f32>) -> vec4<f32> {
    if projection_enabled(m) {
        return p.x * m.projection_x + p.y * m.projection_y + m.projection_origin;
    }
    return vec4<f32>(affine, 0.0, 1.0);
}
fn projected_camera_clip(m: Transform, p: vec2<f32>) -> vec2<f32> {
    if m.projection_mode > 1.5 {
        return vec2<f32>(dot(m.camera_z.xyz, vec3<f32>(p, 1.0)), dot(m.camera_w.xyz, vec3<f32>(p, 1.0)));
    }
    return vec2<f32>(0.5, 1.0);
}
