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
    <Text width=100% height=100% text="Room around an idea."
        style={font_size: 18px, fill: rgb(32, 40, 48)}
    />
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
        <Text x=12px y=12px width={100% - 24px} height=24px text="A short note" />
        <Rectangle width=100% height=100% fill=rgb(246, 241, 230) />
    </Group>
    <Group height=96px>
        <Text x=12px y=12px width={100% - 24px} height=72px
            text="A longer note has more room for its contents."
        />
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

Text and native controls can report measurements after initial layout, and
autosized ancestors react to those updates. Fonts, wrapping, and platform
controls influence the result. Check the settled layout with realistic
content rather than depending on its first frame. The
[`auto-sized-containers` example](https://github.com/paxproject/pax/tree/dev/examples/src/auto-sized-containers)
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
    <Text width=100% height=36px text="On the workbench"
        style={font_size: 24px, fill: rgb(32, 40, 48)}
    />
    <Stacker class="cards" y=64px width=100% gutter=16px>
        <NoteCard width=100% height=100%
            title={self.title} progress={self.progress}
        />
        <NoteCard width=100% height=100%
            title="Sketchbook" progress=0.75
        />
    </Stacker>
    <Button class="advance" width=160px height=36px
        label="Advance" @button_click=self.advance
    />
</Group>

@settings {
    .board { height: 296px }
    .cards {
        height: 160px
        direction: StackerDirection::Horizontal
    }
    .advance { y: 248px }

    if $viewport.width < 720 {
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

`$viewport.width` and `$viewport.height` describe the mounted app's viewport
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
        self.compact.replace_with(Property::computed(
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
[`materials` example](https://github.com/paxproject/pax/tree/dev/examples/src/materials)
uses this bounds-derived pattern.

Avoid making the measured property depend on the very size rule it controls.
A width supplied by the caller is a clear input for choosing the component's
internal arrangement. [Properties](state-properties.md#computed-properties)
explains the dependency model.

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
[Motion](animation-motion.md) covers changes over time;
[Events](event-handling-rust.md#events-and-coordinates) explains converting
input positions through transformed geometry.

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
axis. A longhand expression using `$base` sees that axis's shorthand value.
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
        <Text x=12px y=12px width={100% - 24px} height=40px
            text="More room for a thought."
        />
        <Rectangle width=100% height=100% fill=rgb(246, 241, 230) />
    </Group>
    <Text width=160px height=32px text="Field notes" />
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
    <Text width=100% height=40px text="Below the safe area" />
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
    <Text width=100% height=40px text="A safe heading" />
</Group>
```

Declare `pub safe_top: Property<f64>` on the owning Rust component. The
[calculator example](https://github.com/paxproject/pax/tree/main/examples/src/calculator) binds all four edges to keep its
content clear while its background covers the whole window.

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
