// A fixed overhead sun supplies the floor light. The camera looks straight down:
// both axes use the same logical-pixel scale, independent of viewport and DPR.
fn pool_color(p: vec2<f32>, base: vec3<f32>) -> vec3<f32> {
    let time = parameters.wave.x;
    let frequency = 0.0582 * parameters.wave.y;
    let amplitude = parameters.wave.z;
    let q = p * frequency + vec2<f32>(-9.2, 3.1);
    let u = q.x;
    let v = q.y;

    // Cross-wind makes short ripples cluster and dissolve over larger swells.
    let wind = 0.52 + 0.28 * sin(0.26*u + 0.14*v + 1.8 + 0.52*sin(0.38*v)) * cos(0.22*v - 0.14*time)
        - 0.19 * sin(0.11*u + 0.06*v);
    let chop = 0.15 + 1.04*smoothstep(0.18, 0.88, wind);
    let slow = sin(vec2<f32>(0.72*u - 0.33*v + 0.31*time, -0.38*u + 0.63*v - 0.27*time));
    let ripples = sin(vec4<f32>(
        1.49*u + 1.87*v + 1.10*slow.x + 0.38*slow.y - 0.91*time,
        -1.54*u + 1.22*v + 0.88*slow.y + time + 3.1,
        3.71*u - 0.99*v + 1.92*sin(0.91*u + 0.73*v - 0.45*time) + 1.32*time,
        -0.94*u + 4.20*v + 1.51*sin(0.74*u - 0.93*v + 0.57*time) - 1.61*time));
    let short_waves = sin(vec2<f32>(2.83*u + 8.01*v + 1.75*ripples.y + 0.67*ripples.w - 2.30*time,
        -5.34*u + 7.28*v + 0.97*ripples.x + 1.08*ripples.z + 2.07*time));
    let ripple_slope = vec2<f32>(dot(ripples.xyz, vec3<f32>(0.41, 0.37, 0.14)) + 0.085*short_waves.x,
        dot(ripples.xyw, vec3<f32>(0.35, -0.34, 0.20)) + 0.10*short_waves.y);
    let swell = q + vec2<f32>(
        1.13*sin(0.29*u - 0.43*v + 0.23*time) + 0.56*sin(0.21*u + 0.42*v - 0.19*time),
        1.05*sin(0.34*u + 0.23*v - 0.24*time) + 0.38*sin(0.48*u - 0.31*v + 0.35*time))
        + chop*ripple_slope*vec2<f32>(0.85, 1.12) * (amplitude / 0.65);

    let kx = vec4<f32>(1.12, -0.32, 0.78, -0.67);
    let ky = vec4<f32>(0.11, 0.92, -0.68, -0.65);
    let phase = kx*swell.x + ky*swell.y + vec4<f32>(0.43, -0.52, 0.56, 0.40)*time
        + vec4<f32>(2.1, 1.4, 0.3, 1.2);
    let weights = vec4<f32>(0.69, 0.75, 0.58, 0.38);
    let curvature = weights*cos(phase);
    let gradient = weights*sin(phase);
    let slope = vec2<f32>(dot(kx, gradient), dot(ky, gradient));

    // Estimate refracted footprint compression from the curvature of these four waves.
    // The finite core and asymmetric shoulder keep crossings bright and bounded.
    let focus = (1.68 + 0.24*sin(0.33*u + 0.21*v + 0.20*time)) * (amplitude / 0.65);
    let cross_curve = focus*dot(kx*ky, curvature);
    let det = (1.0 - focus*dot(kx*kx, curvature))*(1.0 - focus*dot(ky*ky, curvature)) - cross_curve*cross_curve;
    let facing = smoothstep(-0.60, 0.85, dot(slope, vec2<f32>(0.68,-0.43)) + 0.40*sin(0.53*swell.x - 0.71*swell.y + 0.40*time));
    let width = 0.050 + 0.048*facing;
    let pixel = max(input0_pixel_size().x, input0_pixel_size().y) * parameters.wave.w * frequency;
    let filtered = max(width, pixel*(0.72 + 0.58*chop));
    let core = (width/filtered) / (1.0 + (det/filtered)*(det/filtered));
    let side = det / (0.12 + 0.17*facing);
    let shoulder = (0.45 + 0.55*smoothstep(-0.29, 0.23, det)) / (1.0 + side*side);
    let activity = smoothstep(-0.65, 0.75, dot(slope, vec2<f32>(0.61,-0.25)) + 0.35*sin(0.28*u + 0.34*v + 0.23*time) + 0.20*ripples.z);
    let focus_energy = ((0.48 + 0.65*activity)*core + 0.32*shoulder + 0.105/(1.0 + det*det/0.2304))*(0.25 + 0.88*facing);

    let normal_xy = vec2<f32>(0.46, 0.49)*slope + vec2<f32>(0.77, 0.82)*chop*ripple_slope;
    let sky_angle = dot(normal_xy, vec2<f32>(0.18,0.96)) + 0.35*sin(0.31*v - 0.20*time);
    let sky = 0.18*smoothstep(-0.08,0.78,sky_angle) + 0.07*smoothstep(0.49,0.86,sky_angle);
    let shade = dot(slope,vec2<f32>(0.38,0.51)) + 0.59*chop*dot(ripple_slope,vec2<f32>(1.0,-0.52))
        + 0.28*sin(0.43*swell.x - 0.25*swell.y + 0.21*time);
    let lee = (det - 0.41)/0.28;
    let paint = base * (0.86 + 0.20*smoothstep(-1.2,0.85,shade)
        - 0.045*exp(-lee*lee)*smoothstep(-0.64,0.4,slope.y)*facing);
    // Diffuse skylight washes out some of the floor pigment on sky-facing slopes.
    let sky_wash = mix(base, vec3<f32>(0.90), 0.75);
    let reflected = mix(paint, sky_wash, sky);

    let sun_angle = dot(normal_xy, vec2<f32>(-0.33,0.92));
    let fine = dot(short_waves, vec2<f32>(0.32,0.23));
    let rough = chop*(dot(ripple_slope,vec2<f32>(0.42,0.32)) + fine);
    let fleck = smoothstep(0.16,0.80,rough + 0.29*normal_xy.x) * smoothstep(0.28,0.78,sun_angle);
    let daylight = 1.0 - exp(-1.80*focus_energy);
    let sunlit = mix(reflected, vec3<f32>(1.0), daylight + (1.0-daylight)*fleck*(0.16 + 0.67*wind));
    let normal = normalize(vec3<f32>(-normal_xy * (0.56*amplitude), 1.0));
    let lamp = scene_specular(p, normal, 96.0);
    let lit = sunlit + (vec3<f32>(1.0)-sunlit)*(1.0-exp(-0.85*lamp));
    return mix(base, lit, smoothstep(0.0,0.55,amplitude));
}
