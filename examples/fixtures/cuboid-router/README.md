# Cuboid Router

This fixture enables `pax-kit/3d` in its Cargo dependency; 3D is off by default
for other applications.

Live route content on a freely turning cube, plus a small solid on each route
with separately authored front, right and top content.

```sh
pax-cli run --path examples/fixtures/cuboid-router --target web
```

Add `--release` when evaluating animation performance to run the optimized,
baked cartridge. The default debug build is intended for development.

Choose a direction, edit the native textbox or press a face button, then choose
**Next route**. Navigate again during the turn, change axes, or use browser
Back/Forward. **Speed** selects 600, 1200, or 2400ms for the next navigation.
The destination samples direction and duration once; changing controls during
motion does not separate the faces.

Sibling `RouteCuboid` shells coordinate one rotation. Interruptions preserve
its current pose and angular velocity, then ease to the latest destination.
Several routes can remain live on the six reusable faces. Returning to a
participating route preserves its textbox and button state. A new route uses
a hidden face; only the selected route remains after settling. Later visits
create fresh instances. There is no persistent route cache, screenshot, cloned
control tree, or application timing guard.

The normalized cube is projected to fit the viewport. Content is undistorted
at rest; intermediate shapes stretch with the viewport's aspect ratio. Resize
during a turn to exercise the shared geometry. An outer Frame clips the
silhouette, and each shell closes the aperture of its back-facing or edge-on
content without unmounting it. This shell requires the default orthographic
camera; custom camera projections and spatially rotated ancestors are not supported.

The real Router handles `/`, `/notes` and the `/studio` fallback. Settled
browser Back/Forward keeps the authored direction; during motion it can return
to an existing face. Focused textboxes and pointer ownership follow mounted
instance lifetime. Every route has a native textbox, stateful button, and a
small cuboid with its own slider. Rapid navigation can keep all three nested
cubes live together, exercising nested geometry and native-control occlusion.
Each small cube appears when its page is at least 740px wide or 680px tall.
Each side is painted once: `fill=[]` disables the cuboid's shared paint, and six
authored face backgrounds close the solid. This avoids drawing a second opaque surface
behind the three colored faces.

GPU rendering is required for geometric depth. Native/native and distinct
Scroller-layer intersections retain platform limitations. This fixture has no
authored lights; its face colors are explicit paints. The Light / Space fixture
demonstrates spatially lit materials. See the
[camera guide](../../../pax-docs/book/src/layout-responsiveness.md#controlling-the-root-camera)
for camera support outside this route shell.

`RouteCuboid` is exported by `pax-kit`, alongside `Route`, `RouteCard`, and
`RouteModal`; this fixture needs no application-defined route shell. See
[the routing guide](../../../pax-docs/book/src/routing.md#cuboid-route-transitions)
for face selection, settings, and lifetime details.
