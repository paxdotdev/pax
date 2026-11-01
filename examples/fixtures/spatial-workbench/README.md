# Spatial Workbench

This fixture enables `pax-kit/3d` in its Cargo dependency; 3D is off by default
for other applications.

A web-first testbed for live native controls on rotating flat Pax surfaces.
The teal card contains a Textbox, Slider, Button, and an independently tilting
child. An amber surface intersects it; geometric depth determines which part
is visible. The closer amber paint hides the covered teal controls, and closer
teal paint hides the covered portion of “Through the plane.” This also applies
in perspective mode. Pose changes preserve the native controls' mounted state.

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
root camera, zero-thickness surfaces, and `rotate_x`, `rotate_y`,
`z`, including reactive `position=[x, y, z]` placement; it does not introduce
a scene wrapper or a camera component.
The shared renderer's Metal pixel regressions cover intersecting translucent
paint, projected clips, coplanar back-face reversal, and opacity cache changes.
The same fixture also runs on the macOS chassis. Its native textbox and button,
state preservation, tilted presentation, back-face masking, and aperture clipping
have been exercised there. The experimental perspective adapter retains AppKit
editing, projects focus feedback and text geometry, and masks live controls.
iOS has not yet been validated.

Current limits: native surfaces still fade independently; native/native
intersections and intersections across independently composited scroller layers
are not general spatial composition. GPU group fades preserve interleaving within one logical canvas layer using
a bounded compositor (up to eight scene views per tile); native transparency
stacking remains approximate. Multiple cameras remain deferred. Perspective is experimental.
Opaque `Ellipsoid` solids support non-default orthographic and perspective
cameras, near/far clipping, native-control occlusion, and pointer depth. GPU
effects and vector mask sources preserve camera projection, and lighting follows
world normals and the active camera.
Full macOS assistive-technology workflows and native menus/popovers remain
unqualified under perspective. CPU renderers diagnose
perspective as unsupported and display the default orthographic view. See the book's
[spatial authoring guide](../../../pax-docs/book/src/layout-responsiveness.md#rotating-flat-surfaces-in-3d).

The root `@camera` block binds reactive position and zoom. **Pan camera** shifts
the view by 80 logical pixels; **Zoom out** moves between 1× and 0.8×. Both ease
in and out over 450 ms. Clicking again during motion reverses from the current
value; Reset pose cancels camera motion and restores the default view. Type in
the card, change the view, and press its button to check preserved native state
and input alignment. Camera motion leaves local scroll-content sizing unchanged.

**Switch to perspective** activates a 60° vertical field of view. The initial
camera distance is `viewport height / (2 × tan(30°))`, so the Z=0 plane starts at
its orthographic size while nearer and farther surfaces change size. **Vertical
field of view** widens the lens; **Dolly back** moves the camera farther away.
These two sliders affect perspective mode. The controls and the Scroller are part
of the same world, so changing the lens also projects them; scroll inside the
projected viewport. Reset pose restores orthographic mode and the initial lens.

**Open tile / near-plane lab** switches to a 4096×4096 scrolling grid with a
tilted nested viewport and a live editor. **Jump to far tiles** moves to (3072,
3072) and brings the same editor along; return to check that its text and nested
scroll position survive. **Animate zoom** alternates 1× and 0.65×. **Cross near
plane** brings the tilted panel across the camera's clipping plane; restoring
it must recover its canvas and native content without remounting. These paths,
including nested wheel scrolling and editing after recovery, have been exercised
on GPU web and macOS. This mode is a qualification fixture, not a claim that
arbitrary scroller-layer intersections support spatial composition.

The **Show solid volume** control replaces the amber card with a lit `Ellipsoid`.
Pan, zoom, switch projection, adjust Amber depth, and toggle the aperture to compare
its curved intersection with the live textbox, slider, and button. The camera
moves the same retained mesh; controls keep their editing state as the volume
passes in front of them. Near/far cuts remove surface triangles without adding caps.
