# Pool floor

A straight-down view of sunlight on rippling water. Broad patches of the selected floor color give way to thin caustic streaks and small flashes on the surface. The pattern stays anchored to the panel’s local pixels as it resizes.

From the monorepo root, run:

```sh
cargo run -p pax-cli -- run -p examples/fixtures/pool-floor -t web --hot-reload=pax
```

Open `/preview.html` on the printed server. Move the pointer or tap the water to position the warm point light. **Pause waves** holds the water while the lamp remains interactive. The preview also provides a floor-color hex field (RGB or RGBA), half/full resolution, and water/paint/identity comparisons. Click the floor-color swatch beside the hex field to open the standard picker. Edit the paint using the palette, hue and opacity sliders, or RGBA fields. Click outside the picker to dismiss it. Use `/preview.html?pax_renderer=cpu` to see the solid-paint fallback.

## Use in another Pax app

From a sibling project such as `pax-website`, add the fixture as a dependency:

```toml
pool-floor = { path = "../examples/fixtures/pool-floor", default-features = false }
```

Import `pool_floor::PoolFloor` in the Rust module that owns the template. The package enables `pax-kit/shaders`. Its `demo` feature adds the preview controls and runnable entry point.

```pax
<LightFrame width=100% height=360px>
    <Text x=24px y=24px width=65% height=48px
        style={font: "Georgia", font_size: 32px, fill: rgb(9,53,63)}>Poolside</Text>
    <PoolFloor width=100% height=100% density=1.0 amplitude=0.65 speed=1.0
        floor_color=rgb(0,197,211)/>
    <LightSource x=70% y=30% width=1px height=1px z=94px radius=420px
        intensity=2.1 color=rgb(255,226,175)/>
</LightFrame>
```

The lamp is optional. `PoolFloor` supplies the sunlit pattern on its own and consumes eligible scene lights for additional reflections. Keep copy and native controls above the panel as siblings.

| Property | Default | Behavior |
| --- | --- | --- |
| `density` | 1 | 0.25–3× spatial frequency. At 1× the principal wavelength is about 108 logical pixels. |
| `amplitude` | 0.65 | 0–1. Couples refraction, focusing and surface tilt. Zero bypasses the effect and holds the phase. |
| `speed` | 1 | 0–3× time. Zero freezes the current shapes. Changing speed preserves their phase. |
| `floor_color` | `rgb(0,197,211)` | Sets the hue of the wave troughs. White paint gives gray troughs under neutral daylight. Also supplies the solid CPU fallback. |
| `full_resolution` | `false` | Half physical resolution on each output axis by default. Full resolution gives fine ripples more detail. |
| `identity` | `false` | Performance comparison. Samples the same capture at the same resolution, with the same wave-clock and light invalidations. |

## Rendering and visibility

The fixture wraps the standard `PoolFloorSurface` shader. Its single WGSL pass samples an unlit Rectangle once per output pixel. Four crossing swells bend a field of shorter ripples. Wave curvature approximates how overhead light concentrates on a flat receiver, with a finite-width fold and a shaded shoulder. Wave slopes also tilt the normal used by Pax’s scoped `scene_specular` helper. Point position and falloff, strip extent, light direction, color and intensity affect the surface reflection. The floor caustics use a fixed overhead sun. The shader uses no image asset, history buffer, readback, scene backdrop, or fluid simulation.

Only the water output is downsampled. Source capture and composition still use the renderer’s full-resolution attachments. The unlit source can be reused as the phase or a light changes. The panel mounts its clock only on GPU while visible with positive amplitude and speed. It updates the water at most 60 times per second, discards long timing gaps, and resumes from the held pose. The supplied web preview host suspends the Pax app when its tab is hidden. A website embedding its own Pax host can use the same [suspension API](../../../pax-docs/book/src/targets-build-deploy.md#suspend-an-embedded-web-app). Vello CPU renders the selected floor paint.

## Web release measurements

The November 25, 2026 release study ran in Chrome 158 on an Apple M2 Max using WebGPU/Metal. Each size and resolution used three interleaved water, plain-paint and identity runs of 65 animation callbacks apiece after warm-up. The water clock issued about 60 updates/sec on this 120 Hz display. Times below are WebGPU timestamp percentiles over rendered updates. The frame span runs from the first render pass to the last reported end timestamp, including capture/composition work. Plain paint issued zero steady-state render passes.

| Panel in CSS pixels | Output | Water pass p50 / p95 | Whole GPU span p50 / p95 | Identity GPU span p50 / p95 |
| --- | --- | --- | --- | --- |
| 1200 × 480, DPR 2 | Half | 0.52 / 0.72 ms | 1.97 / 2.44 ms | 1.38 / 1.77 ms |
| 1200 × 480, DPR 2 | Full | 1.64 / 2.82 ms | 3.15 / 4.72 ms | 1.18 / 2.24 ms |
| 390 × 640, emulated DPR 3 | Half | 0.20 / 15.07 ms | 0.66 / 19.10 ms | 0.52 / 19.66 ms |
| 390 × 640, emulated DPR 3 | Full | 0.52 / 16.71 ms | 1.05 / 22.03 ms | 0.52 / 15.68 ms |

The compact run had long-tail timing samples even in identity mode. A second five-run compact comparison that timestamped only the half-resolution fragment pass gave 0.33 / 0.79 ms for water and 0.07 / 0.78 ms for identity. These runs used the Mac GPU. Physical mobile performance remains unmeasured. A separate consumer with two 600 × 480 panels at DPR 2 produced a median whole GPU span of 0.52 ms and a p95 of 19.27 ms. All measured configurations reused warmed textures and pipelines.

At half resolution, the desktop capture allocated a 2432 × 960 retained source and scratch color target (8.91 MiB each), a 4× MSAA color target (35.63 MiB), 4× stencil (8.91 MiB) and the 1216 × 480 effect output (2.23 MiB). The compact capture used the same number of allocated texels with different dimensions. These are descriptor sizes before driver padding and other frame targets. The measured pass used 4,920 bytes of WGSL and one 16-byte parameter uniform. It adds no texture assets.

In matched optimized two-panel consumer builds, replacing plain Rectangles with `PoolFloor` added **68,085 raw WASM bytes / 19,984 gzip bytes** with `shaders` already enabled. A separate paint-only comparison measured **950,909 raw / 297,046 gzip bytes** for first enabling `shaders` in that minimal host. The website already enables that feature. Both comparisons used the same consumer, code-generation settings and Binaryen 117, with gzip level 6.
