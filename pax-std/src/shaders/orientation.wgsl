fn luminance(p: vec2<f32>) -> f32 {
    let c = sample_input0(p);
    return dot(c.rgb, vec3<f32>(0.2126,0.7152,0.0722));
}
fn effect(p: vec2<f32>) -> vec4<f32> {
    let h = parameters.size * 0.4;
    let gx = luminance(p + vec2<f32>(h,0)) - luminance(p - vec2<f32>(h,0));
    let gy = luminance(p + vec2<f32>(0,h)) - luminance(p - vec2<f32>(0,h));
    // Tangent to the local color edge; a diagonal fallback keeps flat fields painterly.
    let tangent = normalize(vec2<f32>(-gy,gx) + vec2<f32>(0.014,0.007));
    return vec4<f32>(tangent * 0.5 + 0.5, clamp(length(vec2<f32>(gx,gy)),0.0,1.0), 1.0);
}
