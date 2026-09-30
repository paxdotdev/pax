# Animated Pax logo

`AnimatedPaxLogo` is a reusable component exported by this example crate. It
keeps its original 920.72 × 463.95 coordinate system; scale an enclosing Group
to fit another layout. Bind `progress` from 0 to 1 to play or scrub the motion.
Clicking or tapping the logo restarts its 1440ms animation by default, including
while it is already playing. This uses the same `progress` property; a two-way
binding keeps parent controls such as a scrubber in sync. Replay does not add
automatic playback on mount.

```pax
<AnimatedPaxLogo width=920.72px height=463.95px
    background_mode=LogoBackgroundMode::Dark
    progress=bind:logo_progress click_to_replay=true/>
```

Import both `AnimatedPaxLogo` and `LogoBackgroundMode` from `pax_logo` in the
consuming Rust component. `background_mode` describes the **surrounding
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
