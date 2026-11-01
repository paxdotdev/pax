# Animated Pax logo

This is the default `pax-cli create` starter. Its three source files contain
the scene, controls, choreography clock, and native entry point. It starts on
a dark background with `Radiance` enabled around the canonical logo,
including its obsidian-like letter treatment.

The light follows the flag's left-to-right unfurl handoffs during the original
1440 ms reveal. It then accelerates left, crosses the original path's 0%
endpoint (x = 160) at 1960 ms, and eases into −20% at 2100 ms. Over that final
140 ms of travel it dims from brightness 5 to **1.70**. The **12-second** idle
cycle spans **−20% to 120%** of the study's original path: (x = 28, y = 233)
to (x = 952, y = 177), returning smoothly with no rebound.

**Replay**, or clicking the logo, restarts the complete sequence. Dragging the
slider scrubs the original reveal and its light accompaniment and pauses the
clock; Replay resumes from the beginning. The parent owns replay and sets the
nested `AnimatedPaxLogo.click_to_replay=false` so the two clocks cannot diverge.

**Disable radiance** renders the plain logo. To change the scene's startup
flag, set `radiance_enabled: Property::new(false)` in `Example::default`.
When disabled, the shader container is not mounted and the clock stops after
the reveal. The scene also uses plain rendering on CPU; WGPU is required for
radiance. The reusable `AnimatedPaxLogo` itself remains a plain component.
The background toggle reverses the logo palette. Light mode uses an off-white
background and enables CMY beam dispersion at the shared 0.15 saturation
default. The contour rim is shelved as the separate `RimRefraction` shader.

The starter enables the `shaders` Cargo feature on `pax-kit`. Both this scene
and the separate `logo-radiance` tuning fixture use the shared standard-library
`Radiance`; the fixture retains its independent 18-second inspection
path and starts at brightness 2.20 on dark or 5 on light. See the [shader guide](../../pax-docs/book/src/compositing-effects.md#radiance)
for properties and limitations. Native parity and production performance are
not established by the web checks. System reduced-motion integration remains
deferred to PAX-1048.

`AnimatedPaxLogo` comes from `pax-std`, re-exported through `pax_kit`. Its geometry
and animation source live in `pax-std/src/drawing/pax_logo/`, outside the starter.
It keeps its original 920.72 × 463.95 coordinate system; scale an enclosing Group
to fit another layout. Bind `progress` from 0 to 1 to play or scrub the motion.
Clicking or tapping the logo restarts its 1440ms animation by default, including
while it is already playing. This uses the same `progress` property; a two-way
binding keeps parent controls such as a scrubber in sync. Replay does not add
automatic playback on mount.

The starter's `logo_light` timeline samples a parent-owned `playhead_ms`.
It drives `logo_progress`, the introductory light path, and brightness. Rust
advances that shared clock and derives the periodic idle position. Consumers
using the plain reusable logo can still bind and ease its progress directly:

```pax
<AnimatedPaxLogo width=920.72px height=463.95px
    background_mode=LogoBackgroundMode::Dark
    progress=bind:logo_progress click_to_replay=true/>
```

Use `use pax_kit::*;` in the consuming Rust component to bring `AnimatedPaxLogo`
and `LogoBackgroundMode` into scope; no dependency on this example is needed. `background_mode` describes the **surrounding
surface**, not the logo's signboard:

- `Light` (default): dark signboard/post, light letters and backing.
- `Dark`: light signboard/post, dark letters and backing.

Existing `fill` and `letter_fill` customizations remain the Light-mode palette.
Dark mode swaps those two fills consistently across the banner, counters, and
post. The mode does not paint the surrounding surface. The standalone example
has a background toggle alongside its existing replay/scrub controls, so both
palettes can be inspected during animation.

Set `click_to_replay=false` for a decorative still or when an external timeline
must exclusively control progress:

```pax
<AnimatedPaxLogo width=920.72px height=463.95px
    progress=1.0 click_to_replay=false/>
```

The flag suppresses both replay and the replay pointer cursor. It does not
disable an enclosing component's own click handlers. Use the same flag with an
externally supplied `progress={self.logo_progress}` to preserve that controller.

For a settled drawing without animation masks or click behavior, use
`StaticPaxLogo`. It shares the coordinate system and the `background_mode`,
`fill`, and `letter_fill` palette properties. Living Quilt uses it for its
settled card while retaining `AnimatedPaxLogo` during playback.
