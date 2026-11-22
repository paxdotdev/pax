// Repeat in local glass coordinates. Noise and mould marks stay put as lights move.
fn grain_hash(q: vec2<f32>) -> f32 {
    let bits = bitcast<vec2<u32>>(vec2<i32>(q));
    var h = (bits.x * 1597334677u) ^ (bits.y * 3812015801u);
    h = (h ^ (h >> 16u)) * 2246822519u;
    h = (h ^ (h >> 13u)) * 3266489917u;
    return f32((h ^ (h >> 16u)) & 65535u) / 65535.0;
}

// Value and its analytic gradient, used for rolled faces and the moulded lip.
fn glass_noise(p: vec2<f32>) -> vec3<f32> {
    let c = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let du = 6.0 * f * (1.0 - f);
    let a = grain_hash(c);
    let b = grain_hash(c + vec2<f32>(1, 0));
    let d = grain_hash(c + vec2<f32>(0, 1));
    let e = grain_hash(c + vec2<f32>(1, 1));
    return vec3<f32>(mix(mix(a,b,u.x), mix(d,e,u.x),u.y),
        du * vec2<f32>(mix(b-a,e-d,u.y), mix(d-a,e-b,u.x)));
}

fn mould_band(depth: f32, center: f32, width: f32, aa: f32) -> f32 {
    return 1.0 - smoothstep(width * 0.35, width + aa, abs(depth - center));
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    if parameters.identity != 0u || source.a <= 0.0 { return source; }
    let pitch = parameters.pitch;
    let aa = clamp(max(input0_pixel_size().x, input0_pixel_size().y) / pitch, 0.0007, 0.03);
    let cell = floor(p / pitch);
    let uv = fract(p / pitch) - 0.5;
    let seed = vec2<f32>(grain_hash(cell), grain_hash(cell + 37.0)) * 90.0;
    let folds = glass_noise(seed + vec2<f32>(uv.x * 3.3 + uv.y * 1.8, uv.y * 2.7 - uv.x));
    let warp = vec2<f32>(folds.x, glass_noise(seed.yx + 3.7 * uv).x);
    let fine = glass_noise(seed.yx + 9.0 * uv + 1.5 * warp);
    let angle = seed.x * 0.16;
    let axis = vec2<f32>(cos(angle), sin(angle));
    let striation = vec2<f32>(dot(uv, axis) * 21.0, dot(uv, vec2<f32>(-axis.y, axis.x)) * 5.0);
    let cords = glass_noise(seed + striation + 4.0 * warp);
    let rough = parameters.roughness;
    let thickness = parameters.thickness;
    let wobble = rough * (0.025 * (folds.x - 0.5) + 0.011 * (fine.x - 0.5));
    let corner = 0.064;
    let q = abs(uv) - (0.476 - corner);
    let edge = max(q, vec2<f32>(0));
    let d = corner - length(edge) - min(max(q.x, q.y), 0.0) + wobble;
    let cover = smoothstep(-aa, aa, d);
    let backing = source.rgb / source.a;
    let daylight = smoothstep(0.12, 0.90, dot(backing, vec3<f32>(0.2126, 0.7152, 0.0722)));
    let mortar = source.rgb * mix(0.60, 0.98, daylight);
    if cover <= 0.0 { return vec4<f32>(mortar, source.a); }
    let outward = sign(uv) * select(select(vec2<f32>(0,1), vec2<f32>(1,0), q.x > q.y),
        edge / max(length(edge), 0.0001), any(edge > vec2<f32>(0)));
    let inner_depth = mix(0.036, 0.078, thickness) + 0.015 * (folds.x - 0.5);
    let inner_q = abs(uv + vec2<f32>(0.004,-0.008)) - (0.476 - inner_depth - 0.047);
    let inner_d = 0.047 - length(max(inner_q, vec2<f32>(0))) - min(max(inner_q.x, inner_q.y), 0.0) + wobble;
    let shoulder = mould_band(d, 0.016, 0.009, aa);
    let lip = mould_band(inner_d, 0.0, 0.009 + 0.006 * fine.x, aa);
    let groove = mould_band(d, inner_depth * 0.55, inner_depth * 0.27, aa);
    let rim_slope = 1.0 - smoothstep(0.0, inner_depth, d);
    let relief = rough * (0.13 * folds.yz + 0.085 * fine.yz + 0.045 * cords.yz);
    let view = scene_view_direction(p);
    let plane = scene_world_normal(vec3<f32>(0,0,1));
    let side = select(-1.0, 1.0, dot(plane, view) >= 0.0);
    let n = side * plane;
    let facet = side * scene_world_normal(vec3<f32>(outward * (1.65 * rim_slope - 0.68 * lip) + relief, 1));
    let fresnel = 0.045 + 0.955 * pow(1.0 - clamp(dot(view, facet), 0.0, 1.0), 5.0);
    let ribbon = 1.0 - smoothstep(0.015, 0.070 + aa, abs(cords.x - 0.52));
    let polish = ribbon * smoothstep(0.32, 0.73, fine.x) * rough;
    let base = vec3<f32>(0.045,0.065,0.071) * (0.72 + 0.3 * folds.x + 0.36 * shoulder - 0.34 * groove);
    let inner_highlight = parameters.inner_bezel * lip;
    let outer_highlight = parameters.outer_bezel * shoulder;
    var rear_energy = vec3<f32>(0.0);
    var front_energy = vec3<f32>(0.0);
    var mask = scene_light_mask();
    while mask != 0u {
        let light = scene_incident_light(p, firstTrailingBit(mask));
        mask &= mask - 1u;
        let cosine = dot(n, light.direction);
        let directional = light.distance < 0.0;
        let height = select(cosine * light.distance, cosine, directional);
        let band = select(8.0, 0.06, directional);
        let front = smoothstep(-band, band, height);
        let contact = select(smoothstep(0.0, band, light.distance), 1.0, directional);
        let half_sum = view + light.direction;
        let half_vector = half_sum / max(length(half_sum), 0.0001);
        let glint = pow(clamp(dot(half_vector, facet), 0.0, 1.0), mix(160.0, 35.0, rough));
        let reflected = glint * (0.19 + 4.5 * fresnel + 1.25 * outer_highlight + inner_highlight);
        // Broad rear illumination with a displaced internal reflection at the lip.
        // The concentrating ridges turn with the light across each pressed brick.
        let rim_direction = dot(side * scene_world_normal(vec3<f32>(outward, 0)), light.direction);
        let folds_lit = dot(facet, -light.direction);
        let ripples = (0.12 * (folds.x - 0.5) + 0.26 * (fine.x - 0.5)) * rough;
        let body = (0.24 + ripples + 0.65 * pow(max(folds_lit, 0.0), 2.5)
            + 0.26 * polish * (0.28 + max(folds_lit, 0.0)))
            * exp(-thickness * (0.43 + 0.55 * rim_slope));
        let broken = mix(0.26, 1.45, smoothstep(0.15, 0.9, fine.x));
        let ridges = broken * (outer_highlight * (0.045 + 2.8 * pow(max(rim_direction, 0.0), 2.2))
            + inner_highlight * (0.065 + 3.4 * pow(max(-rim_direction, 0.0), 2.2)));
        let transmitted = (1.0 - fresnel) * (body * (1.0 - 0.43 * groove) + ridges);
        rear_energy += light.rgb * (contact * (1.0 - front) * transmitted);
        front_energy += light.rgb * (contact * front * reflected);
    }
    var rgb = base + (1.0 - exp(-(rear_energy + front_energy)));
    if daylight > 0.0 {
        // In a bright surround the pressed face transmits the backing. Shape the
        // relief in ambient light and leave headroom for a white surface glint.
        let rake = vec2<f32>(-0.55, -0.83);
        let relief_shade = rough * (0.17 * (folds.x - 0.5) + 0.12 * (fine.x - 0.5))
            + 0.26 * dot(relief, rake) + dot(outward, rake) * (0.13 * rim_slope - 0.09 * lip)
            - 0.18 * groove - 0.16 * (1.0 - smoothstep(0.0, 0.020, d));
        let pane = backing * (vec3<f32>(0.87, 0.90, 0.91)
            - thickness * vec3<f32>(0.064, 0.047, 0.041) + relief_shade);
        let rear = 1.0 - exp(-rear_energy);
        let peak = max(rear.r, max(rear.g, rear.b));
        // On white, give the transmitted hue contrast by absorbing its complement.
        let tinted = pane * (1.0 - 0.94 * (peak - rear)) + (1.0 - pane) * rear;
        let sheen = 1.0 - exp(-2.8 * front_energy);
        rgb = mix(rgb, tinted + (1.0 - tinted) * sheen, daylight);
    }
    return vec4<f32>(mix(mortar, rgb * source.a, cover), source.a);
}
