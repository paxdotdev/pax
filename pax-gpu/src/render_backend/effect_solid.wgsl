fn pax_effect_sample_solid(p: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let uv = input_uv(p);
    if !visible_local(p) || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return vec4<f32>(0.0);
    }
    // Inverse projection maps a backing pixel's footprint onto both local axes.
    // Use the source density even when this pass shades at a coarser resolution.
    let surface = vec3<f32>(domain.rect.xy + uv * domain.rect.zw, 1.0);
    let pixel = domain.rect.zw / (dot(domain.local2.xyz, surface) * domain.storage.xy);
    let dx = (domain.local0.xy - p.x * domain.local2.xy) * pixel;
    let dy = (domain.local1.xy - p.y * domain.local2.xy) * pixel;
    let span = max(vec2<f32>(abs(dx.x) + abs(dx.y), abs(dy.x) + abs(dy.y)), vec2<f32>(1e-8));
    let inside = saturate(p / span + 0.5) - saturate((p - domain.local_size.xy) / span + 0.5);
    return color * (inside.x * inside.y);
}
