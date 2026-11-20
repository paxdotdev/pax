// Integer hashing keeps anchors independent of time, capture origin, and GPU trig precision.
fn flashy_hash(value: u32) -> u32 {
    var h = value;
    h = (h ^ (h >> 16u)) * 0x7feb352du;
    h = (h ^ (h >> 15u)) * 0x846ca68bu;
    return h ^ (h >> 16u);
}

fn flashy_random(value: u32) -> f32 {
    return f32(flashy_hash(value) & 0x00ffffffu) / 16777216.0;
}

fn flashy_key(c: vec2<i32>) -> u32 {
    return flashy_hash(bitcast<u32>(c.x)) ^ flashy_hash(bitcast<u32>(c.y) + 0x9e3779b9u) ^ parameters.seed;
}

fn flashy_anchor(c: vec2<i32>, key: u32) -> vec2<f32> {
    let jitter = vec2<f32>(flashy_random(key), flashy_random(key + 1u));
    return (vec2<f32>(c) + vec2<f32>(0.24) + jitter * 0.52) * parameters.spacing;
}

struct Facet { tangent_u: vec3<f32>, tangent_v: vec3<f32>, normal: vec3<f32> }

fn flashy_facet(key: u32) -> Facet {
    // Seeded facet normals stay fixed when only the scene light moves.
    let cycle = (parameters.phase - flashy_random(key + 2u)) * 6.28318530718;
    let wave = vec2<f32>(sin(cycle), cos(cycle));
    let roll = flashy_random(key + 3u) * 6.28318530718 + 0.42 * wave.x + 0.18 * wave.y;
    let pitch = 0.55 * wave.x + 0.14 * (2.0 * flashy_random(key + 4u) - 1.0);
    let yaw = 0.12 * wave.y + 0.10 * (2.0 * flashy_random(key + 5u) - 1.0);
    let u = vec3<f32>(cos(roll), sin(roll), 0.0);
    let v = vec3<f32>(-sin(roll), cos(roll), 0.0);
    let tipped_v = v * cos(pitch) + vec3<f32>(0.0, 0.0, sin(pitch));
    let tipped_n = vec3<f32>(0.0, 0.0, cos(pitch)) - v * sin(pitch);
    let facet_u = u * cos(yaw) - tipped_n * sin(yaw);
    let normal = tipped_n * cos(yaw) + u * sin(yaw);
    return Facet(facet_u, tipped_v, normal);
}

// Coverage and illumination depend only on the seeded facet and the current capture.
fn prepare_cell(c: vec2<i32>) -> vec4<f32> {
    if parameters.brightness <= 0.0 { return vec4<f32>(0.0); }
    let key = flashy_key(c);
    let anchor = flashy_anchor(c, key);
    let coverage = sample_input0(anchor).a;
    if coverage <= 0.0 { return vec4<f32>(0.0); }
    let specular = scene_specular(anchor, flashy_facet(key).normal, 110.0);
    if max(specular.r, max(specular.g, specular.b)) < 0.0002 { return vec4<f32>(0.0); }
    return vec4<f32>(specular, coverage);
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    if parameters.brightness <= 0.0 {
        return source;
    }
    let spacing = parameters.spacing;
    let flare_strength = min(parameters.flare, 1.0);
    let flare_scale = max(parameters.flare, 1.0);
    let support_radius = spacing * 0.95 * flare_scale;
    let cell = vec2<i32>(floor(p / spacing));
    let pixel_size = input0_pixel_size();
    let pixel = max(pixel_size.x, pixel_size.y);
    let core_width = max(spacing * 0.038, max(0.55, pixel * 0.6));
    var energy = vec3<f32>(0.0);
    // Anchors stay 0.24 cells inside each edge. An omitted cell is therefore
    // at least neighborhood + 0.24 cells away, even at the pixel cell's edge.
    // Keep the original 3x3 search at flare <= 1; expand only as support needs it.
    let neighborhood = i32(ceil(0.95 * flare_scale - 0.24));
    for (var y = -neighborhood; y <= neighborhood; y += 1) {
        for (var x = -neighborhood; x <= neighborhood; x += 1) {
            let c = cell + vec2<i32>(x, y);
            let light = load_cell(c);
            if light.a <= 0.0 { continue; }
            let key = flashy_key(c);
            let delta = p - flashy_anchor(c, key);
            let radius = length(delta);
            if radius >= support_radius { continue; }
            let facet = flashy_facet(key);
            // Only the tiny surface core inherits the facet's projected shape.
            // Invert its tangent frame to preserve rotation and foreshortening.
            // Bright facets face the viewer; the floor also protects very dim poses.
            let surface_q = vec2<f32>(
                delta.x * facet.tangent_v.y - delta.y * facet.tangent_v.x,
                facet.tangent_u.x * delta.y - facet.tangent_u.y * delta.x
            ) / max(facet.normal.z, 0.15);
            let core = 3.2 * exp(-dot(surface_q, surface_q) / (core_width * core_width));

            // The long star belongs to the shared viewing optics. Every glint
            // uses the same local-camera axes, independent of its facet's pose.
            let q = abs(vec2<f32>(dot(delta, vec2<f32>(0.939373, -0.342898)),
                dot(delta, vec2<f32>(0.342898, 0.939373))));
            let reach = spacing * mix(0.18, 0.42, flare_strength) * flare_scale;
            let narrow = max(core_width * 0.65, 0.45);
            let rays = exp(-q.x / reach - q.y / narrow)
                + 0.65 * exp(-q.y / (reach * 0.72) - q.x / narrow);
            // Broader shoulders scatter the rays through a little haze; the quiet
            // radial halo joins them without turning the surface into a uniform bloom.
            let mist = 0.20 * exp(-q.x / reach - q.y / (reach * 0.22))
                + 0.10 * exp(-radius / (reach * 0.52));
            let support = 1.0 - smoothstep(spacing * 0.60 * flare_scale, support_radius, radius);
            energy += light.a * light.rgb * (core + flare_strength * (rays + mist)) * support;
        }
    }
    let rgb = vec3<f32>(1.0) - exp(-energy * parameters.brightness);
    let alpha = max(rgb.r, max(rgb.g, rgb.b));
    // Premultiplied source-over: preserve the captured material between glints.
    return vec4<f32>(rgb, alpha) + source * (1.0 - alpha);
}
