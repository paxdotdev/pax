fn hash(v: u32) -> u32 {
    var h = v;
    h = (h ^ (h >> 16u)) * 0x7feb352du;
    h = (h ^ (h >> 15u)) * 0x846ca68bu;
    return h ^ (h >> 16u);
}
fn random(v: u32) -> f32 { return f32(hash(v) & 0x00ffffffu) / 16777216.0; }
fn cell_key(cell: vec2<i32>, seed: u32) -> u32 {
    return hash(bitcast<u32>(cell.x)) ^ hash(bitcast<u32>(cell.y) + 0x9e3779b9u) ^ seed;
}
// A monotone clock: random endpoints agree across each beat boundary, while a
// little creep keeps even the quiet part moving. No per-drop CPU state is needed.
fn descent(time: f32, key: u32) -> f32 {
    let period = 1.8 + random(key + 41u) * 1.6;
    let phase = time / period + random(key + 42u) * 31.0;
    let beat = floor(phase);
    let part = fract(phase);
    let k = hash(key + u32(beat) * 73u);
    let start = 0.25 + random(k + 1u) * 0.28;
    let end = start + 0.18 + random(k + 2u) * 0.16;
    let z = clamp((part - start) / (end - start), 0.0, 1.0);
    let progress = 0.035 * part + 0.965 * z * z * (3.0 - 2.0 * z);
    let a = beat + random(hash(key + u32(beat) * 73u)) * 0.3;
    let b = beat + 1.0 + random(hash(key + (u32(beat) + 1u) * 73u)) * 0.3;
    return mix(a, b, progress) * period;
}
// Nearby drops overlap in one smooth potential. An isolated contribution has
// a convex pear-shaped contour; overlapping heads fill their common saddle.
// Compact support makes this a local proximity calculation, with no history.
fn drop_field(p: vec2<f32>, head: vec2<f32>, radius: f32, reach: f32, bend: f32,
    impact: f32, key: u32) -> vec3<f32> {
    let angle = impact * (random(key + 61u) - 0.5) * 1.5;
    let c = cos(angle); let s = sin(angle);
    let delta = p - head;
    let local = vec2<f32>(c * delta.x + s * delta.y, -s * delta.x + c * delta.y);
    let rx = radius * (1.0 + impact * 0.15);
    // Two half-ellipses meet with a continuous gradient at the widest part:
    // a long upper taper and a round lower head. Every influence contour stays
    // convex, including the outer contours exposed when several drops pool.
    let ry = (radius + select(0.0, reach * 0.5, local.y < 0.0)) * (1.0 - impact * 0.1);
    let shear = bend / (radius + reach);
    let y = local.y / ry;
    let x = (local.x + shear * local.y) / rx;
    let squared = x * x + y * y;
    if squared >= 9.0 { return vec3<f32>(0.0); }
    let q = sqrt(squared);
    let value = exp(0.5 * (1.0 - squared));
    let z = clamp((q - 2.5) / 0.5, 0.0, 1.0);
    let fade = 1.0 - z * z * (3.0 - 2.0 * z);
    let gradient = 2.0 * vec2<f32>(x / rx, x * shear / rx + y / ry);
    let derivative = -0.5 * value * (fade + 6.0 * z * (1.0 - z) / (0.5 * max(q, 0.001)));
    let local_gradient = gradient * derivative;
    return vec3<f32>(value * fade,
        c * local_gradient.x - s * local_gradient.y, s * local_gradient.x + c * local_gradient.y);
}
fn water_distance(p: vec2<f32>) -> vec3<f32> {
    let scale = parameters.scale;
    var field = vec3<f32>(0.0);
    let lane = i32(floor(p.x / 88.0));
    // Scale changes the spread above the small-drop floor. Expand the spatial
    // lookup only as far as the largest supported contribution can reach.
    let lane_reach = i32(ceil((4.2 + 9.8 * scale) * 4.7 / 88.0));
    // Frequency adds overlapping generations; it never multiplies descent time.
    for (var offset = -lane_reach; offset <= lane_reach; offset += 1) {
        for (var slot = 0; slot < 12; slot += 1) {
            let key = cell_key(vec2<i32>(lane + offset, slot), parameters.seed + 113u);
            if (f32(slot) + random(key) * 0.8) / 12.0 > parameters.wetness { continue; }
            let size = random(key + 3u);
            let radius = 4.2 + 9.8 * pow(size, 1.5) * scale;
            let rate = (0.35 + size * 0.85) * (0.7 + random(key + 4u) * 0.6);
            let velocity = parameters.speed * rate;
            let paired = random(key + 7u) < 0.3;
            let small_radius = min(radius * 0.85, max(3.2, radius * (0.45 + random(key + 8u) * 0.2)));
            let gap = radius * 2.2 + 20.0 * scale;
            let margin = radius * 1.5;
            let start_y = mix(margin + select(0.0, gap, paired),
                max(margin + gap, parameters.extent.y - margin), pow(random(key + 5u), 2.0));
            let period = 1.8 + random(key + 41u) * 1.6;
            let lifetime = (parameters.extent.y + radius * 9.0 + 24.0 * scale - start_y)
                / max(velocity, 0.001) + 2.0 * period + 0.2;
            let prehistory = pow(random(key + 10u), 3.0) * lifetime;
            let repeat = lifetime / max(parameters.frequency, 0.001);
            let phase = (parameters.time + prehistory) / repeat;
            let side = select(-1.0, 1.0, (key & 1u) == 0u);
            let separation = vec2<f32>(side * radius * 0.8, -gap);
            let approach_time = 24.0 + random(key + 9u) * 32.0;
            let touch_phase = max(0.0, 1.0 - (radius + small_radius) / length(separation));
            let touch_age = approach_time * touch_phase;
            let volume_fraction = pow(small_radius / radius, 3.0);
            let merged_radius = radius * pow(1.0 + volume_fraction, 1.0 / 3.0);
            let pooled_offset = separation * (1.0 - touch_phase) * volume_fraction / (1.0 + volume_fraction);
            for (var cohort = 0u; cohort < 12u; cohort += 1u) {
                var age = fract(phase) * repeat + f32(cohort) * repeat;
                var generation = i32(floor(phase)) - i32(cohort);
                if parameters.frequency <= 0.0 {
                    if cohort > 0u { break; }
                    age = parameters.time + prehistory;
                    generation = 0;
                }
                if age > lifetime { break; }
                let arrival_key = hash(key + bitcast<u32>(generation) * 193u);
                let x = (f32(lane + offset) + 0.1 + random(arrival_key + 1u) * 0.8) * 88.0;
                if abs(p.x - x) > radius * 4.7 { continue; }
                let moved = max(0.0, descent(max(0.0, age - 0.18), key) - descent(0.0, key)) * velocity;
                // Both drops descend before contact. Surface tension then pulls
                // their centres together in 160ms; the small body stays retired.
                let snap = select(0.0, smoothstep(touch_age, touch_age + 0.16, age), paired);
                let raw_head = vec2<f32>(x, start_y + moved);
                let head = raw_head + pooled_offset * snap;
                let impact = 1.0 - smoothstep(0.06, 0.42, age);
                let appearance = smoothstep(0.0, 0.065, age) * (1.0 + impact * 0.2);
                if appearance < 0.005 { continue; }
                let grown = mix(radius, merged_radius, snap) * appearance;
                let full_reach = grown * mix(1.6 + random(key + 6u) * 1.1, 1.6, snap);
                let reach = min(full_reach, moved);
                let bend = (random(key + 2u) - 0.5) * grown * 0.7 * reach / max(full_reach, 0.001) * (1.0 - snap * 0.7);
                if p.y < head.y - gap - (reach + grown) * 3.5 || p.y > head.y + (reach + grown) * 3.5 { continue; }
                field += drop_field(p, head, grown, reach, bend, impact, arrival_key);
                if paired && snap < 0.999 {
                    let approach = min(touch_phase, age / approach_time);
                    let center = mix(raw_head + separation * (1.0 - approach), raw_head + pooled_offset, snap);
                    let r = small_radius * pow(1.0 - snap, 1.0 / 3.0) * appearance;
                    let small_moved = max(0.0, center.y - start_y + gap);
                    field += drop_field(p, center, r, min(r * 2.0, small_moved) * (1.0 - snap),
                        bend * 0.4, impact, arrival_key + 1u);
                }
            }
        }
    }
    let density = max(field.x, 0.000001);
    let relief = 12.0 * scale;
    return vec3<f32>(log(density) * relief, field.yz * relief / density);
}
fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    let surface = water_distance(p);
    let distance = surface.x;
    if distance <= 0.0 || parameters.wetness <= 0.0 || parameters.strength <= 0.0 { return source; }
    // The shared potential carries its derivative through every contribution. No
    // screen derivatives or repeated finite-difference scene evaluations.
    let slope = surface.yz * 0.8 * sqrt(5.0 * parameters.scale / max(distance, 0.12));
    let normal = normalize(vec3<f32>(-slope, 1.0));
    let pixel = input0_pixel_size();
    let coverage = smoothstep(0.0, max(pixel.x, pixel.y) * 1.6, distance);
    // Clear water adds a tight reflection, not a broad body-colour fill.
    // Away from the glint the captured surface shows through unchanged.
    let illumination = scene_specular(p, normal, mix(320.0, 32.0, parameters.roughness));
    let quiet = mix(1.0, mix(0.12, 1.0,
        smoothstep(parameters.extent.x * 0.25, parameters.extent.x * 0.70, p.x)), parameters.quiet);
    let energy = illumination * coverage * parameters.strength * quiet;
    let highlight = (vec3<f32>(1.0) - exp(-energy * 1.1)) * 0.68;
    return vec4<f32>(source.rgb + highlight * source.a * (vec3<f32>(1.0) - source.rgb), source.a);
}
