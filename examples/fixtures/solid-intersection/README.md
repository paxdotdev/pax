# Surface / Solid

Two opaque `Ellipsoid` instances and a tilted live panel share one fading Group.
A translucent blue planar band belongs to the outside scene and can move between
the group's members. The same retained meshes supply GPU depth, native occlusion
contours, volume bounds, and pointer depth. The textbox and button remain mounted
through motion and fades.

```sh
pax-cli run --path examples/fixtures/solid-intersection --target web
pax-cli run --path examples/fixtures/solid-intersection --target macos
```

This fixture enables the default-off `pax-kit/3d` and `pax-kit/shaders` features.
Use the GPU renderer.

- **Behind / Through / In front** select reproducible intersection states.
- **Play motion** sweeps depth; pause before manually adjusting the depth slider.
- **Fade / Restore** animates opacity; **Group opacity** reaches the exact
  zero and one endpoints. Hidden rear members must not show through the front.
- **Outside depth** moves the blue band through the assembly independently.
- **Panel rotation** changes the intersection plane without remounting controls.
- Type into the textbox, move the solid, then expose the textbox again to check
  preserved content. Click exposed portions of the panel button or the solid to
  exercise their separate counters. Hidden panel regions must not activate it.
- Scroll the page to check canvas/native alignment and retained tile replay.
- **Animate opacity** changes only the group's fade when motion is paused.
  **Hide panel UI** removes native panel members while retaining its GPU surface;
  **Remove solids** unmounts both ellipsoids. Use these controls to compare work
  and check resource retirement, then mount them again.
- **Scroll study** opens an editorial page using scroll-driven poses, lighting,
  and fading. Edit its note and press its button, scroll to the lower cuboid,
  resize, and return to check that the native controls preserve their state.
  The page switcher scrolls with the heading in both views; return to the top
  to switch pages.
- **Mask study** reveals a striped gradient through two rotating Ellipsoids used
  as a detached alpha source. Adjust rotation, feather and shared opacity; scroll
  through tile edges, then remove and remount the source. The overlapping shapes
  share the group's opacity rather than accumulating it. Controls stay outside
  the capture, and an empty source hides the reveal completely.
- **Show filter** switches that study to a blurred solid/plane assembly. Internal
  depth resolves before filtering; the result sits on the tilted filter container
  plane. The gold band remains outside and crosses that plane. The same controls
  adjust pose, blur radius, output opacity, and source lifetime.

For web CPU measurements, append `?pax_scroll_perf&pax_surface_trace` to the URL,
open **Surface trace snapshot**, then choose **Profile 2 seconds**. The bounded
aggregate survives event-ring rollover. Compare stationary, opacity-only,
moving solids, and moving solids with panel UI at the same viewport and build
mode. Phase durations measure CPU work, not GPU time or presented frames; see
[the profiling guide](../../../pax-docs/book/src/developer-workflow.md#profile-a-web-scene).

The solid uses a shared, immutable 64 × 32 sphere mesh. Width, height, depth, and
pose transform that mesh; motion does not upload new vertex/index data. Each
immutable world pose retains up to four camera/chart projections, each with its
silhouette. Native and GPU solid/plane coverage share a runtime-wide cache bounded
by 2,048 entries and 4 MiB, including all GPU tile charts. Paint-only updates reuse the pose; motion replaces it and
removing the solid releases its contours. Hard
projected clips and whole-group fades are supported. Exact spatial fades use up
to eight scene views per composition domain in each canvas tile (one assembly
needs two); source captures are separate domains. Overflow reports a
warning and leaves intermediate GPU fades opaque, while zero remains hidden.
Opacity-only changes reuse captures. Opaque solids can supply detached alpha
sources; their captures use the same depth/group compositor in an isolated scene.
Shader containers resolve internal solid depth and present a planar result.
Alpha-masked solid content, translucent
solid materials and arbitrary concave meshes are not supported. Root cameras,
including experimental GPU perspective, apply to the solids and their fades.
Native transparency stacking remains approximate; this checkpoint does not
rasterize native text or controls into those scene views. Native/scroller layers
keep their existing stacking limits; intersections resolve within one logical
canvas layer.
