# Scrolling and Viewports
<!-- summary: Size scrollable content, bind its position, and build paged or scroll-driven interfaces. -->
<!-- tags: scrolling, viewports, scroller, carousel, snapping, autosize -->
<a id="scrolling--viewports"></a>

A list of notes, a horizontal shelf, and an illustrated story all need a way
to show content that extends beyond the available space. `Scroller` gives
that content a viewport, clips it at the edges, and connects it to the
platform's scrolling behavior on web, macOS, iOS, and iPadOS.

The scroll position is also a property. You can use it to return to the top,
show reading progress, or move through an animation as the reader explores.

This chapter builds on [Layout](layout-responsiveness.md) and
[property handles and bindings](state-properties.md#property-handles). For Rust handlers,
see [Event Handling](event-handling-rust.md).

## Viewport, content, and position

A Scroller has three related sets of dimensions:

| Property | Meaning |
| --- | --- |
| `width`, `height` | The visible viewport, placed by ordinary Pax layout |
| `scroll_width`, `scroll_height` | The extent of the scrollable content pane |
| `scroll_pos_x`, `scroll_pos_y` | The offset into that pane, as numeric pixel values |

Here is a 240-pixel-tall viewport onto 720 pixels of notes. Paste this into
`src/lib.pax` in a project whose Rust main component imports `pax_kit::*`:

```pax
<Scroller x=24px y=24px width={100% - 48px} height=240px
    scroll_width=100% scroll_height=720px corner_radius=16>
    <Group width=100% height=720px>
        for i in 0..4 {
            <Group y={(i * 180)px} width=100% height=180px>
                <Text x=20px y=20px width={100% - 40px} height=40px
                    text={"Field note " + (i + 1)}
                    style={font: "Arial", font_size: 24px, fill: rgb(36, 54, 47)} />
                <Rectangle x=8px y=8px width={100% - 16px} height=164px
                    corner_radius=12 fill=rgb(225, 234, 220) />
            </Group>
        }
    </Group>
</Scroller>
```

Increasing `scroll_pos_y` moves the visible window farther down the content;
the notes move upward on screen. At rest, the vertical range here is
`0` through `720 - 240 = 480` pixels. If the content fits inside the
viewport, there is no travel on that axis. Platform overscroll and bounce
can temporarily present the edges differently.

`scroll_height=300%` would also describe 720 pixels for this viewport:
percentages on the scroll extent are relative to the Scroller's own viewport.
The extent establishes how far you can scroll; it does not stretch the child
tree to that size. A direct child at `height=100%` still receives the
viewport-height layout frame. The explicit 720-pixel Group above gives its
descendants a content-sized coordinate space.

Scroller does not paint a background. Place painted content inside it, or a
background sibling behind it, depending on which should move. Earlier Pax
siblings appear in front; that is why each note's Text precedes its Rectangle.
`corner_radius` is a numeric pixel radius for the viewport clip.

## Autosized Scrollers

For a document that grows as you add content, let Scroller measure its pane:

```pax
<Scroller x=24px y=24px width={100% - 48px} height=240px
    scroll_width=100% autosize=true corner_radius=16>
    <Stacker width=100% autosize=true gutter=12px>
        for i in 0..5 {
            <Text width=100% height=80px text={"Measured note " + (i + 1)}
                style={font: "Arial", font_size: 24px, fill: rgb(36, 54, 47)} />
        }
    </Stacker>
</Scroller>
```

The stack measures five 80-pixel children and four 12-pixel gutters. Scroller
uses that 448-pixel content height while its viewport remains 240 pixels tall.
Changing the children or their measured sizes updates the scrollable extent.

`autosize=true` on Scroller manages the **vertical content extent** by default.
Horizontal size continues to use `scroll_width`; `autosize_x=true` opts into
horizontal measurement. `autosize_y=false` turns off vertical measurement even
when `autosize=true`.

On a measured axis, Scroller uses its resolved content measurement in preference
to `scroll_height` or `scroll_width`; the explicit value is a fallback if that
measurement cannot be resolved. The public size property remains the input,
so reading `self.some_scroll_height` does not automatically give you the
measured result. This differs from asking a container to measure its own
outer dimensions.

Give measured content a useful starting point: concrete row heights, text
measurement, or an autosized stack. Avoid making an expanding document depend
only on `height=100%`. Font loading and native measurements can change the
settled extent. [Layout's autosize section](layout-responsiveness.md#autosize)
explains the shared measurement rules.

## Read and set the scroll position

Use `bind:` to share the Scroller's position with a component property. User
scrolling updates that property; setting it from Rust requests a new position.
This also gives buttons and visual indicators one place to read the state.

An initial nonzero position applies when the Scroller mounts, including when
it appears inside an `if` branch. Later property writes move the native host
and the renderer's viewport together; no user scroll is needed to reveal the
new position. This matters for large two-dimensional panes such as a graph:
center with `(content extent - viewport extent) / 2` on each axis, and update
the offsets together with the content extent when zooming. Keep requested
offsets inside the available range; active native gestures and platform
bounce still control their normal interaction behavior.

In `src/lib.rs`:

```rust
use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub scroll_y: Property<f64>,
}

impl Example {
    pub fn back_to_top(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.scroll_y.set(0.0);
    }

    pub fn last_note(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.scroll_y.set(480.0);
    }
}
```

In `src/lib.pax`:

```pax
<Button x=24px y=24px width=120px height=36px label="Back to top"
    @button_click=self.back_to_top />
<Button x=156px y=24px width=120px height=36px label="Last note"
    @button_click=self.last_note />
<Text x=24px y=72px width=280px height=28px
    text={"Offset: " + self.scroll_y}
    style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />

<Scroller x=24px y=120px width={100% - 48px} height=240px
    scroll_width=100% scroll_height=720px scroll_pos_y=bind:self.scroll_y>
    <Group width=100% height=720px>
        for i in 0..4 {
            <Text x=16px y={(i * 180 + 16)px} width={100% - 32px} height=80px
                text={"Field note " + (i + 1)}
                style={font: "Arial", font_size: 24px, fill: rgb(36, 54, 47)} />
        }
        <Rectangle width=100% height=100% fill=rgb(225, 234, 220) />
    </Group>
</Scroller>
```

Use plain numbers for these offsets: `480.0`, rather than `480px`. Their Rust
type is `f64`; the extent properties use `Size` and accept `px` or `%`.

The example's 480-pixel destination comes from its known dimensions. For a
responsive document, derive destinations from current layout and content,
and keep them within the available travel. Content shrinkage and viewport
resizing can constrain what the native host can show. Avoid repeatedly
writing a saved position while the person is actively scrolling; native
gesture updates and application writes would compete for the same state.

### Position versus scroll events

`@scroll` sends an `Event<Scroll>` with `delta_x` and `delta_y`. It is a shared
delta stream used by wheel/touch input and native position notifications.
It is useful when an action needs movement information. Use the bound
`scroll_pos_y` for reading progress or a return position, rather than building
a second position by accumulating event deltas.

Native position notifications report movement that has already happened.
They are not a cancellable, cross-platform “before scrolling” hook. Likewise,
`@wheel` describes wheel input and does not cover every way someone can scroll.
See [Event Handling](event-handling-rust.md) for event binding and propagation.

### A resizable two-dimensional graph

The canonical `calculator` example uses both scroll axes for a function plot.
Its visible graph grows with the LCD, while zoom changes the content pane
size. The important template relationship is:

```pax
<Scroller width={(self.graph_width)px} height={(self.graph_height)px}
    scroll_width={(self.graph_extent)px} scroll_height={(self.graph_extent)px}
    scroll_pos_x=bind:self.scroll_x scroll_pos_y=bind:self.scroll_y>
    <Group width={(self.graph_extent)px} height={(self.graph_extent)px}>
        <!-- Axes, labels, and bounded curve geometry share this coordinate space. -->
    </Group>
</Scroller>
```

The Rust component supplies the five numeric properties shown here. Its 1×
magnification is 16 logical pixels per graph unit. The opening polar rosette,
`1+3*cos(11*θ)`, selects a zoom step that fits the viewport until the first
interaction. After that it keeps the chosen scale on resize,
so a larger window reveals more data. Zoom changes the scale and the pane
extent together. Both operations preserve the world-space center where the
finite boundary permits, derive the new scroll offsets, and
regenerate geometry for the exact visible viewport, with no overscan margin. Native scroll
position bindings drive subsequent updates; the application does not accumulate
a second position from scroll events. Sampling the viewport rather than the
whole pane keeps work bounded as the person explores.

Run `pax-cli run --path examples/src/calculator --target web` from a source
checkout and switch to Graph. Try `1/x`, pan, then resize the window. The Rust
expression engine deliberately leaves gaps across undefined domains and caps
sampling work. Its finite world is −256 through 256 on each axis; this example
has one visible function, calculator-style editing, and zoom from 0.25× to 16×.
Its `2nd` → `x θ` (MODE) sequence selects Cartesian or polar plotting; polar formulas use `θ` over
0…2π radians. It preserves separate drafts and valid plots for both modes.
While graphing, GRAPH submits the current expression, centers the origin,
and focuses the editor without changing scale. Invalid input preserves the
last valid curve and displays an error inside the LCD.
The sampler reserves work across the viewport, culls off-screen bounds, and
represents unresolved continuous subpixel detail with a disclosed range envelope
instead of spending the whole budget on the first dense region. The example's README
documents the complete grammar, controls, and numerical limits.
For polar curves, refinement measures geometric deviation from a chord in
screen pixels, not how evenly the parameter moves along it. Conservative
range checks cover the whole interval, and a bounded second pass reuses
budget left by simpler or culled angular slices. Both passes share the same
work and geometry caps; unresolved detail can still appear at extreme complexity.

Calculate mode demonstrates a second, vertical Scroller for history, with a
fixed editor outside the viewport. Its content height comes from wrapped
character rows. Changing Calculate text size reflows those rows without
changing Graph zoom. New results set the bound history offset to the new
bottom; ordinary frame updates leave native scrolling in control. This keeps
a long history reachable even when the LCD is small.

## Horizontal regions and snapping

For a horizontal shelf, make `scroll_width` larger than `width` and keep the
vertical extent at `100%`. The same geometry works on both axes.

Snap positions add landing points. This strip contains three viewport-width
panels and snaps at their starts:

```pax
<Scroller x=24px y=24px width={100% - 48px} height=180px
    scroll_width=300% scroll_height=100%
    snap_positions_x=[0px, 100%, 200%] corner_radius=16>
    for i in 0..3 {
        <Group x={(i * 100)%} anchor_x=0% width=100% height=100%>
            <Text x=20px y=20px width={100% - 40px} height=48px
                text={"Panel " + (i + 1)}
                style={font: "Arial", font_size: 24px, fill: rgb(36, 54, 47)} />
            <Rectangle x=6px width={100% - 12px} height=100%
                corner_radius=16 fill=rgb(225, 234, 220) />
        </Group>
    }
</Scroller>
```

Each panel uses `anchor_x=0%` so its percentage position locates its left edge.
Snap percentages resolve against the viewport on the corresponding axis.
`snap_positions_y` provides the vertical equivalent. Leave the lists empty
for ordinary continuous scrolling. Web uses native CSS scroll snapping;
Apple targets choose native scroll endpoints from the supplied positions.
Gesture momentum and settling can differ between platforms.

### Carousel pages

`Carousel` packages page layout, content extents, and snapping. Each supplied
child becomes a page:

```pax
<Carousel x=24px y=24px width={100% - 48px} height=220px
    axis=CarouselAxis::Horizontal page_size=100% show_dots=true>
    <Rectangle width=100% height=100% fill=rgb(225, 234, 220) />
    <Rectangle width=100% height=100% fill=rgb(242, 218, 183) />
    <Rectangle width=100% height=100% fill=rgb(205, 224, 236) />
</Carousel>
```

Horizontal paging and `page_size=100%` are the defaults. Choose
`CarouselAxis::Vertical` for vertical pages, and bind `scroll_pos_x` or
`scroll_pos_y` when the application needs the position. `page_size` controls
each page's extent along the scrolling axis; its percentage is relative to
the Carousel viewport. With only one child, that page fills the viewport.

Dots are optional position indicators, hidden when there is only one page.
They are not clickable navigation controls. Provide explicit previous/next
buttons when your interface needs them, using the bound scroll position.

## Scroll-driven motion

Scrolling can reveal a drawing, turn a diagram, or carry a caption through
a sequence. Start with a normalized position:

`progress = scroll position / (content extent - viewport extent)`

Clamp the result to `0..1` and handle a zero-length scroll range. Then map
progress to the timeline's playhead range. The dimensions in the state
example give a travel of 480 pixels, so this addition to its `lib.pax` makes
a reading-progress bar. Add the Group before the Scroller and place the
timeline at the end of the file:

```pax
<Group x=24px y=104px width={100% - 48px} height=6px>
    <Rectangle id=reading_progress height=100% fill=rgb(56, 100, 78) />
    <Rectangle width=100% height=100% fill=rgb(210, 218, 206) />
</Group>

@timeline reading {
    duration: 100,
    playhead: {Math::min(1, Math::max(0, self.scroll_y / 480)) * 100},
    loop: false,
    #reading_progress {
        width: {
            0%: 0%, Linear,
            100%: 100%,
        },
    },
}
```

Here `duration: 100` defines the timeline's sampling range. There is no
running clock: scrolling backward samples earlier positions, and resting
leaves the bar still. A responsive version needs current viewport and content
measurements in place of the fixed `480` denominator. Keep the Scroller's
geometry independent of the decorative animation so the progress mapping
does not change its own scroll range.

The same playhead can drive several tracks. The canonical `scroll-garden`
example binds a vertical Carousel to a playhead for its articulated scenes.
[Animation and Motion](animation-motion.md#share-a-playhead) explains track
ownership, easing, and playback; [Drawing](drawing-styling.md#paths-and-svg)
owns Path and Handwriter reveals.

## Nested viewports and native content

A horizontal shelf can live inside a vertical document. Give each Scroller a
bounded viewport and its own content extent. For example, place the horizontal
strip above inside a taller content Group in the outer Scroller. Its `width`
then follows that group's width, while its 180-pixel height remains a visible
window onto the strip.

Use nesting when the regions have distinct jobs. Same-axis nesting makes
gesture ownership harder to anticipate; test what happens at each edge with
the target's trackpad, mouse, and touch input. Pax relies on the platform
scroll container for native scrolling, momentum, and gesture arbitration.

Text, form controls, vectors, and images can share the scrolling content tree.
The native host moves the content, while Pax maintains the rendering,
clipping, and input-coordinate relationships. A web root Scroller that fits
the viewport and scrolls only vertically can delegate to page scrolling;
you still use Scroller properties rather than browser DOM operations.

For a toolbar that stays in place, keep it outside the Scroller as a sibling,
usually reserving space for it in the surrounding layout. A child at `y=0px`
belongs to the scrolling content and moves with it. `layout_role=LayoutRole::Breakout`
removes a child from the relevant layout measurement/placement rules; it
does not let that child escape the Scroller's clipping or rendering tree.

Use `NodeContext::local_point` for custom pointer interaction inside moving
content. On iOS and iPadOS, scroll recognition can suppress a tap while
the child still receives its lower-level touch sequence. The
[touches-inside-a-Scroller section](event-handling-rust.md#touches-inside-a-scroller)
explains how to clear transient feedback. Check native controls and nested
gestures together on the targets you ship.

Scroller-owned rendering surfaces also matter for overlays. A translucent
root Rectangle is not a universal dimmer over every native scroll region.
[Compositing](compositing-effects.md#scroller-islands-and-modal-underlays)
owns the cross-surface explanation and modal-underlay pattern.

## Large collections and practical checks

Pax uses viewport-aware drawing and tiled surfaces to limit rendering work
for scrollable content. That does not make `for` a virtualized list: repeated
children still participate in tree expansion, properties, and lifecycle.
Offscreen components may still cost startup time and perform application work.

On native iOS, iPadOS, and macOS, surface sizing uses the current screen's
pixel density. Shrinking `scroll_width` or `scroll_height` (for example, when
zooming out on a graph) keeps the content tiled until a single surface fits
the native backing limits at that density. This avoids a sudden oversized
GPU allocation when the content crosses a tile-size threshold; it requires
no application-level zoom restriction.

Start with realistic collection sizes and measure first paint as well as
scrolling. For a large data set, consider application-level paging or loading
bounded batches. Stable keys preserve item identity during changes; they
do not defer offscreen initialization. See
[Components](components-composition.md) for collection ownership.

Exercise the finished view with its real text, images, and controls:

- Resize it and change the content count, including an empty or short list.
- Reach the first and last items, then use a position button and scroll again.
- Try nested regions, native-control interaction, and rounded edges.
- Test fast movement on the actual browser/backend and Apple devices you
  support. Startup cost and tile presentation remain workload- and
  target-dependent; a desktop web check cannot establish every target's behavior.

From the repository root, these examples offer larger inspection surfaces:

```sh
pax-cli run --path examples/src/rounded-scroller-tiles --target web
pax-cli run --path examples/src/scroll-matrix --target web
pax-cli run --path examples/src/scroll-garden --target web
```

Run one at a time. `rounded-scroller-tiles` focuses on rounded viewports and
mixed content; `scroll-matrix` exercises nesting, transforms, and controls;
`scroll-garden` explores scroll-driven scenes. Their source lives under
`examples/src/` and remains the place to follow the complete applications.

### A cinematic catalog

`examples/src/paxflix` combines a large hero with ten horizontal movie shelves
inside one vertical Scroller. Its 100 cards reuse 35 local images through stable
thumbnail paths; a selected film opens a full-resolution root-level detail panel.
Each shelf's viewport spans the screen, with gutter space inside its scrolling
content. At zero scroll the first card aligns with the heading; scrolling lets
the cards cross the fixed layout gutters and clip only at the viewport edges.
Card titles and metadata sit over a dense lower image gradient that follows
the selected dark or light theme.
The shelves remain mounted while the panel opens, preserving their scroll
positions. Short landscape windows use a side-by-side detail layout.
Its previous/next controls animate the bound scroll position with `ease_to`;
wheel or touch input cancels that animation so direct scrolling takes over.

Run it with `pax-cli run --path examples/src/paxflix --target web`. The example's
README documents the asset inventory and a repeatable culling/interaction check.
All 100 cards participate in tree expansion: use it to inspect viewport-aware
drawing and asset reuse, not as an example of list virtualization.

## Read more

- [Layout and Responsiveness](layout-responsiveness.md): parent frames,
  measurement, and responsive structure.
- [Properties](state-properties.md): bindings and derived state.
- [Event Handling](event-handling-rust.md): coordinate conversion and gestures.
- [Animation and Motion](animation-motion.md): playheads, tracks, and motion choices.
- [Compositing and Effects](compositing-effects.md): clips, masks, and native surfaces.
- [Scroller API](api/pax-std/core/scroller.md) and
  [Carousel API](api/pax-std/layout/carousel.md): property reference.
