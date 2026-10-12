fn effect(p: vec2<f32>) -> vec4<f32> {
    switch parameters.mode {
        case 1u: { return paint_gauguin(p); }
        case 2u: { return paint_van_gogh(p); }
        case 3u: { return paint_pointillist(p); }
        case 4u: { return paint_cezanne(p); }
        default: { return paint_basic(p); }
    }
}
