@group(1) @binding(0) var pax_cells: texture_2d<f32>;
fn load_cell(c: vec2<i32>) -> vec4<f32> {
    let origin = parameters.pax_cell_rect.xy;
    if all(c >= origin) && all(c < origin + parameters.pax_cell_rect.zw) {
        return textureLoad(pax_cells, c - origin, 0);
    }
    return pax_evaluate_cell(c);
}

@fragment fn fs_prepare_cells(v: EffectVertex) -> @location(0) vec4<f32> {
    return pax_evaluate_cell(parameters.pax_cell_rect.xy + vec2<i32>(v.position.xy));
}
