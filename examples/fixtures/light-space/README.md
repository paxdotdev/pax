# Light / Space

A website-oriented study of world-space lighting on moving, live Pax surfaces.
All six faces of the gold Cuboid share one material. Their shading follows the
light and their actual orientation; the front face contains a live textbox and
button. Pause motion to inspect the pose, change the key light with the slider,
and use Fade solid to exercise group opacity. Scroll to tilt the solid and turn
the second study's lamp rig. Its light and marker inherit the same parent pose.

```sh
pax-cli run --path examples/fixtures/light-space --target web
```

The viewport remains orthographic. Material response and geometric depth require
the GPU renderer; CPU rendering keeps ordinary unlit paints. Native controls,
native Text, and bitmap Images retain their platform/unlit appearance. There are
no shadow maps or texture-map materials. Fading a spatial group uses one retained
capture when its surfaces form an uninterrupted paint span; outside geometry
interleaved through that group still takes the more expensive region path.
Arbitrary intersections between distinct native or Scroller layers retain the
current platform limits.

The clock drives a declarative timeline and can be paused. Scrolling drives the
other pose changes in ordinary expressions. No screenshots or route-specific
transition machinery are involved. The light's `z` is local, radius is in world
pixels, and shading stays independent of tile and capture origins.
