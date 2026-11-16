# Rain on dark glass

A web/WGPU study of a dark, wet surface under real scoped Pax lights. Scattered
raindrop impacts and descending teardrops pool into one implicit surface. Drops land across the glass as brief rounded splotches; a continuous
taper grows only as each drop descends.
The shader derives relief and normals from that contour, so the head and its
rivulet are one form. This fixture uses analytic animation, not a fluid solver.
There are no texture assets, generated images, readbacks or history buffers.

From the repository root:

```sh
cargo run -p pax-cli -- run --path examples/fixtures/rain-pane --target web
```

Open `/preview.html` on the printed local server for the visibility-aware host.
It calls the documented `window.Pax.setSuspended` API when its tab is hidden.
Opening `/` directly skips that host policy.

Foreground Text and the button remain native elements outside the shader capture.
The rain is passive: mouse and touch input do not displace the water or intercept
scrolling. The fixture controls below the panel adjust its treatment.

`WetPane::default()` defines the tuned surface settings; the fixture initializes
its controls from the same defaults. Hosts can omit these properties when using
the default treatment and bind `time` to their own animation clock.

| Setting | Default |
| --- | --- |
| Wetness | 0.69 |
| Drop scale | 0.70× |
| Rain frequency | 10× |
| Roughness | 0.73 |
| Highlight strength | 0.40 |
| Descent speed | 2.4 logical pixels/sec |
| Seed | 17 |
| Output quality | Half resolution |
| Copy space | Textured (`quiet=false`) |

## Controls

- **Pause / Play** holds or resumes water and light. Scrubbing the light pauses.
- **Reset rain** restores the deterministic starting pose.
- **Rain frequency** binds `rain_frequency` on `WetPane`, a 0–12× arrival-cadence
  multiplier (default 10). Higher values add overlapping generations without
  multiplying descent speed or prematurely replacing an older drop. Zero disables
  recurring arrivals; the seeded initial drops continue draining. This is relative
  to each emitter's drainage time, not a fixed number of drops/second across all
  panel sizes. Editing frequency reschedules the analytic field; it does not
  preserve a history of previous parameter values. More frequent rain also raises
  the amount of water present; wetness independently selects active emitters.
- **View** cycles water, a plain Rectangle, and an identity shader container.
  Plain removes the capture; identity isolates its overhead.
- **Motion** selects both clocks, water only, or light only. Both run by default.
- **Light** defaults to three drifting cyan, magenta and yellow point lights, 220px
  above the surface. It also cycles through a white strip, white point source,
  and no direct illumination. Without lights the shader adds no highlights.
  Each point follows a different responsive path, computed from the light clock
  through the Pax property DAG. These are ordinary scoped `LightSource` nodes;
  their colours and positions are not baked into the water shader.
- **Quality** selects half or full physical resolution for the output pass.
  Source capture and MSAA scratch remain full resolution. The clocks update at
  most 60 times/sec on wide layouts and 30 times/sec below 700px.
- **Copy space** optionally softens highlights behind the text. The default is
  textured across the full panel.
  Wetness, drop scale, roughness and highlight strength tune the surface without
  adding passes.

Both clocks hold when the panel is outside the viewport. A clock gap of 100ms or
more is discarded, so resuming does not simulate missed time. The light traverses
a 36-second cycle. `speed` sets the nominal descent rate in logical pixels/sec
(default 2.4). Each drop has a seeded average rate and a 1.8–3.4 second rhythm:
slow creep interrupted by short eased advances, with different timing on every
beat. At the default speed, slips move a few logical pixels rather than long
jumps. Larger drops generally descend faster. No rain drops remain pinned; Pause
and light-only motion deliberately hold the water clock.

**Drop scale** spans 0.3–4× (default 0.7). It scales the size spread above a 4.2px
primary-drop radius floor, so lowering it mainly shrinks large drops and pools
while retaining the fine droplets. At the default scale the primary radii span
4.2–11.06px; at 1× they span 4.2–14px.
At 0.3× the span is 4.2–7.14px. Small collection partners
have a 3.2px floor before absorption. These are nominal logical radii: impacts,
collection and overlapping fields can make a combined pool larger. Scale does not
spread emitter positions. Wider spatial searches at large scales and more
simultaneous generations at high frequency increase shader work.

## Pooling and persistence

Drops contribute to a shared broad potential field. Nearby heads fill the saddle
between them before a single relief and normal are derived, so a contact reads as
one fuller pool rather than a narrow bridge between independently lit domes.
Constructed catch-up pairs pull into a common centre within 160ms of head contact;
the smaller body stays absorbed for the rest of that generation. The pooled body
uses their combined volume proxy and a shorter, rounder taper.

This is a visual surface-tension approximation. It does not guarantee a globally
convex outline for every multi-drop cluster. Incidental contacts share the same
surface while close, but can separate again when their prescribed paths diverge;
the proximity effect is deliberately recomputed each frame, allowing water to
join and separate without storing collision history. It is not a volume-conserving
fluid simulation.

Twelve independently timed emitters per 88px lane keep the candidate count bounded.
Wetness admits a similar count in every lane, with positions jittered within it.
Up to twelve overlapping generations per emitter support the frequency range.
A seeded prehistory makes the first frame already wet. Each arrival appears
inside the panel as a rapid connected splotch; descent begins after 0.18 seconds.
Tail length is capped by distance traveled. Old generations retire only after
their surface has left the bottom; new generations land at different horizontal
positions. Birth heights are seeded per emitter. There is no top-edge feed or
separate stationary bead layer.

Rust stores clocks and control values only. Ordinary Pax properties feed ten typed
uniform slots (160 bytes) to the shader. All water geometry and shading run in one
fragment pass over an unlit dark Rectangle. Animation can reuse that unchanged
source capture; there is no per-frame image rasterization or upload. Higher rain
frequency evaluates more overlapping generations and therefore costs more GPU
work; half-resolution output remains the default.

## Rendering boundaries

`WetPane` requests `ShaderLighting` and calls `scene_specular` with the final
surface normal. LightFrame scope, transforms, radius falloff, colour and camera
come from the engine. Analytic derivatives travel through the summed potential
before the shader computes the shared relief and local normal.
The shader takes one source sample per output pixel; source capture and composition
are additional renderer work. It does not sample a displaced backdrop or native
pixels, and it does not define a public material API.
Lighting remains independent of this effect: `Example` owns the CMY emitters,
their motion and comparison controls in `src/lib.pax` and `src/lib.rs`.
`WetPane` creates no lights and receives scoped RGB illumination from its host.
A website using the effect supplies its own `LightSource` nodes in the applicable
lighting scope; it need not copy the example's light rig or light clock.

The water adds a narrow specular reflection without diffuse body colour or a broad
soft highlight underneath. Roughness controls the reflection's width; highlight
strength controls its brightness. The captured source and its alpha are preserved
outside the reflections, so reducing body haze does not require dimming the glints.
This is a clear-looking highlight treatment: no frosted-glass layer, transmission,
or backdrop refraction is included in this study.

CPU fallback design and qualification remain deferred. The existing CPU shader
bypass can show the plain child; it is not a qualified wet treatment. Automatic
system reduced-motion handling remains PAX-1048; use Pause for a static pose.
Other WGPU chassis are not qualified by a web build.

For local profiling, append `?pax_scroll_perf=1&pax_surface_trace=1&pax_work_profile=1`
to the preview URL. Compare matched plain/identity/water views, stationary and
animated cases, and different arrival frequencies. CPU times and browser callback
intervals are not GPU execution times. Research results and proposed budgets live
in [PAX-1077](https://linear.app/paxdev/issue/PAX-1077).
