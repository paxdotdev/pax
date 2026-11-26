// gas packs density, logical billow radius, drift phase and the rim switch.
// tint.a includes gas opacity. These two vec4s occupy two Pax uniform slots.

// A bank of overlapping lobes has a bright crown, a soft undercut and a
// coherent silhouette. Three staggered banks supply depth without ray marching.
fn seed(id: f32) -> f32 {
    return fract(sin(id * 127.1 + 17.4) * 4375.5453);
}

// Cross-sections of a smoothly joined cluster. Integrate the near surface
// analytically; its outward normal gives a broad crown and a shaded underside.
fn bank(q: vec2<f32>, layer: f32, t: f32, density: f32) -> vec2<f32> {
    let u = q.x + 7.3 * layer - t * (0.085 + 0.042 * layer);
    // Gas collects along a slowly curling stream. Its thicker pockets persist as
    // density falls; the connecting strands thin, break, and finally disappear.
    let pocket = 0.5 + 0.33 * sin(0.94 * u + 1.2 * layer) + 0.17 * sin(1.84 * u + 0.21 * t);
    let spine = 0.26 + 0.35 * sin(1.13 * u - layer) + 0.13 * sin(2.1 * u + 0.12 * t);
    let half_width = max(0.0, density - 2.1 * pocket) * 0.94;
    let feather = max(0.21, 3.1 / parameters.gas.y);
    let strand = smoothstep(0.0, feather, half_width - abs(q.y - spine));
    if strand == 0.0 { return vec2<f32>(0); }
    let cell = floor(u);
    var outline = -3.0;
    var weighted_light = 0.0;
    var weights = 0.0;
    for (var i = -1; i <= 1; i++) {
        let k = cell + f32(i);
        let r = seed(k + layer * 31.0);
        let center = vec2<f32>(k + 0.23 + 0.54 * r,
            0.38 * sin(1.67 * k + layer + 0.28 * t) + 0.65 * r);
        let radius = (0.68 + 0.48 * r) * (0.85 + 0.065 * density);
        // Bound the horizontal support, including its soft fringe, to three cells.
        let axes = vec2<f32>(min(1.0, radius / 0.92), radius);
        let delta = (vec2<f32>(u, q.y) - center) / axes;
        let edge = 1.0 - dot(delta, delta);
        let z = sqrt(max(edge, 0.0));
        let join = max(0.24 - abs(outline - edge), 0.0);
        outline = max(outline, edge) + join * join / 0.96;
        let w = exp2(6.0 * (z * radius + 0.38 * r - 1.0)) * smoothstep(-0.38, 0.25, edge);
        // A soft sky key gives the bank form even between direct lights.
        let n = normalize(vec3<f32>(delta, max(0.18, z)));
        let lit = smoothstep(-0.74, 0.86, dot(n, vec3<f32>(-0.49, -0.69, 0.53)));
        weighted_light += w * lit;
        weights += w;
    }
    let occupied = strand * smoothstep(-0.48, 0.38, outline);
    return vec2<f32>(occupied, weighted_light / max(0.001, weights));
}

fn gas_layers(p: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    if parameters.gas.x <= 0.0 || parameters.tint.a <= 0.0 {
        return vec2<f32>(0);
    }
    let t = parameters.gas.z;
    let xy = vec2<f32>(p.x, p.y - size.y * 0.5) / parameters.gas.y;
    let swirl = sin(xy.x * 1.27 + 0.45 * sin(1.8 * xy.y - t * 0.18) - t * 0.15);
    let curl = vec2<f32>(sin(2.1 * xy.y + 1.5 * swirl + 0.16 * t), swirl);
    let q = xy + 0.23 * curl + 0.06 * sin(4.7 * xy.yx + 2.2 * curl);
    let near = bank(q * 1.27 + vec2<f32>(0.0, -1.31), 3.0, t, parameters.gas.x);
    var coverage = near.x;
    var tone = coverage * (0.77 + 0.218 * near.y);
    if coverage < 1.0 {
        let mid = bank(q * 0.97 + vec2<f32>(0.0, -0.06), 2.0, t, parameters.gas.x);
        let amount = (1.0 - coverage) * mid.x;
        coverage += amount;
        tone += amount * (0.75 + 0.23 * mid.y);
    }
    if coverage < 1.0 {
        let far = bank(q * 0.72 + vec2<f32>(0.0, 0.99), 1.0, t, parameters.gas.x);
        let amount = (1.0 - coverage) * far.x;
        coverage += amount;
        tone += amount * (0.83 + 0.145 * far.y);
    }
    return vec2<f32>(coverage * parameters.tint.a, tone / max(0.0001, coverage));
}

fn effect(p: vec2<f32>) -> vec4<f32> {
    let coverage = sample_input0(p).a;
    if coverage <= 0.0 { return vec4<f32>(0); }
    let size = max(domain.local_size.xy, vec2<f32>(1));
    let layers = gas_layers(p, size);
    let gas_alpha = layers.x;
    let tone = layers.y;
    // Read the same vector silhouette used by the rim. Restrict the crown's
    // specular normal to the face and bevel, independently of the moving gas.
    let bevel = vec2<f32>(
        sample_input0(p - vec2<f32>(2.0, 0.0)).a - sample_input0(p + vec2<f32>(2.0, 0.0)).a,
        sample_input0(p - vec2<f32>(0.0, 2.0)).a - sample_input0(p + vec2<f32>(0.0, 2.0)).a);
    let uv = p / size - 0.5;
    let normal = scene_world_normal(vec3<f32>(uv * vec2<f32>(0.10, 0.34) + bevel * 0.46, 1));
    let plane = scene_world_normal(vec3<f32>(0, 0, 1));
    let eye = scene_view_direction(p);
    var scatter = vec3<f32>(0);
    var spec = vec3<f32>(0);
    var mask = scene_light_mask();
    while mask != 0u {
        let light = scene_incident_light(p, firstTrailingBit(mask));
        mask &= mask - 1u;
        let h = eye + light.direction;
        let half_dir = h / max(length(h), 0.0001);
        spec += light.rgb * pow(max(dot(normal, half_dir), 0.0), 240.0);
        scatter += light.rgb * pow(abs(dot(plane, light.direction)), 2.4);
    }
    let cool_shadow = vec3<f32>(0.92, 0.95, 1.0);
    let albedo = parameters.tint.rgb * mix(cool_shadow, vec3<f32>(1), smoothstep(0.75, 0.96, tone));
    let gas = albedo * (tone + (1.0 - tone) * (1.0 - exp(-2.1 * scatter)));
    // Clear glass contributes only its bevel and reflected light.
    let bevel_alpha = 0.32 * clamp(length(bevel), 0.0, 1.0);
    let reflection = vec3<f32>(1) - exp(-spec * 2.6);
    let reflected_alpha = max(max(reflection.r, reflection.g), reflection.b);
    let shell_alpha = mix(bevel_alpha, 1.0, reflected_alpha);
    let shell = reflection + (1.0 - reflected_alpha) * bevel_alpha * vec3<f32>(0.56, 0.65, 0.70);
    let body = vec4<f32>(shell + (1.0 - shell_alpha) * gas * gas_alpha,
                         shell_alpha + (1.0 - shell_alpha) * gas_alpha);

    if parameters.gas.w != 0.0 { return coverage * button_glint(p, bevel, body); }
    return coverage * body;
}
