# Radiant Aperture

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
of light and catch a cool sheen as the source moves. There is no added outline
or stroke.

**Pause light** freezes the light without freezing the original logo reveal.
Dragging **Light path**, **Light X**, or **Light Y** also pauses playback.
**Resume light** returns to the authored path at the current phase; manual X/Y
positions are inspection poses, not edits to the path. **Show light position**
places a blue marker at the light's projected XY position, outside the effect
capture. **Show plain (A/B)** switches the treatment off in place.
**Reset study** restores the light path and effect controls and resumes playback.
**Replay stencil** and its slider retain the original 1440 ms reveal.

| Control | Initial value |
| --- | --- |
| Light X / Y | 820 / 185 local px |
| Light depth (behind logo) | 340 local px |
| Brightness | 5 |
| Reach | 320 local px |
| Falloff | 1 |
| Transmission | 0.16 |
| Glass sheen | 0.55 |

Position, depth, and reach use the canonical 920.72 × 463.95 logo coordinate
system. Bringing the light closer widens the fan; moving it farther away makes
the projected rays more parallel. Brightness scales the accumulated light,
reach caps its travel, and falloff attenuates it with distance. Scaling the
logo's enclosing Group scales the whole treatment.

**Transmission** (0–0.5) controls how much light passes through dark source
coverage. It affects both the rays projected from the letters and the beam
field visible inside them. **Glass sheen** (0–1) controls a soft moving highlight
and narrow contour highlights tied to the same light position. Set both to
zero to restore the previous solid-letter treatment while keeping exterior
rays. **Show plain (A/B)** disables the entire treatment, including glass.
**Reset study** restores transmission 0.16 and sheen 0.55.

`RadiantAperture` is an experimental standard-library primitive exported through
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
unchanged. Dark coverage attenuates the projected volume and applies a small
refraction offset and a stylized polished highlight. Only a narrow, fixed
contour footprint supplies edge normals; there is no nearest-edge selection
or face-wide bevel. All glass shading is clipped to the current source coverage.

Transmission concerns this effect's own captured light field. It does not
refract arbitrary scene content behind the container, and it leaves the
source alpha coverage intact. The cool absorption tint and highlight shape
are art-directed constants, not a physically calibrated obsidian material.

The study enables the `shaders` feature and has been checked on web/WGPU in
debug and baked release builds. CPU rendering retains the plain children.
Native descendants are unsupported inside the capture. This fixture does not
change hit regions, native occlusion masks, or the standard logo API. Native
rendering parity and production performance have not been qualified.

The main `pax-logo` example uses the same shader with default-on, disableable
radiance and synchronized unfurl/return choreography, followed by brightness
1.70 and a 12-second idle loop. This fixture keeps its 18-second independent
path and brightness 5 for material inspection.

Light motion pauses when the stage leaves the viewport or when the plain A/B
state is shown. Operating-system reduced-motion integration remains deferred
to the engine capability tracked in PAX-1048.
