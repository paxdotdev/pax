# Logo Corona

A live monochrome corona study around the canonical Pax logo. Run from the
repository root with the current CLI:

```sh
pax-cli run --path examples/fixtures/logo-corona --target web
```

The fixture starts with the settled logo and a moving corona. **Replay stencil**
and the reveal slider drive the original 1440 ms choreography. **Pause corona**
freezes only the surface field; the reveal can still play or scrub independently.
**Show plain (A/B)** retains the same layout for comparison. **Large view** gives
the treated mark more room; narrow viewports show one mark with the A/B control.

**Black on light** reverses the source palette and emitted color. The emitting
shape is derived from the two authored source colors, independently of their
luminance. This experiment supports opaque solid foreground/background colors,
not arbitrary gradient or multicolor source content.

The controls adjust emission opacity, maximum reach, flare-length variation,
distance falloff, speed, and tonal depth within the core. Reach is measured in
the logo's canonical local pixels; scaling its enclosing Group scales the entire
treatment. The two presets offer different starting points for those controls.

`Corona` is an example-owned shader primitive using Pax's existing experimental
shader-container API. Its first pass constructs a contour-following emission
field at half resolution; its second pass retains the original full-resolution
core and antialiasing. Both use the currently rendered stencil, including its
masks and partial reveals. The emitting aperture is exactly the foreground-colored
portion of the rendered logo. Rays spread from its internal boundaries into the
painted letter regions as well as beyond the outside silhouette; both use the
same reach, field, and falloff. The emitting core retains its full-resolution edge.
The source field is periodic and deterministic, without simulation history.

The `shaders` feature is enabled only for this fixture. WGPU applies the effect;
CPU rendering preserves the ordinary logo. Native controls remain outside the
capture. Shader-created coverage does not change native punch-through masks or
hit regions. Keep text and controls clear of the emission bounds.

The owner stops phase advancement when the stage leaves the viewport, when
paused, or when showing the plain A/B state. System reduced-motion integration
awaits the engine capability tracked in PAX-1048; this fixture does not claim to
read the operating system's preference. The ordinary logo components and the
default CLI starter are unchanged.
