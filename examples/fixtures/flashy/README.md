# Flashy

A focused study composing `Material::frosted` with the reusable `Flashy` shader: vector lettering above a broad
surface of small overlapping discs. The glints stay attached to the
painted source while one positioned light crosses both specimens in a 14-second
round trip. Broad frosted sheen and narrow facet reflections use the same light
direction and falloff. The facet phase stays fixed: moving the light alone
triggers both responses. The starburst rays share a fixed direction in the
effect's local viewing frame. The source geometry stays at full resolution.

From the repository root, use the current workspace CLI:

```sh
cargo run -p pax-cli -- run --path examples/fixtures/flashy --target web
```

Pause or drag **Light sweep** to hold the light. **Show plain (A/B)** bypasses the
material sheen and flare effect without changing the artwork. **Outline lettering** switches the filled
vector word to a 3-pixel stroke, useful for judging sparse alpha coverage.
**Light background** checks contrast on a pale surface. **Seed** changes the
spatial distribution; **Reset** restores the material defaults and resumes.
Spacing, scene light intensity, material sheen, and atmospheric flare are shared by both specimens.
**Atmospheric flare** ranges from 0 to 5: zero leaves pinpoint cores, 0–1 adds
rays and haze, and 1–5 enlarges their reach up to five times the size at 1.
Larger flares keep the same anchors and lighting but require more GPU work.
Motion pauses when the stage leaves the viewport or the plain comparison is
selected. CPU retains plain artwork and does not advance the motion clock.

The component is exported by `pax_kit::*` and `pax_std::shaders` when the
`shaders` Cargo feature is enabled. See [Compositing & Effects](../../../pax-docs/book/src/compositing-effects.md#flashy)
for its API, capture restrictions, and bounded GPU cost. The first qualification
target is web/WGPU; native parity and production performance are not established
by this fixture. Native text and controls stay outside the captures.

To integrate into another component, wrap its GPU-rendered artwork with:

```pax
<LightFrame width=400px height=180px>
    <Flashy width=100% height=100% spacing=18 brightness=1.4 flare=0.65 seed=17>
        <Ellipse x=50% y=50% width=320px height=120px
            fill={paint: rgb(50, 65, 95) material: {Material::frosted(0.8)}}/>
    </Flashy>
    <LightSource x={(self.light_x)px} y=90px z=180px width=1px height=1px
        anchor=0px radius=1200px intensity=2 color=WHITE/>
    <AmbientLight intensity=1 color=WHITE/>
</LightFrame>
```

The clock drives `light_x = 460 - 720 * cos(turns * TAU)` in the common
920×460 stage, at Y=230 and authored depth 220. Depth and attenuation radius
are world-space distances, so the template scales them with the responsive stage. One scoped `LightSource` illuminates both
specimens; their materials render first, then Flashy adds glints and flares.
No per-container coordinate subtraction is needed. Pausing freezes both
responses. Flashy's optional facet `phase` stays zero. Spacing uses the authored
coordinate system; scaling the stage scales the whole treatment. Outer clipping
can cut off flares. Frosted sheen shades the surface without sampling or blurring
a backdrop. All artwork here is coplanar with its shader container.

`src/lettering.rs` contains normalized vector outlines of “flashy” traced from
the repository's `pax-website/assets/fonts/Manrope.ttf` at weight 800. The
OFL license accompanies this derived artwork. No font parser runs in the app.
To regenerate, install `fonttools` and `skia-pathops` in a Python environment,
run `python tools/trace-lettering.py` from this directory, then run `rustfmt
--edition 2021 src/lettering.rs`. The generator unions overlapping font contours
so the stroke-only view follows the actual silhouette.
