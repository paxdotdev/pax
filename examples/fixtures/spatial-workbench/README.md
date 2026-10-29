# Spatial Workbench

A web-first testbed for live native controls on rotating flat Pax surfaces.
The teal card contains a Textbox, Slider, Button, and an independently tilting
child. An amber surface intersects it; geometric depth determines which part
is visible. Pose changes preserve the native controls' mounted state.

From the repository root:

```sh
pax-cli run --path examples/fixtures/spatial-workbench --target web
```

Use Pause motion before setting a precise pose. X/Y tilt and Z position affect
the teal card; Amber depth moves the crossing panel. Child tilt crosses the
coplanar boundary. Coplanar resets the card's three pose values; Back face
turns it away so its background hides its front content. Aperture clips both
panels through a stationary 2D opening. Reset pose restores the initial pose
without replacing typed text or the button counter. The controls move below
the stage in narrow windows, where the page scrolls.

This fixture requires the GPU renderer for depth composition. It uses the
implicit orthographic view, zero-thickness surfaces, and `rotate_x`, `rotate_y`,
`translate_z`; it does not introduce a scene wrapper or a camera component.
The shared renderer's Metal pixel regressions cover intersecting translucent
paint, projected clips, coplanar back-face reversal, and opacity cache changes.
The same fixture also runs on the macOS chassis. Its native textbox and button,
state preservation, tilted presentation, back-face masking, and aperture clipping
have been exercised there. iOS has not yet been validated.

Current limits: native surfaces still fade independently; native/native
intersections and intersections across independently composited scroller layers
are not general spatial composition. If an outside object interleaves between
noncoplanar members of a translucent group, opacity applies to separate
contiguous runs. Scene cameras, perspective, solid geometry, and transformed
lighting normals remain later checkpoints. See the book's
[spatial authoring guide](../../../pax-docs/book/src/layout-responsiveness.md#rotating-flat-surfaces-in-3d).
