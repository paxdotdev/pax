# Animated Pax logo

This is the default `pax-cli create` starter. Its `src/` contains only `lib.pax`
(the scene and controls), `lib.rs` (state and handlers), and `main.rs` (native
entry point). A slider scrubs the animation position; Replay and clicking the
logo restart it; a button switches between light and dark backgrounds.

`AnimatedPaxLogo` comes from `pax-std`, re-exported through `pax_kit`. Its geometry
and animation source live in `pax-std/src/drawing/pax_logo/`, outside the starter.
It keeps its original 920.72 × 463.95 coordinate system; scale an enclosing Group
to fit another layout. Bind `progress` from 0 to 1 to play or scrub the motion.
Clicking or tapping the logo restarts its 1440ms animation by default, including
while it is already playing. This uses the same `progress` property; a two-way
binding keeps parent controls such as a scrubber in sync. Replay does not add
automatic playback on mount.

The replay handler resets and eases the same local progress view. Shared
`Property::set` publications reach that view at a UI settlement boundary, so
publishing a reset immediately before starting a local transition would use the
old start value. External replay controls should likewise use
`let progress = self.logo_progress.local(); progress.set(0.0); progress.ease_to(...)`
after cancelling any active transition.

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
