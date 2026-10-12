fn paint_cezanne(p: vec2<f32>) -> vec4<f32> {
    var paint = ground(pigment(sample_input0(p),-0.08,1.08,0.0),0.08);
    for (var layer = 0u; layer < 2u; layer += 1u) {
        let spacing = parameters.size * select(0.8,0.42,layer == 1u);
        let cell = vec2<i32>(floor(p / spacing));
        var colors = vec4<f32>(0.0);
        var coverage = 0.0;
        for (var y = -2; y <= 2; y += 1) {
            for (var x = -2; x <= 2; x += 1) {
                let id = cell + vec2<i32>(x,y);
                let r = random(id,300u + layer);
                let anchor = (vec2<f32>(id) + vec2<f32>(0.5) +
                    vec2<f32>(r - 0.5,random(id,311u) - 0.5) * 0.38) * spacing;
                if (length(p - anchor) > spacing * 1.6) { continue; }
                // Irregular patches keep a local relationship without imposing one
                // canvas-wide hatch direction or a visible square patch grid.
                let patch_size = parameters.size * 2.4;
                let patch_p = anchor / patch_size;
                let cluster = vec2<i32>(floor(patch_p + 0.24 *
                    vec2<f32>(sin(patch_p.y * 2.1),sin(patch_p.x * 1.7))));
                let group = random(cluster,320u);
                let curved = random(cluster,321u) > 0.65;
                let fan_phase = random(cluster,322u) * 6.283185;
                // Centers outside the patches yield broad nested arcs, preserving
                // Cézanne's planar construction instead of forming little vortices.
                let fan_center = (vec2<f32>(cluster) + vec2<f32>(0.5) +
                    vec2<f32>(cos(fan_phase),sin(fan_phase)) *
                    (1.0 + 0.55 * random(cluster,323u))) * patch_size;
                let radial = anchor - fan_center;
                let fan_angle = atan2(radial.y,radial.x) + 1.5707963;
                let angle = select(group * 3.1415927,fan_angle,curved) + (r - 0.5) * 0.18;
                let axis = vec2<f32>(cos(angle),sin(angle));
                let center = moving_mark(anchor,axis,spacing,group * 2.0,vec2<f32>(0.18,0.06),0.55);
                let d = p - center;
                let breath = 1.0 + 0.075 * sin(parameters.phase * 0.7 + group * 6.283185);
                let along = dot(d,axis) / (spacing * 0.87 * breath);
                // Neighboring curved marks share a center, forming nested arcs.
                let curvature = select(0.0,min(0.55,spacing * 1.4 / max(length(radial),spacing)),curved);
                let across = dot(d,vec2<f32>(-axis.y,axis.x)) / (spacing * 0.27 / breath) -
                    curvature * along * along;
                let taper = 0.15 * along + 0.07 * sin(across * 12.0 + r * 7.0);
                let shape = max(abs(along) + taper,abs(across) + 0.08 * along * along);
                let frequency = 8.0 * max(input0_pixel_size().x,input0_pixel_size().y) / (spacing * 0.27);
                let bristles = 1.0 - smoothstep(1.8,3.14,frequency);
                let mask = brush_edge(shape,spacing) * (0.94 + 0.06 * bristles * sin(across * 8.0 + r * 10.0));
                if (mask <= 0.0) { continue; }
                let sample = sample_input0(center);
                let field = sample_input1(center);
                let tone = (group - 0.5) * 0.035 + (r - 0.5) * 0.075;
                let color = pigment(sample,(group - 0.5) * 1.15,1.15,tone);
                let detail = select(1.0,0.28 + field.z * 0.7,layer == 1u);
                colors += color * mask * detail;
                coverage += mask * detail;
            }
        }
        paint = mix(paint,colors / max(coverage,0.0001),min(coverage,1.0) * 0.94);
    }
    // Fragmented blue-violet contour strands follow local source edges. Unequal offsets
    // and a coherent break pattern avoid turning every silhouette into a solid outline.
    let wobble = (noise(p / (parameters.size * 0.7),340u) - 0.5) * parameters.size * 0.13;
    let q = p + vec2<f32>(wobble,-wobble);
    let h = min(max(input0_pixel_size().x,input0_pixel_size().y) * 1.2,parameters.size * 0.1);
    let gx = sample_input0(q + vec2<f32>(h,0.0)).rgb - sample_input0(q - vec2<f32>(h,0.0)).rgb;
    let gy = sample_input0(q + vec2<f32>(0.0,h)).rgb - sample_input0(q - vec2<f32>(0.0,h)).rgb;
    let strand = smoothstep(0.025,0.3,length(gx) + length(gy));
    let breaks = smoothstep(0.25,0.75,noise(p / (parameters.size * 0.24),341u));
    let ink = vec3<f32>(0.15,0.19,0.27) * paint.a;
    paint = vec4<f32>(mix(paint.rgb,ink,strand * breaks * 0.48),paint.a);
    return paint;
}
