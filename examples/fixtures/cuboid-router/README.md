# Cuboid Router

Live route content rotating on adjacent Cuboid surfaces, plus a small solid
with separately authored front, right and top faces.

```sh
pax-cli run --path examples/fixtures/cuboid-router --target web
```

Choose a turn direction, edit the native textbox or press a face button, then
choose **Next route**. The outgoing route remains mounted through its 1.2s
exit; the incoming route mounts on the adjacent surface. Each branch's
standard `RouteCuboid` shell contributes its live front face. The two shells share the
same geometric center/depth, with rotation angles 90 degrees apart, so they
form adjacent faces of one turning cuboid. Their paints are empty; the route
contents supply the faces' opaque backgrounds. No route screenshot or cloned
control tree is used.

Horizontal transitions use viewport width as depth; vertical transitions use
height. Both endpoints fit the viewport. The route aperture clips the solid's
intermediate silhouette as it rotates. The view is currently orthographic.

The real Router handles `/`, `/notes` and the `/studio` fallback, including
browser history. Ordinary route lifetime applies: an exiting branch unmounts
when its transition finishes, and a later visit creates a fresh instance.
This example does not add persistent route caching. Direction controls are
held during a navigation initiated by the example so both faces share one
turn direction. The small cuboid's slider independently exercises side
geometry and native-control occlusion.

GPU rendering is required for geometric depth. Native/native and distinct
Scroller-layer intersections retain platform limitations. Material lighting
still uses the existing planar lighting contract; face colors here are
explicit paints, not simulated lighting. Cameras and fixed overlays are
separate work.

`RouteCuboid` is exported by `pax-kit`, alongside `Route`, `RouteCard`, and
`RouteModal`; this fixture needs no application-defined route shell. Siblings
share a `direction` binding and `duration=1200ms`. The standard type derives
its pivot, depth, and face placement and applies `InOutQuad` easing. The
example holds direction controls during its own navigation to keep the pair
coordinated. Browser Back/Forward uses the currently selected direction.
