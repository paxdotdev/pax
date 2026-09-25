# Animated Pax logo

`AnimatedPaxLogo` is a reusable component exported by this example crate. It
keeps its original 920.72 × 463.95 coordinate system; scale an enclosing Group
to fit another layout. Bind `progress` from 0 to 1 to play or scrub the motion.

```pax
<AnimatedPaxLogo width=920.72px height=463.95px
    background_mode=LogoBackgroundMode::Dark progress={self.logo_progress}/>
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
