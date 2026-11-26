fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    if source.a <= 0.0 { return vec4<f32>(0.0); }
    let body = vec4<f32>(pool_color(p, source.rgb / max(source.a, 0.0001)), 1.0);
    let bevel = vec2<f32>(
        sample_input0(p - vec2<f32>(2.0, 0.0)).a - sample_input0(p + vec2<f32>(2.0, 0.0)).a,
        sample_input0(p - vec2<f32>(0.0, 2.0)).a - sample_input0(p + vec2<f32>(0.0, 2.0)).a);
    return source.a * button_glint(p, bevel, body);
}
