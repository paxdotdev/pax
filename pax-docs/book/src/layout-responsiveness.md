<a id="layout--responsiveness"></a>

# Layout and Responsiveness
<!-- summary: Understand parent-local geometry, percentage alignment, containers, content-driven size, and responsive composition. -->
<!-- tags: layout, responsive, positioning, anchors, autosize, padding -->

The same note card might appear in a narrow phone view, beside another card
on a desktop, or inside a small inspector panel. Its content needs an area
to occupy and a few rules for adapting to that area.

Pax gives each node a local coordinate space. Sizes, positions, and transforms
describe its relationship to its container; layout containers can assign
smaller areas to their children. This chapter builds that spatial model,
then uses it to arrange the cards from
[Components and Composition](components-composition.md#make-a-reusable-component)
at different widths. [Templates](template-language.md) covers the syntax and
settings rules used here.

## The coordinate and size model

A node has a width and height, and a transform that places those bounds in
its parent's space. The local origin is the upper-left corner; x increases
to the right and y increases downward. Children use that local space, even
when the whole group is moved or rotated.

The relevant area is usually the parent's content area. Padding reduces that
area. A Stacker assigns each child a cell, which becomes the area the child
uses for its own layout.

Use `px` for a concrete length and `%` for a proportion of the relevant axis.
These are logical layout units; `1px` need not equal one physical display
pixel on a high-density screen. Width percentages use available width, and
height percentages use available height.

For example:

```pax
<Group x=24px y=24px width={100% - 48px} height=160px>
    <Rectangle x=16px y=16px width={100% - 32px} height=128px
        fill=rgb(246, 241, 230) corner_radius=12
    />
</Group>
```

The outer Group leaves 24 pixels on each side of its parent. The Rectangle
then leaves 16 pixels on each side of the Group. Its percentage width refers
to the Group, not the app window. Mixed-unit expressions retain those two
parts: `100% - 32px` means “the available width minus 32 pixels.”

Ordinary omitted dimensions fill the available axis. Content measurement can
supply omitted dimensions for text, native controls, and autosized containers,
so make important constraints explicit. A fixed-height card should declare
that height; a wrapping paragraph usually needs a width and room to grow.
Read [Text, Fonts, and Images](text-fonts-images.md) for text measurement.

### Position and alignment

Percentage positions also supply a default anchor on the node's own bounds.
That makes `x=50%` a convenient way to center an element, and `x=100%` a way
to align its right edge. Pixel positions default to a zero anchor.

For an untransformed 100-pixel-wide child in a 400-pixel-wide area:

| Position | Default anchor | Child's left edge |
| --- | --- | --- |
| `x=0%` | Left edge | 0px |
| `x=50%` | Center | 150px |
| `x=100%` | Right edge | 300px |
| `x=200px` | Left edge | 200px |

Within the usual zero-to-100-percent range, the default behavior can be read
as a percentage of the remaining travel: `left = (parent width - child width)
* percentage`. The same rule applies vertically.

```pax
<Group width=400px height=160px>
    <Rectangle x=50% y=50% width=100px height=40px
        fill=rgb(55, 120, 90)
    />
</Group>
```

This centers the rectangle on both axes. To align an element with a pixel
inset from the far edge, combine units:

```pax
<Rectangle x={100% - 24px} y=24px width=100px height=40px
    fill=rgb(55, 120, 90)
/>
```

The percentage part gives it a right-edge anchor, and the pixel part moves
that edge 24 pixels inward. It does not place the left edge 24 pixels from
the parent's right edge.

### Explicit anchors

Set `anchor_x` or `anchor_y` when you want a particular point on the element
to meet a particular point in the parent:

```pax
<Rectangle x=50% y=24px anchor_x=0px width=100px height=40px
    fill=rgb(55, 120, 90)
/>
```

Here the left edge begins halfway across the parent. An explicit anchor
takes precedence over the anchor inferred from position. Anchors are measured
against the node's own bounds: `anchor_x=50%` selects its center, while
`anchor_x=12px` selects a point 12 pixels from its left edge.

Be deliberate with anchors when placing content beyond an edge. For example,
`y={100% + 8px} anchor_y=0px` places the top of a child eight pixels below its
parent. The explicit anchor is important to that relationship.

## Choose a container

A Group establishes a shared coordinate space. It does not automatically
arrange siblings into rows. Use it for deliberately positioned content,
layered backgrounds, badges, and pieces that should transform together.

A Frame provides similar grouping with clipping at its bounds by default.
Its `corner_radius` rounds the clipping boundary:

```pax
<Frame width=240px height=100px corner_radius=12>
    <Rectangle x=200px y=20px width=120px height=60px
        fill=rgb(55, 120, 90)
    />
    <Rectangle width=100% height=100% fill=rgb(246, 241, 230) />
</Frame>
```

Only the portion of the green rectangle inside the Frame is visible. A Group
of the same size would allow it to extend beyond the group's bounds.
[Compositing](compositing-effects.md) explains clipping and native/rendered
content in more detail.

### Rows and columns with Stacker

A Stacker assigns one cell to each participating child. Its default direction
is vertical; use `StackerDirection::Horizontal` for a row.

```pax
<Stacker width=100% height=160px
    direction=StackerDirection::Horizontal gutter=16px
>
    <NoteCard width=100% height=100% title="Field notes" progress=0.25 />
    <NoteCard width=100% height=100% title="Sketchbook" progress=0.75 />
</Stacker>
```

The gutter separates the two cells. By default, they divide the remaining
main-axis space equally: in a 400-pixel-wide row with a 16-pixel gutter,
each gets 192 pixels. Each card's `width=100%` fills its own cell. It does
not request the entire row's width.

A child still has its own layout within its cell. A smaller child can use
`x=50%` or `y=50%` to align itself there. Nest horizontal and vertical
Stackers for more complex arrangements; a Stacker forms a single row or
column and does not automatically wrap.

To assign different cell sizes, supply `sizes` along the extending axis:

```pax
<Stacker width=400px height=160px
    direction=StackerDirection::Horizontal gutter=16px
    sizes=[Some(120px), None]
>
    <Rectangle width=100% height=100% fill=rgb(220, 216, 206) />
    <Rectangle width=100% height=100% fill=rgb(55, 120, 90) />
</Stacker>
```

The first cell is 120 pixels wide; the second receives the remaining 264
pixels. Supply an entry for each child when mixing fixed and flexible cells.
Keep the requested sizes and gutters within the available space. A child's
own width or height affects its content inside the assigned cell; `sizes`
controls the cell allocation.

Container placement and draw order are separate. Earlier siblings still
render in front of later siblings. Keep backgrounds after their foreground
content, as in the Frame example. See
[Element order](template-language.md#element-order).

## Padding

`padding_x` adds equal inner spacing at the left and right;
`padding_y` does the same at the top and bottom. They change the area
available to children while leaving the node's outer bounds in place.

```pax
<Group width=240px height=160px padding_x=12px padding_y=16px>
    <Text width=100% height=100%
        style={font_size: 18px, fill: rgb(32, 40, 48)}>Room around an idea.</Text>
</Group>
```

The Text receives a 216-by-128-pixel area, offset by 12 and 16 pixels.
Percentage padding resolves against the corresponding outer dimension:
`padding_x=10%` reserves one tenth of the width on each side.

Padding applies to the child layout area. A background child that should
cover the entire outer surface is often clearest as a sibling of a padded
content Group. This separates the surface's bounds from its content inset.

<a id="content-driven-size"></a>
## Autosize

A fixed area gives children a space to fill. With `autosize`, content can
instead determine an omitted container dimension. This is useful for a stack
of differently sized notes or a label whose surrounding frame should grow.

```pax
<Stacker width=280px autosize=true gutter=12px>
    <Group height=48px>
        <Text x=12px y=12px width={100% - 24px} height=24px>A short note</Text>
        <Rectangle width=100% height=100% fill=rgb(246, 241, 230) />
    </Group>
    <Group height=96px>
        <Text x=12px y=12px width={100% - 24px} height=72px>A longer note has more room for its contents.</Text>
        <Rectangle width=100% height=100% fill=rgb(220, 216, 206) />
    </Group>
</Stacker>
```

This vertical stack has a width of 280 pixels and a measured height of 156:
48 plus 96, with one 12-pixel gutter. Its height is omitted so autosize can
supply it. An explicit height would continue to constrain that axis.

For a Group or Frame, measurement includes the placed extent of its content.
A child beginning at `x=200px` with width 20 contributes an extent of 220
pixels from the origin. Content wholly before the origin does not add a
positive width. Padding contributes to the measured outer size as well.

### Axis controls

With `autosize=true`, these are the defaults:

| Container | Measured axes |
| --- | --- |
| Vertical Stacker | Its extending axis: height |
| Horizontal Stacker | Its extending axis: width |
| Group and Frame | Both axes |
| Scroller | Scroll-content height, separately from the viewport |
| Link | Both axes from supplied content |
| Tooltip | Both axes from its trigger content |

`autosize_x` and `autosize_y` override those per-axis choices. For example,
a vertical stack can measure height while keeping a caller-supplied width.
Use `autosize=true` alongside Stacker's axis overrides. An explicitly
specified dimension still wins over measurement for that dimension.

Autosize needs a measurable starting point. A parent whose height comes only
from a child at `height=100%` leaves the calculation circular. Use concrete
child sizes or intrinsic measurements on the axis being measured. When the
axis cannot be resolved, current containers can fall back to top-down layout.
Empty measured content collapses to zero on managed axes.

Conditional children participate in this measurement too. Adding or removing
rows inside an autosized Stacker reflows its cells and updates its measured
height; later size changes in a newly added row also propagate to the stack.

Text and native controls can report measurements after initial layout, and
autosized ancestors react to those updates. Fonts, wrapping, and platform
controls influence the result. Check the settled layout with realistic
content rather than depending on its first frame. The
[`auto-sized-containers` example](https://github.com/paxproject/pax/tree/dev/examples/fixtures/auto-sized-containers)
explores the available surfaces.

## Responsive layout

Start with fluid relationships: percentage widths, deliberate insets, and
cells that divide the available space. Add a breakpoint when the arrangement
needs to change, such as two cards becoming too narrow to read beside each
other.

Use the `Notes` and `NoteCard` declarations from Components, including the
`advance` handler. Replace `lib.pax` with this small responsive board:

```pax
<Group class="board" x=24px y=24px width={100% - 48px}>
    <Text width=100% height=36px
        style={font_size: 24px, fill: rgb(32, 40, 48)}>On the workbench</Text>
    <Stacker class="cards" y=64px width=100% gutter=16px>
        <NoteCard width=100% height=100%
            title={self.title} progress={self.progress}
        />
        <NoteCard width=100% height=100%
            title="Sketchbook" progress=0.75
        />
    </Stacker>
    <Button class="advance" width=160px height=36px @button_click=self.advance>Advance</Button>
</Group>

@settings {
    .board { height: 296px }
    .cards {
        height: 160px
        direction: StackerDirection::Horizontal
    }
    .advance { y: 248px }

    if @viewport.width < 720 {
        .board { height: 472px }
        .cards {
            height: 336px
            direction: StackerDirection::Vertical
        }
        .advance { y: 424px }
    }
}
```

At 720 pixels and above, the cards share a row. Below that threshold, they
occupy two 160-pixel rows separated by the same 16-pixel gutter. The Button
moves beneath them. Both layouts leave a 24-pixel outer inset.

These conditions change settings on the existing nodes. The cards retain
their identity and state while the window crosses the breakpoint. A
structural `if` that creates a different subtree has different lifetime
implications; see
[Components](components-composition.md#conditional-content).

The threshold belongs to this composition. Choose it from the space the
content needs, then try widths just above and below it. Also test a short
window: adapting a row into a column does not guarantee that all content fits
vertically. Use a correctly sized Scroller when the content needs more space;
[Scrolling](scrolling-viewports.md) owns viewport/content sizing and input.

### Viewport or component bounds?

`@viewport.width` and `@viewport.height` describe the mounted app's viewport
in logical pixels. In an embedded app, that is the embed's viewport, not the
entire surrounding page. Orientation helpers describe that same area.
They are useful for screen-level composition; see the complete
[built-in globals](data-binding-expressions.md#built-in-globals).

A reusable component may receive only a fraction of that area. For example,
a 320-pixel sidebar should not choose a wide layout merely because the app
window is wide. Derive a property from the component's own bounds when its
layout policy should follow the space its caller allocates.

For Notes, add `pub compact: Property<bool>` and initialize it with
`compact: Property::new(false)` in the existing custom default. Then add:

```rust
impl Notes {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.compact.replace_with(LocalProperty::computed(
            move || bounds.get().0 < 720.0,
            &deps,
        ));
    }
}
```

If Notes already has a mount method, add the computation there. Change the
settings condition to `if self.compact`; it now follows Notes' allocated
width. Read bounds in the component's lifecycle context here, rather than a
Button's event context. The latter would describe the Button. The
[`materials` example](https://github.com/paxproject/pax/tree/dev/examples/materials)
uses this bounds-derived pattern.

Avoid making the measured property depend on the very size rule it controls.
A width supplied by the caller is a clear input for choosing the component's
internal arrangement. [Properties](state-properties.md#computed-properties)
explains the dependency model.

`bounds_self` carries both dimensions. When a component also feeds an autosized
height back to its parent, even a formula that reads only the width can form a
cycle through those shared bounds. For a layout policy that converges through
measurement feedback, use a mount-owned subscription instead:

```rust
let bounds = ctx.bounds_self.clone();
let compact = self.compact.local();
let deps = [bounds.untyped()];
let update = move || { compact.set_if_neq(bounds.get().0 < 720.0); };
update();
ctx.subscribe(&deps, update);
```

The initial call supplies the policy before descendants mount. Subsequent
changes settle through the reactive queue, including during
[yielded web startup](how-pax-runs.md#experimental-resumable-startup-on-web),
without requiring another `@pre_render`. Only write when the result changes;
an oscillating size policy still needs its layout dependencies redesigned.

### Shared visual settings

Classes keep repeated visual choices together, and conditional settings can
adapt typography, spacing, or surface treatment as well as geometry.
Keep a card's common appearance in its component or shared settings; let the
containing layout own the arrangement. `ImportSettings` can share a settings
vocabulary across templates. See
[Imported settings](template-language.md#imported-settings) and
[Drawing and Styling](drawing-styling.md) for themes, fills, and materials.

Text alignment controls content within the Text node's bounds. It does not
position that node in its parent. Use layout position and anchors for the
node, and `style.align_horizontal`, `style.align_vertical`, and
`style.align_multiline` for text within it. [Text](text-fonts-images.md)
covers the distinction in more detail.

## Transforms and origins

`rotate`, `scale_x`/`scale_y`, and `skew_x`/`skew_y` transform a node and
its descendants around its anchor. Scaling changes the displayed geometry;
`width` and `height` remain the node's unscaled layout bounds.

```pax
<Group width=320px height=180px>
    <Rectangle x=50% y=50% width=160px height=80px
        rotate=-8deg scale=90% fill=rgb(55, 120, 90)
    />
</Group>
```

The default 50-percent anchors keep this rectangle centered as it turns and
scales. Use explicit anchors when an object should rotate around another
point. A parent's transform also transforms its children, so group pieces
that should move together.

An assigned Stacker cell does not automatically rearrange its siblings to
avoid visual overlap caused by a transform. Flow-space measurement excludes
the full rotated, scaled, or skewed silhouette, so do not expect autosize to
reserve all the space a transformed drawing can cover.
This remains true when an ancestor is scaled to zero: its descendants keep
their local layout measurements, and content or native text measurements can
still update autosized containers before the ancestor becomes visible again.
[Motion](animation-motion.md) covers changes over time;
[Events](event-handling-rust.md#events-and-coordinates) explains converting
input positions through transformed geometry.

### Rotating flat surfaces in 3D

Enable the default-off `3d` feature on your application's `pax-kit` dependency:

```toml
pax-kit = { version = "0.39.1", features = ["3d"] }
```

This capability includes spatial transforms, depth composition and picking,
Cuboid/CuboidFace, Ellipsoid, RouteCuboid, and lighting on transformed surfaces. Ordinary
2D transforms, materials and planar lighting remain available without it.
See [optional runtime capabilities](targets-build-deploy.md#optional-runtime-capabilities)
for build and dependency behavior.

The experimental GPU spatial path adds `rotate_x`, `rotate_y`, and `translate_z`
to ordinary elements and groups. No scene wrapper is required. Unspecified
values are zero, so existing interfaces keep their 2D layout and paint order.
`rotate` remains rotation around Z. Rectangle and Ellipse remain flat shapes;
these properties do not extrude them into solids.

```pax
<Group width=320px height=180px x=50% y=50%
    rotate_x=12deg rotate_y=-30deg translate_z=40px>
    <Textbox x=20px y=70px width=280px height=40px text="Edit on a tilted surface"/>
    <Rectangle width=100% height=100% fill=rgb(20, 125, 120)/>
</Group>
```

Layout, percentages, padding, and anchors remain local to each element.
Rotation uses the existing anchor. Scale/skew and the local `transform` act
in the element's plane; the pose then applies X rotation, Y rotation, and
`rotate` (Z rotation), followed by placement in the parent. `translate_z`
moves along the **parent's** Z axis, in logical pixels. It accepts numbers or
`px`, including expressions such as `{20px + offset}` with an absolute result;
percent depth is not supported. Positive Z points toward the default viewer
when the parent is unrotated. An ancestor's tilt also tilts its descendants'
coordinate axes.

The current scene view is orthographic. Tilt changes a surface's projected
shape; changing Z alone does not change its size. Local layout dimensions stay
unchanged, including when a surface is edge-on. Edge-on surfaces are not pointer
targets. Non-finite spatial poses paint no area and keep their mounted state,
so replacing the invalid value can restore the subtree.

Geometric depth takes precedence over template order. When projected flat
surfaces intersect, their front/back order can change across the intersection.
Coplanar content retains Pax's earlier-sibling-on-top order. Rotating a shared
group keeps its coplanar descendants together; independently tilting or moving
a child out of that plane can separate it. On the back of a spatial surface,
the coplanar layer stack reverses: an opaque background hides the labels and
controls painted on its front. When opposite-facing surfaces coincide exactly, front-facing paint wins the
tie. Surfaces have zero thickness in this checkpoint.

A Frame remains a projected 2D aperture over its descendants. Its clip does
not become a volume, and clipping does not flatten the descendants' depth.
Native text and controls use their exact projected affine footprints and keep
platform ownership through pose changes. On the web, changing projected scale
allows native text and controls to rerasterize, including when a face arrives
from an edge-on pose; permanent `will-change: transform` overrides can prevent
that refresh. Unmounting a native element releases its mask and clip resources.
The engine compares logical hit
geometry at the pointer; it does not infer extra hit regions from shader pixels.
See [Events and coordinates](event-handling-rust.md#events-and-coordinates).

Try [Spatial Workbench](https://github.com/paxproject/pax/tree/dev/examples/fixtures/spatial-workbench)
for crossing surfaces, live controls, child tilt, aperture clipping, group opacity,
and back-face checks:

```sh
pax-cli run --path examples/fixtures/spatial-workbench --target web
```

This is a bounded first spatial implementation:

- Depth composition is implemented by the GPU renderer within a logical canvas
  layer. Distinct native/scroller surfaces retain their existing platform
  stacking restrictions; arbitrary intersections between those layers are not
  resolved. Native controls also retain the existing separate-opacity and
  platform-popup limitations.
- GPU canvas paint fades an assembled group once. Hidden opaque members stay
  hidden during a fade, including when outside geometry interleaves with the
  group. This uses a bounded composition path; see [Spatial group fades](#spatial-group-fades).
  Native surfaces retain their separate transparency approximation.
- CPU fallback projects the transforms but retains document paint order. It
  emits a warning when spatial depth is requested; native occlusion and engine
  targeting follow that same fallback order. Use the GPU renderer for this
  example and for geometric depth.
- Scene-camera selection, perspective presentation, spatial
  clipping volumes are not enabled by these
  properties. Shader containers and detached mask-source captures retain their
  existing raster composition semantics.
- With fully opaque group scopes, the GPU compositor splits intersecting
  surfaces into ordered fragments and batches their clip changes and paint
  into one render pass. Individual fills can still be translucent. Coplanar
  content shares a plane, and unsplit surfaces need no extra spatial clip.
  A translucent group that occupies one uninterrupted span in that paint order
  shares a single capture, including its intersecting surfaces. Changing only
  its opacity reuses that capture. Disjoint fragments can be reordered to keep
  that span together: a separate tilted panel farther down a page does not by
  itself force repeated captures of a translucent solid. Overlapping fragments
  keep their relative paint order. Genuine outside interleaving uses the bounded
  presence-view composition below. These paths target modest numbers of
  intersecting surfaces. Screenshot mirroring on targets without surface-copy
  support preserves the same clip and fade ordering.

### Opaque curved solids

`Ellipsoid` adds a curved volume to an ordinary Pax tree. Enable `pax-kit/3d`
and use the GPU renderer. Its `width` and `height` follow normal layout; `depth`
is an absolute logical-pixel extent centered on local Z=0, defaulting to 100px.
Equal extents produce a sphere. Pose changes transform shared mesh geometry.

```pax
<LightFrame width=100% height=100%>
    <Ellipsoid x=50% y=50% width=240px height=180px depth=200px
        rotate_y=25deg translate_z=-40px
        color=rgb(239, 167, 77) material={Material::glossy(0.35)} />
    <LightSource x=20% y=10% z=400px radius=1500px intensity=2.0 />
    <AmbientLight intensity=0.4 />
</LightFrame>
```

A flat panel may pass through the volume: only the part of the solid in front
of the panel hides it. The GPU draw, native-control punch-through contour, and
pointer depth use the same tessellated surface. Hard clips remain projected
apertures; they clip the visible result, without adding solid caps at the cut.
Controls keep their mounted identity as the occlusion changes. Tile coverage
includes the full volume, including a pose whose local XY plane is edge-on.

The mesh topology stays resident during motion. Each immutable pose also keeps
its projected silhouette and a bounded cache of solid/plane coverage contours,
shared by repeated queries against that pose. Paint-only changes reuse these
contours; changing the pose replaces them. This reduces repeated clipping work
for native siblings, tiles, and spatial fades, but animated geometry still has
CPU intersection work. Use the [web scene profiler](developer-workflow.md#profile-a-web-scene)
to distinguish that work from paint-only updates in your scene.

This first mesh primitive accepts one opaque `color` and a normal Pax
`material`. It does not wrap live children onto a curved surface. Zero, negative,
or non-finite extents and singular poses have no coverage. Node/group opacity
uses the spatial fade contract below. Translucent surface color, alpha masks,
shader-effect captures, and detached mask sources are unsupported: affected
ellipsoids are omitted with a bounded diagnostic. CPU rendering also omits them. Transparent shells and arbitrary concave meshes are
not implemented. The logical-layer and native-control limits above still apply.

Try the [Surface / Solid fixture](https://github.com/paxproject/pax/tree/dev/examples/fixtures/solid-intersection)
for a clipped editable panel, reproducible depth presets, rotation, and a
scroll-driven page with live controls:

```sh
pax-cli run --path examples/fixtures/solid-intersection --target web
```

### Spatial group fades

`opacity` fades an assembled appearance. It does not make each surface into a
transparent material: the back of an opaque ellipsoid stays hidden while the
ellipsoid fades. The same rule applies to overlapping opaque children of a
Group. Material/color alpha remains a distinct paint operation; translucent
ellipsoid shells are not supported yet.

```pax
<Group width=100% height=100% opacity={self.visibility}>
    <Ellipsoid x=50% y=50% width=220px height=260px depth=180px
        translate_z=50px color=rgb(239, 167, 77) />
    <Rectangle x=50% y=50% width=360px height=240px
        fill=rgb(246, 239, 216) />
</Group>
```

Within one logical canvas layer, geometric depth still determines where outside
GPU content appears. For example, when an outside blue panel sits between a
group's opaque front and rear members,
fading that group reveals the blue panel without making the hidden rear member
bleed through its front member. Nested fades apply conditionally within their
parent assembly.

Uninterrupted planar groups keep the existing single-capture path. Fading meshes
and genuinely interleaved groups use weighted **presence views**: fully composed
scene views with the assembly present or absent. These views include outside
content at its actual depth; their premultiplied colors and alpha are combined
once. Opacity-only updates reuse the views. Geometry, clipping, lighting, image,
or backdrop changes invalidate affected captures.

This experimental path has a budget of **eight scene views per physical canvas
tile**, in addition to its ordinary backdrop draw, and retains at most eight
view textures. One fading assembly needs two views; three independent overlapping
fades can need eight. Nested absent scopes prune their descendants, and disjoint
projected capture regions add their costs instead of multiplying them. The budget
is a work/residency bound, not a frame-rate guarantee.

If a component exceeds the available budget, the renderer reports a bounded
warning and renders that GPU component opaque at intermediate opacity values.
Zero-opacity scopes remain hidden. Simplify overlapping fades or fade a shared
ancestor to stay within the exact path. Native controls continue to use their
platform opacity even at this limit.

Native text and controls keep their mounted identity and platform-owned input.
Occlusion between members sharing the same fading ancestor excludes that shared
opacity from the native cutout, preserving internal hiding. Independently faded
native/canvas layers still have the transparency-stacking approximation described
in [Compositing](compositing-effects.md#native-compositing); these views do not
capture native pixels. Opacity alone does not disable pointer or keyboard input.

The Surface / Solid fixture exposes **Fade**, group opacity, an outside
surface's depth, rotation, and native text editing. Its two solids and live panel
share a fade; the blue translucent band belongs to the outside scene.

### Cuboids with live face content

`Cuboid` supplies six rectangular surfaces with real spatial placement. Its
`width` and `height` follow normal layout; `depth` is an absolute length in
logical pixels (100px by default). Local Z runs from `-depth / 2` to
`+depth / 2`. Rotation uses the existing XY anchors and local Z=0, so a centered
XY anchor rotates around the solid's center. `translate_z` moves the whole
solid independently of its depth. Negative or non-finite depth collapses to
zero; depth percentages are not supported.

```pax
<Cuboid width=320px height=200px depth=180px
    x=50% y=50% anchor_x=50% anchor_y=50%
    rotate_x=15deg rotate_y=-30deg fill=rgb(230, 232, 225)>
    <CuboidFace side=CuboidSide::Front width=100% height=100%>
        <Textbox x=20px y=30px width={100% - 40px} height=36px
            text="This is a live control" />
    </CuboidFace>
    <CuboidFace side=CuboidSide::Right width=100% height=100%>
        <Text x=16px y=20px width={100% - 32px} height=40px text="Right face" />
        <Rectangle width=100% height=100% fill=rgb(225, 180, 110) />
    </CuboidFace>
</Cuboid>
```

`fill` paints every face, using the existing layered fill/material model.
`fill=[]` leaves just authored content. `CuboidFace` places a live subtree on
the chosen side of its nearest owning Cuboid. Contents are clipped to a
projected rectangular face aperture. Changing the side, dimensions, or pose
moves the same mounted content; it does not take a screenshot or duplicate
controls. Multiple containers on one side use normal coplanar paint order.
Intermediate components can contain face declarations, and nested Cuboids
provide their own face geometry.

| Side | Content dimensions | Outward direction before rotation |
| --- | --- | --- |
| Front / Back | width × height | +Z / −Z |
| Left / Right | depth × height | −X / +X |
| Top / Bottom | width × depth | −Y / +Y |

Each face's local coordinates are upright when viewed from outside that face.
The shared fill sits beneath authored face content. Flat Rectangle/Ellipse
elements keep their existing behavior; only Cuboid introduces these six
surfaces. The spatial backend restrictions above still apply, including CPU
paint ordering, separate native/scroller layers, and group opacity. Face
orientation also supplies its material's lighting normal; all faces can share
the same paint while responding differently to a light. See
[Spatial lighting](drawing-styling.md#spatial-lighting) for lights inside
transformed groups and the distinction between vector materials and native UI.

For a rotating route transition in a viewport of width W and height H, use
depth W for a horizontal quarter-turn or H for a vertical quarter-turn. The
incoming face then ends at W × H without stretching its content. A fixed
non-square cuboid cannot make all six sides the same viewport dimensions.
See the [Cuboid Router fixture](https://github.com/paxproject/pax/tree/dev/examples/fixtures/cuboid-router)
and [RouteCuboid](routing.md#cuboid-route-transitions) for a reusable route
branch that computes this geometry and supplies normal enter/exit timelines:

```sh
pax-cli run --path examples/fixtures/cuboid-router --target web
```

### Spatial math for engine integrations

Rust integrations can use `pax_runtime_api::math::Transform3`, `Point3`, and
`Plane3`, together with `pax_runtime_api::camera`, to project geometry and
intersect a pointer ray with a transformed plane. The runtime also retains
local placement and unprojected world geometry separately from its 2D
presentation. Use the common properties above to pose a live element. The
camera module provides reusable projection and picking math; constructing a
`Camera` in application code does not select a scene camera.

`Transform3::from(transform_2d)` preserves the existing affine coefficients
and leaves z unchanged. Matrices default to identity and compose as
`parent * child`. Coordinates use logical pixels, x right, y down, and positive
z toward the default viewer. Positive z-rotation matches the existing `rotate`.

An orthographic camera with an identity view preserves window x/y:

```rust
use pax_runtime_api::{
    camera::{Camera, CameraProjection},
    math::{Point3, Transform3},
};

let projection = CameraProjection::orthographic((800., 600.), 1000., -1000.).unwrap();
let camera: Camera = Camera::new(Transform3::identity(), projection).unwrap();
let screen = camera.project(Point3::new(120., 80., 0.)).unwrap();
// screen.position is (120, 80); screen.depth is 0.5.
```

The depth range above is chosen by this example. Camera projection reports
normalized depth from 0 (front) to 1 (back). Perspective cameras look along -z
and use positive near/far distances. Projection and picking share a matrix and
its inverse; invalid transforms, points outside the depth range, and edge-on
plane intersections return `None`. Plane picking returns local coordinates;
the caller still checks logical bounds, clips, and visibility.

See the [camera API](api/pax-runtime-api/camera.md) and
[spatial math API](api/pax-runtime-api/math/spatial.md) for the checked constructors and
coplanarity rules. These utilities run on native and WebAssembly targets;
they do not establish 3D rendering support on either target.

Engine integrations with a `NodeInterface` can inspect the retained geometry:

```rust,ignore
// `node` is a pax_runtime::node_interface::NodeInterface.
let geometry = node.spatial_geometry().get();
let world = geometry.world_transform();
let plane = geometry.plane(); // None when the local x/y plane has collapsed.
let local = node.local_transform_and_bounds().get();
```

These reactive properties are owned by the engine: subscribe to or inspect
them, rather than replacing their bindings. Local placement includes parent
padding and any frame assigned by a container. It remains available beneath
a collapsed ancestor, and moving an ancestor does not recompute descendant
local placement when its layout inputs and parent dimensions are unchanged.
World geometry composes those local placements before projection. The current
2D path keeps affine multiplication; full spatial matrices are retained when
using `SpatialGeometry` directly in Rust.

`SpatialGeometry::is_coplanar_with` compares geometric planes with explicit
distance and angular tolerances. Opposite-facing planes can be coplanar;
`plane()` preserves their distinct normals. Coplanarity alone does not grant
permission to flatten across opacity, clipping, or paint-order boundaries.
The retained geometry is in unscrolled scene coordinates; native scroll
presentation is applied separately. See the
[runtime spatial API](api/internal/pax-runtime/spatial.md).

## Multi-Axis Common Properties

The `anchor`, `scale`, and `skew` shorthands accept a scalar for both axes
or a two-item list in `[x, y]` order:

```pax
<Group width=240px height=120px
    x=50% y=50% anchor=[50%, 50%] scale=80% skew=[0deg, 4deg]
>
    <Rectangle width=100% height=100% fill=rgb(220, 216, 206) />
</Group>
```

Axis-specific properties remain available. In the same settings layer, a
longhand such as `scale_y` takes precedence over the corresponding shorthand
axis. A longhand expression using `@base` sees that axis's shorthand value.
The wider settings precedence rules belong to
[Templates](template-language.md#multiple-and-reactive-classes).

## Layout Role

`layout_role=LayoutRole::Breakout` excludes a child from its parent's flow
and measured content extent. It is useful for a badge or floating panel that
should remain near its owner without making the owner larger.

```pax
<Group width=160px autosize=true>
    <Group y={100% + 8px} anchor_y=0px width=220px height=64px
        layout_role=LayoutRole::Breakout
    >
        <Text x=12px y=12px width={100% - 24px} height=40px>More room for a thought.</Text>
        <Rectangle width=100% height=100% fill=rgb(246, 241, 230) />
    </Group>
    <Text width=160px height=32px>Field notes</Text>
</Group>
```

The label determines the outer Group's measured height of 32 pixels. The
floating panel begins eight pixels below it and does not enlarge that
measurement. Earlier source order keeps the panel in front where it overlaps.

Breakout changes layout participation while retaining the normal ancestor
relationships. It still scrolls, clips, masks, renders, and hit-tests within
that tree. It does not create a portal above a clipping Frame or pin content
to the app window. Percentage sizes still use the relevant parent area.
The default `LayoutRole::Default` participates normally in flow and
measurement.

## Opt into native safe-area spacing

The iOS and iPadOS chassis draw edge to edge. Add a standard-library
`DynamicIslandSpacer` where content should avoid the status bar or top cutout:

```pax
<Stacker width=100% autosize=true gutter=0px>
    <DynamicIslandSpacer />
    <Text width=100% height=40px>Below the safe area</Text>
</Stacker>
```

With no explicit dimensions, the spacer fills its container's width and
measures its height from UIKit's current top safe-area inset, in logical
pixels. It renders nothing and does not intercept input. A content-sized
Stacker gives that measured space its own cell. In a plain Group, siblings
are positioned independently, so adding a spacer alone does not move them.

The optional `edge` selects `SafeAreaEdge::Top` (the default), `Bottom`, `Left`,
or `Right`. Top and Bottom fill width and measure height; Left and Right fill
height and measure width. Explicit width or height overrides that axis's
measurement. The read-only `inset` output supports deliberate positioning:

```pax
<DynamicIslandSpacer inset=bind:self.safe_top />
<Group y={(self.safe_top)px} width=100% height={100% - (self.safe_top)px}>
    <Text width=100% height=40px>A safe heading</Text>
</Group>
```

Declare `pub safe_top: Property<f64>` on the owning Rust component. The
[calculator example](https://github.com/paxproject/pax/tree/main/examples/calculator) binds all four edges to keep its
content clear while its background covers the whole window.
It also uses an edge-to-edge case below 600 logical pixels of window width
on web and macOS; native iOS and iPadOS always use that presentation. This
width breakpoint belongs to the example's layout, independently of safe-area
insets. The keypad keeps its touch sizes, and the LCD receives the remaining
height; short windows scroll the calculator.

Insets come from the visible window's safe rectangle, including devices
without a Dynamic Island, and update on rotation and window resizing in both
debug and release. Landscape often needs side spacing instead of top spacing;
use the corresponding edge when that side contains content. The spacer does
not inset the root or detect padding already applied by an ancestor. Add it
once at the relevant window edge to avoid reserving the same space twice.

Currently this component reserves space on **native iOS and iPadOS only**.
Web (including Safari), macOS, and other platforms contribute zero on the
inset axis. A Stacker's separately authored gutter still applies around a
zero-sized child.

## Read more

You can now predict an element's area, choose how it aligns within that area,
and reshape a collection while keeping its state intact. Continue with
[Text, Fonts, and Images](text-fonts-images.md) for content and measurement,
[Drawing and Styling](drawing-styling.md) for the visual vocabulary, and
[Scrolling](scrolling-viewports.md) for content larger than its viewport.
[Motion](animation-motion.md) builds on these same layout relationships.
