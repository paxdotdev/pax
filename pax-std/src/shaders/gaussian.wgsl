fn effect(p: vec2<f32>) -> vec4<f32> {
    if parameters.enabled == 0u { return vec4<f32>(0.0); }
    let sigma = parameters.sigma;
    if sigma < 0.001 { return sample_input0(p); }
    let step_size = dot(input0_pixel_size(), parameters.axis) * parameters.spacing;
    let n = i32(ceil(3.0 * sigma / step_size));
    var sum = vec4<f32>(0.0);
    var weight_sum = 0.0;
    for (var i = -n; i <= n; i += 1) {
        let offset = f32(i) * step_size;
        let weight = exp(-0.5 * offset * offset / (sigma * sigma));
        sum += sample_input0(p + parameters.axis * offset) * weight;
        weight_sum += weight;
    }
    return sum / weight_sum;
}
