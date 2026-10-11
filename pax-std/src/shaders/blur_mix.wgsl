fn effect(p: vec2<f32>) -> vec4<f32> {
    return mix(sample_input0(p), sample_input1(p), parameters.approximation);
}
