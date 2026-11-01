# Radiance

A moving point-light projection through the canonical animated Pax logo. Run
from the repository root with the current CLI:

```sh
pax-cli run --path examples/fixtures/logo-radiance --target web
```

The light starts behind the right side of the mark, eases left, then returns
in an 18-second cycle. All projected rays share the same source position, so
the fan changes coherently. The full-resolution logo sits in front of the
projected light, preserving its exterior edges and letter silhouettes. The dark
letters are mostly opaque obsidian-like glass: they transmit a restrained amount
of light and catch a vertical sheen as the light moves. The logo has no
contour rim; that study is preserved independently as `RimRefraction` in pax-std.

**Pause light** freezes the light without freezing the original logo reveal.
Dragging **Light path**, **Light X**, or **Light Y** also pauses playback.
**Resume light** returns to the authored path at the current phase; manual X/Y
positions are inspection poses, not edits to the path. **Show light position**
places a blue marker at the light's projected XY position, outside the effect
capture. **Show plain (A/B)** switches the treatment off in place.
**Black on light** / **White on dark** switches the logo, shader palette, and
page background together without changing the light path. Each mode remembers
its own brightness (initially 2.20 on dark and 5 on light); other material
controls are shared.
The switch repaints immediately, including while paused; **Replay stencil** is
not needed to refresh the palette.
Light mode uses an off-white page and a continuous CMY prism treatment across
the projected fan. Dark mode retains the white light. **White light (A/B)** and
**CMY light (A/B)** compare the light-mode treatment without changing the glass
settings. **Reset study** restores the light path and effect controls and
resumes playback, keeping the selected white/black mode. **Replay stencil** and
its slider retain the original 1440 ms reveal.

**Ray dispersion** (0–1, initially 1) separates three copies of the projected
field in cyan, magenta, and yellow around the moving light. Their transverse
spread grows with distance from the light, capped at 36 local pixels per side.
The bands follow the current projection continuously, including during reveal;
there is no particle history. **Ray saturation** (0–1, initially 0.25) controls the
chromatic strength independently of brightness. Zero dispersion or saturation
restores the white fan.

The shared white component is removed from overlapping CMY
lobes before applying saturation; equal lobes converge smoothly to neutral.
This keeps bright areas colorful on the off-white page. The source silhouette
stays at full resolution and is never displaced. The glass samples the same
dispersed field for transmission; its vertical polished sheen is unchanged.
This is a bounded resampling approximation, not a spectral ray tracer. It uses
the existing two passes: the 96-sample projection is unchanged, while the
composite samples three spectral offsets instead of one. The fixed capture
outset is 416 local pixels when the prism is active, otherwise 376; moving the
light and adjusting positive dispersion do not change that allocation envelope.

The projection skips attenuation work for transparent stencil samples; the
composite skips exterior light hidden by opaque source coverage and skips
refraction when transmission is zero. These shortcuts preserve the 96 sample
positions, full-resolution antialiased silhouette, and material defaults.
Lowering nonzero brightness does not lower the sampling cost. Use **Pause light**
to stop light-driven redraws when inspecting a still pose.

The reusable `Radiance` defaults to `prism_dispersion=0` and
`prism_saturation=0.25`; the fixture enables dispersion 1 in light mode. The
particle emission API, simulation, and controls have been removed.

| Control | Initial value |
| --- | --- |
| Light X / Y | 820 / 185 local px |
| Light depth (behind logo) | 340 local px |
| Brightness (dark / light) | 2.20 / 5 |
| Reach | 190 local px |
| Falloff | 1 |
| Transmission | 0.16 |
| Glass sheen | 0.55 |
| Ray dispersion (light mode) | 1 |
| Ray saturation | 0.25 |

Position, depth, and reach use the canonical 920.72 × 463.95 logo coordinate
system. Bringing the light closer widens the fan; moving it farther away makes
the projected rays more parallel. Brightness scales the accumulated light,
reach caps its travel, and falloff attenuates it with distance. Scaling the
logo's enclosing Group scales the whole treatment.

**Transmission** (0–0.5) controls how much light passes through dark source
coverage. It affects both the rays projected from the letters and the beam
field visible inside them. **Glass sheen** (0–1) controls a soft vertical band
centered at the light's X position. Set transmission and sheen to zero for
solid letters with exterior rays. **Show plain (A/B)** disables the entire
treatment, including glass. **Reset study** restores the saved defaults above;
the material values come from `Radiance::default()` so the library and harness
stay aligned. The main logo and website retain their choreographed brightness
1.70 ambient idle; this inspection harness uses the mode-specific values above.

`Radiance` is an experimental standard-library primitive exported through
`pax_std::shaders` and `pax_kit::*` with the `shaders` feature, alongside `Corona`. It uses the shader-container extension point with
two passes: a half-resolution projection and a full-resolution composite.
The projector integrates expanded copies of the current rendered aperture
about a shared light position through a fading volume. It samples at uniform
screen distances to avoid skipping narrow strokes far from the light. It has
no waveform, random reseeding, or simulation history.

This is a 2.5D approximation of light scattering, not a full 3D refraction or
caustics simulation. The source must use the declared distinct opaque solid
foreground/background colors. Its masks, opacity, and partial reveal are
captured; there is no hidden full-logo stencil. Foreground coverage stays
unchanged. The `light_color` property defaults to white independently of the
two stencil colors; its alpha scales light strength. Background-colored glass
coverage attenuates the projected volume and applies a small
refraction offset and stylized polished highlights. Only a narrow, bounded
contour footprint supplies edge normals; there is no nearest-edge selection
or face-wide bevel. All glass shading is clipped to the current source coverage.

Transmission concerns this effect's own captured light field. It does not
refract arbitrary scene content behind the container, and it leaves the
source alpha coverage intact. The cool absorption tint and prismatic bands
are art-directed, not a physically calibrated obsidian material.

The study enables the `shaders` feature and has been checked on web/WGPU in
debug and baked release builds. CPU rendering retains the plain children.
Native descendants are unsupported inside the capture. This fixture does not
change hit regions, native occlusion masks, or the standard logo API. Native
rendering parity and production performance have not been qualified.

The main `pax-logo` example uses the same shader with default-on, disableable
radiance and synchronized unfurl/return choreography, followed by brightness
1.70 and a 12-second idle loop. This fixture keeps its 18-second independent
path and mode-specific brightness for material inspection.

Light motion pauses when the stage leaves the viewport or when the plain A/B
state is shown. Operating-system reduced-motion integration remains deferred
to the engine capability tracked in PAX-1048.
