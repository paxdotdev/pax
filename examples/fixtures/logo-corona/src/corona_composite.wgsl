fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input1(p);
    let coverage = aperture(source);
    let field = corona_field(p);
    let shade = coverage * parameters.surface * (1.0 - field);
    let core = vec4<f32>(source.rgb + (parameters.background.rgb - parameters.foreground.rgb) * shade, source.a);
    let beams = sample_input0(p);
    // Keep the full-resolution emitting aperture in front of the beams. The
    // painted letter color sits behind them, just like the outside background.
    // Its premultiplied coverage is source.a - coverage; remove only the portion
    // displaced by beam alpha so opaque interiors and transparent exteriors blend alike.
    let painted = parameters.background.rgb * (source.a - coverage);
    return vec4<f32>(
        core.rgb + beams.rgb * (1.0 - coverage) - painted * beams.a,
        source.a + beams.a * (1.0 - source.a)
    );
}
