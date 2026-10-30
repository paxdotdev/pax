const TRANSFORMS_PER_PAGE: u32 = 160u;
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
    lighting_x: vec4<f32>,
    lighting_y: vec4<f32>,
    lighting_origin: vec4<f32>,
    lighting_normal: vec4<f32>,
}

struct Transforms {
    transforms: array<Transform, 160>,
}


fn identity_transform() -> Transform {
    return Transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
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
