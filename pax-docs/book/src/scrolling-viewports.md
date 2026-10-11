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

These dimensions and offsets are in the Scroller's local, unscaled coordinates.
For example, putting the 240/720-pixel example inside a `scale=50%` Group
halves its on-screen size, but its vertical scroll range remains 480 local
pixels. On web, the declared content pane bounds browser scrollable overflow;
transformed native overlays do not add empty travel beyond that extent.

Scroller does not paint a background. Place painted content inside it, or a
background sibling behind it, depending on which should move. Earlier Pax
siblings appear in front; that is why each note's Text precedes its Rectangle.
`corner_radius` is a numeric pixel radius for the viewport clip.

### Rendering density

Web vector surfaces, including tiled Scroller content, target the browser's
device-pixel ratio at their **presented scale**, in debug and release builds.
For example, a 1,000-unit-wide Scroller inside `scale=50%` appears 500 CSS pixels
wide. On a 2× display it needs about 1,000 backing pixels, not 2,000. A backing
density below one per local unit can therefore still match the screen. Becoming
scrollable or tiled does not opt content into lower-resolution rendering.
The sizing contract is shared by WebGPU and the Vello CPU fallback. Native
text retains the browser's own rasterization.

The full affine transform determines density: rotated orthogonal axes retain
their individual scales; shear uses a conservative bound. Density is rounded
up in small steps, grows immediately, and shrinks after 500ms at a stable scale
to avoid reallocating for tiny variations. Translation and pure rotation do not
change density. Singular transforms do not request drawable surfaces.

The web host checks each backing against the dimensions produced by the same
Rust sizing calculation used for allocation, including fractional-DPR rounding
and hard limits. A valid, unchanged canvas does not request repeated surface
refreshes. Discovering a backend limit invalidates the plan once; it does not
switch tiled content to a separate lower-density policy.

When a pane cannot fit the tile budget at screen density, Pax splits it into
smaller surfaces and retains a bounded window around the viewport. It does not
reduce resolution merely to keep a pane on one canvas. Hard backend/browser
surface-dimension limits still guard allocation: an exceptional clamp emits a
warning and marks the affected canvas with `data-pax-resolution-fallback`,
including actual density and the limit. This is a safety fallback,
not an adaptive scrolling-performance mode. Investigate repeated warnings;
high-density rendering still requires profiling complex or heavily animated
scenes on the devices you support.

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

On Apple hosts, content-size changes preserve the requested position while
clamping the visible viewport to the available content. If the content grows
again, that requested position becomes visible again unless native scrolling
or an application write has replaced it. macOS layout and backing-pixel
alignment do not count as user scrolling: mounting or moving a fractionally
positioned Scroller must not reset its bound offsets.

In `src/lib.rs`:

```rust
use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
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

On web, native position observations update the bound properties and render state without
commanding the host to scroll back to that sampled position. An application write
after an observation is still a scroll command, including during a gesture. This
distinction applies independently to each axis and to nested scrollers as well as
the page scroller; it avoids stale position echoes interrupting native momentum.

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
On web and macOS, a descendant's `@wheel` handler can prevent the native wheel
default with `event.prevent_default()`, for example when adjusting a dial.
Unconsumed wheel input continues to use native scrolling.
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

Run `pax-cli run --path examples/calculator --target web` from a source
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

## Observe viewport proximity

Bind `@viewport_proximity_enter`, `@viewport_proximity_change`, and
`@viewport_proximity_exit` to prepare nearby content, animate a first visible
sample, or release work after departure. Bind any subset; enter works on its
own. The element carrying the binding is the observed target, including when
it is a Group. The reference is always the **global viewport**, with ancestor
clipping and native scroll presentation applied.

```pax
<Group width=100% height=200px
    @viewport_proximity_enter=self.prepare
    @viewport_proximity_change=self.measure
    @viewport_proximity_exit=self.release>
    <Group id=card x=50% y=80px anchor_x=50% anchor_y=50% width=100% height=160px>
        <Rectangle width=100% height=100% fill=rgb(49, 96, 201)/>
    </Group>
</Group>

@timeline reveal {
    duration: 100,
    playhead: {self.playhead},
    loop: false,
    #card {
        opacity: { 0%: 0, Linear, 35%: 1, 100%: 1, },
        y: { 0%: 98px, OutQuad, 100%: 80px, },
    },
}
```

Declare `playhead: Property<f64>` and `departing: Property<bool>` on the
owning component. These handlers reveal the contents at the first sampled
positive overlap, and reverse the timeline when at most 40 logical pixels
remain visible and the visible height is decreasing:

```rust
pub fn prepare(&mut self, _ctx: &NodeContext, _event: Event<ViewportProximityEnter>) {
    // Acquire application resources here; this also runs for initially nearby content.
}

pub fn measure(&mut self, _ctx: &NodeContext, event: Event<ViewportProximityChange>) {
    let visible = event.current.viewport_intersection.map_or(0.0, |r| r.height());
    let previous = event.previous.and_then(|p| p.viewport_intersection)
        .map_or(0.0, |r| r.height());
    if visible > 0.0 && (previous == 0.0 || (self.departing.get() && visible > previous)) {
        self.departing.set(false);
        self.playhead.local().cancel_transitions();
        self.playhead.local().ease_to(100.0, Duration::Milliseconds(420.into()), EasingCurve::Linear);
    } else if visible <= 40.0 && visible < previous && !self.departing.get() {
        self.departing.set(true);
        self.playhead.local().cancel_transitions();
        self.playhead.local().ease_to(0.0, Duration::Milliseconds(180.into()), EasingCurve::Linear);
    }
}

pub fn release(&mut self, _ctx: &NodeContext, _event: Event<ViewportProximityExit>) {
    self.departing.set(false);
    self.playhead.local().cancel_transitions();
    self.playhead.set(0.0);
    // Release this target's resource lease here, and separately on unmount.
}
```

This policy compares two samples; it does not infer a physical scroll velocity.
Layout, resizing, and transforms can also change the visible height. Observe a
stable wrapper and animate its children so the animation does not change its
own measurement. The runnable `examples/viewport-proximity` example adds
270 cards with visit/sample counters, a small uniform scale from 92% to 100%,
and restoration when scrolling reverses. The explicit 50% anchors keep scaling
centered inside the stable 200px observation wrapper. At rest, `y=80px` locates
the center of the 160px card.
The example also loads field notes asynchronously when a page approaches the
viewport. Six adjacent cards share one request and a retained cache entry.
Web fetches the bundled `field-notes.txt` over HTTP; native targets decode the
same bundled fixture on a worker thread. Both introduce 350ms of latency so
scrolling away before completion is reproducible, without an external service.

`notes.rs` owns the application cache and revocable request leases. Exit and
unmount release a consumer; the shared request may finish and populate the cache.
A completion writes to a card only if its lease, nearby state, and data identity
still match. One root `on_tick` drains the completion channel on the Pax thread;
card geometry is never polled. Worker threads send plain data, not `Property`
values. The example uses fixed data IDs and a bounded session cache (45 pages),
including error results; restarting the example retries failures. In an app with
changing data IDs, release and reacquire on those changes too, and choose a cache
expiry/retry policy appropriate to the data.

Scroll inside the example to reveal cards, then reverse direction to restore
departing cards. The counters show proximity visits, delivered geometry samples,
and the current visible height.

<pax-example
  path="viewport-proximity"
  title="Close Enough — Viewport Proximity"
  height="720"
  files="src/lib.pax,src/card.pax,src/lib.rs,src/notes.rs,public/field-notes.txt">
</pax-example>

### Samples and delivery

Each `current` is a `ViewportProximitySnapshot`: `bounds` is the transformed
layout bounding rectangle, `viewport_intersection` is the positive-area
intersection after clipping (or `None`), and `in_proximity` reports proximity
membership. Coordinates are logical pixels relative to the global visual
viewport's origin. `is_in_viewport()` and `intersection_ratio()` provide
convenient derived values. The ratio uses bounding rectangle area, not opaque
pixel coverage.

| Situation | Delivery, for the handlers that are bound |
| --- | --- |
| Initially nearby, or reentering proximity | Enter with `current`, then change with `previous: None` |
| Changed bounds or intersection while nearby | Change with `previous` and `current` |
| Unchanged sample | No event |
| Leaves proximity | Final change, then exit; both share the last inside and terminal outside samples |
| Remains outside, including a jump across the entire region | No event |
| Unmounts | Observation stops; use unmount for cleanup, with no synthetic exit |

Sampling happens once per engine tick after layout settles. Enter and change
share the same frozen initial sample. Handlers can update properties; those
updates settle for rendering and are observed on the next tick. Events are
local and do not bubble or have a cancellable default action. Suspension
clears the visit history; resuming nearby produces a fresh enter/change pair.
Only the previous and current samples are retained. Keep longer histories in
application state if needed, and do not infer movement across a cold gap.

These are sampled observations. Fast scrolling can skip a first-pixel position
or the 40px band, and native scrolling may present pixels before its notification
reaches Pax. Author initial animation state in the template; this API does not
guarantee a callback before the first physical pixel appears.

### Region and cost

The MVP uses fixed margins: one **global viewport width on each horizontal
side**, and one **global viewport height on each vertical side**. Scroll clips
receive the same expansion for proximity; ordinary Frame and hard-mask clips
remain strict. Viewport intersection always uses the unexpanded clips.
`unclippable` retains its escape behavior. There is no margin or alternate
observation-root setting yet. Renderer warm-tile sizes do not affect this
public region, so backend tile policies cannot change the margin.

Contact alone and zero-area layout are outside. Rounded, rotated, and path clips
use bounding envelopes; opacity, alpha coverage, sibling occlusion, and other
windows are ignored. Use the API for geometry and preparation, not impression
or pixel-visibility measurement.

Observation is demand-driven: bindings register targets and share ancestor
and prepared geometry records. Native scrolling queries nearby content in
scroll-owner coordinates instead of rewriting every descendant's bounds.
Nested cold scroll domains are pruned, with conservative fallback for escaping
branches. Initial registration and whole-list relayout still scale with the
registered collection, and each registration uses memory. Many overlapping or
unclippable targets can increase query work. This does not virtualize the scene
or defer mount/tick handlers. With no proximity bindings, no observation
records or detailed samples are installed.

Efficient observation does not make the rest of a large scene free. Updating
labels or animating cards also exercises text layout, native view updates,
compositing, and drawing. See [updating native content](compositing-effects.md#updating-native-content)
for the incremental compositing path and its current boundaries. Profile the complete app on its intended target;
query timings alone do not establish a frame-rate guarantee. Sample collection
precedes final compositing, so handler changes can settle into the same rendered
frame without an extra full compositing pass.

Run the example from the repository root:

```sh
pax-cli run --path examples/viewport-proximity --target web
```

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

When scrolling reuses drawing tiles at new positions, Pax schedules the visible
tiles in that layer for repaint together. Offscreen warm tiles can repaint over
later frames, in travel order. This avoids deliberately splitting visible tile
updates across frames; it does not guarantee a frame rate or prevent a fast
gesture from outrunning the prepared region.

Web renderers aim for tiles roughly one viewport long along each scrollable
axis, subject to pixel rounding, backing limits, and the canvas budget. Vello
CPU preserves matching warm canvases when the tile set changes, compares their
replayed commands, and uploads only damaged regions. A replaced canvas or a
changed tile origin, size, or pixel density requires a complete repaint.

On native iOS, iPadOS, and macOS, surface sizing uses the current screen's
pixel density. Shrinking `scroll_width` or `scroll_height` (for example, when
zooming out on a graph) keeps the content tiled until a single surface fits
the native backing limits at that density. This avoids a sudden oversized
GPU allocation when the content crosses a tile-size threshold; it requires
no application-level zoom restriction.

On iOS and iPadOS, GPU backing surfaces are also limited to a nearby region
around the screen and the presented clips of ancestor native views. A shelf
far down a vertical page does not allocate GPU surfaces merely because its
own horizontal viewport contains cards. Surfaces outside that region are
released and recreated as they approach view; native Scrollers, their scroll
positions, and the Pax component tree stay mounted. The warm margin is a
renderer implementation detail, not an application lifecycle event. This
does not defer image decoding or application data loading, and it is not
a bound on all application memory. Check rapid scrolling and rotation on
the physical device as well as the simulator.


On native GPU and WebGPU backends, adding, removing, or reordering physical
tiles preserves the unchanged survivors and their retained scenes. New tiles
share the GPU context already used by that logical layer and replay only the
content they need. Origin or size changes invalidate the affected tile; a
recreated backing surface receives a new renderer even if its tile key is the
same. Pending warm replay follows surviving tile identities through reordering,
and visible affected tiles replay together before deferred warm work.

On the WGPU path, bounded canvas primitives retain a tile-independent drawing
program as well as tile-local GPU state. A new or resized tile can replay an
unchanged program without calling the component's render hooks again. Programs
include their ancestor clips, vector mask sources, and opacity scopes; paths and
paint values are snapshots, not live property handles. The cache is bounded and
shared across tiles in the same runtime. A first visit, an evicted program, or
changed content still requires ordinary component rendering. This does not
virtualize component state or eliminate image decoding/upload and GPU setup.
The Vello CPU fallback retains its existing component-replay path.

On WebGPU, ready surfaces can also replay those records **before** the next
application tick, including a coalesced wake-up after async surface completion.
At most one extra pass runs between ordinary frames. This presentation-only pass does not advance clocks, dispatch
lifecycle handlers, evaluate layout, or mount components. Pending effects,
geometry, or native-compositing changes block the pass. Removals and canvas/lighting
dirtiness block their layer; dirty content blocks the affected replay region,
not unrelated cold layers;
missing drawing records fall back to the ordinary frame. Masks and opacity use
the same captured drawing programs as normal replay. The normal tick still runs.
An adaptive CPU-cost estimate limits starting additional replay jobs, with an
initial 4ms allowance and at least one eligible job allowed to progress. An
individual upload/encode is not preemptible and can overrun the allowance. This
is a bounded pre-tick opportunity, **not** a frame-time limit on ordinary content
rendering, surface initialization, or all visible repair work. Native chassis do
not yet invoke this shared Rust path; Vello CPU remains on normal rendering.

Tile replay does not replace a simultaneous content change. When a card animates
across a tile boundary, its opacity, transform, and artwork updates must reach
the surviving tiles as well as newly exposed ones. Browser surface allocation
and tile-position changes are reconciled together; a pooled canvas checked out
again starts a new surface generation, even at the same logical tile address.
Browser canvas pooling also waits for every GPU surface and asynchronous initializer
to release the element before clearing or reassigning it. Released WebGPU canvases
are explicitly unconfigured before reuse so a previous row's presentation cannot
carry over into an animated entrance; shrinking the backing store alone is not
sufficient. This does not reset the shared GPU device or its image cache.
Rebinding a logical layer
to a different scroller host uses a fresh assignment; an old row's retained pixels
must never be exposed in the new host. Reuse is governed by ownership, not a delay.

Browser scrollers share a bounded canvas pool. In both desktop and mobile browsers,
admission is per surface across the app, not all-or-nothing per scroller.
The Web adapter supplies measured geometry, movement, and readiness to a shared
Rust policy; it does not maintain a separate JavaScript ranking algorithm.
This shared policy is currently enabled on Web; native chassis activation is
a separate rollout. Web admission limits both surface count and physical backing
pixels. Speculative coverage has a base allowance of two screenfuls of pixels.
After ordinary admission, forward coverage may borrow unused capacity while
leaving four canvas slots and one screenful of backing pixels as headroom for
new visible demand. Borrowing does not increase the overall caps or displace
already-admitted near/safety coverage. New visible demand always takes priority;
the headroom limits borrowing, not mandatory rendering. Assigned, initializing, and
retiring backings remain charged until their allocation or last lease releases.
These are color-backing estimates, not a bound on total GPU or application memory.
Visible coverage, including stationary siblings and the root page, precedes
speculative preparation. Actual local scroll movement prioritizes preparation
ahead of moving scrollers, with a 250ms activity tail and immediate direction
reversal. Hover alone does not claim priority; input remains native.
Among equally near candidates, already-submitted content wins the tie; having
a canvas assigned does not by itself mean the content is ready.

Admitted warm hosts stay presentable under their existing native scroller and
Frame clips. They are not CSS-hidden until a later visibility tick: browser
scrolling can reveal prepared pixels before the application thread runs again.
Cold, unallocated hosts may still be parked. This does not expand clipping bounds
or change viewport-proximity events.

Preparation follows the clipped exposure of the whole scroll ancestry. A moving
vertical catalog prepares the portion of an upcoming horizontal shelf that can
actually appear; it does not grant that idle shelf an independent horizontal
warm window. Simultaneously moving ancestors and children share one bounded
prediction horizon. Currently exposed scrollers also request a half-viewport
safety margin on either side of each scrollable axis, even at rest. These are
separate possible movements: parent and child safety margins are not multiplied
into a larger diagonal/nested region. Offscreen children inherit only the
exposure their ancestors can reveal. The margin is independent of tile size,
so stopping exactly at a tile boundary does not remove first-scroll coverage.

After movement identifies an axis, its currently exposed scroller also requests
one local viewport of **reverse coverage** opposite its last direction. Unlike
retention, this can initialize missing tiles. It persists through pauses and
switches sides on the next observed reversal. Hidden descendants inherit the
parent's reverse exposure, not an additional warm window on their own axes.
Admission funds this buffer after visible demand but before forward speculation,
within the same canvas and pixel limits. Execution still completes visible work
and predicted incoming tiles before replenishing reverse coverage. This separates
space reserved for reversibility from the order of drawing work; overload or
insufficient preparation time can still leave the requested buffer incomplete.

Near already-submitted tiles remain eligible within a one-viewport retention
margin through pauses and reversals. Retention starts no new work and has lower
priority than visible, predicted-forward, and safety coverage. It yields under
the same canvas/pixel budgets or when the tile moves too far away; expiration
of the 250ms motion signal alone does not discard it. The candidate bounding
rectangle includes this retain-only area, but cold tiles there are not admitted.

Forward prediction uses observed speed and a bounded preparation-time estimate.
The Web adapter measures from assignment until first observing submission,
including queueing and first drawing, once per content revision. The shared
policy adapts its horizon between 250 and 750ms, capped at six local viewports
of travel. This is conservative CPU-side readiness feedback, not a guarantee
that asynchronous image loading or compositor presentation has completed.
These distances are initial policy calibration, not per-Scroller authoring APIs.
Beyond that urgent prediction, recently moving axes offer a lower-priority
forward reserve of up to six local viewports. It fills only after visible,
predicted, and stationary-near coverage, within the existing preparation-time
allowance. Paused reserves drop in priority, with a further downgrade after
1.5 seconds, but can finish filling during idle time while capacity permits.
Unchanged observations across a long idle sampling gap preserve this remembered
direction without refreshing urgent motion. Reversal immediately switches the
reserve direction, and new moving branches precede paused reserves.
An offscreen child inherits the ancestor's exposure,
not additional independent-axis overscan. The opposite direction uses the
one-viewport reverse buffer rather than the six-viewport forward reserve. These are capacity-limited candidate
windows, not a promise that all six viewports are already rendered. Abrupt jumps
outside ready coverage can still reveal unfinished content.
All optional work remains subject to the existing global budgets.

Tiles stay anchored in content space;
changing a clip or preparation priority does not resize or rename survivors.
WebGPU scrolling-axis cells are capped near one host viewport, rounded upward
in 256-physical-pixel buckets and subject to the existing backend size/area
limits. A narrow embedded shelf therefore does not prepare a long row of
offscreen posters merely because one large tile intersects the viewport. This
changes preparation granularity, not raster density. The grid uses the host's
viewport, not its changing ancestor clip; resizing or changing projected density
can still require new cells. Native and Vello CPU retain their prior sizing policy.

Coverage uses transformed presentation bounds, so scaling an embedded app
does not mix local dimensions with screen-space distances. Affine projection
preserves rotation/reflection; nonrectangular clips use conservative bounds,
not exact pixel occlusion. Empty ancestor layers do not consume a canvas merely
to retain their DOM ancestry. Obsolete canvas assignments across layers retire
before replacement allocation; GPU ownership must still finish before reuse.
Scrolling through a long collection can release distant drawing surfaces
and recreate them on return. This does not unmount application components or defer
image decoding. The pool size and warm distances are implementation details, not
viewport lifecycle thresholds or a bound on all application memory.

Application-generated or virtualized content needs its own bounded preparation
region. For example, a plot that creates paths only for the current viewport
has no geometry to paint just beyond it, regardless of available warm canvases.
Generate or mount nearby content before exposure; choose that application-level
margin with its generation cost in mind. Surface preparation does not replace
application content preparation.

If visible demand itself exceeds the pool budget, the host preserves valid
visible survivors and admits remaining visible work deterministically, without
speculative work, a larger cap, or lower density. The mount's
`data-surface-budget-state` reports `visible-demand-exceeds-budget`.
`data-surface-backing-bytes-estimate` estimates color backing bytes, **not total
GPU memory**: textures, attachments, and caches cost more.

WebGPU initialization can wait asynchronously for a device. When the shared
device is already ready, eligible additions can initialize during queue servicing
without waiting for another microtask. Cold-content preparation admits creation
one unit at a time under the cooperative allowance described below; the legacy
web initialization path permits up to four attempts per queue service.
Remaining work is revisited during subsequent layout/presentation passes.
Existing tiles remain usable while additions
initialize, and results for removed or replaced surfaces are discarded.
The host publishes an installed plan after retirement and allocation; pending
allocations are not initialized targets. Browser initialization consumes the same
global rank as admission, reconsidering it between individual surface creations.
After waiting for the shared GPU context, it selects again from the current
installed plan and yields to more urgent uninitialized surfaces. Queued replay
is also reranked on plan changes; work for removed assignments is discarded.
A tile does not retain priority merely because it was previously visible or
missed a predicted exposure time.
Visible replay precedes speculative replay across layers. Priority changes do
not change surface identity. For WebGPU cold-content preparation, the same
ordering spans initialization and drawing: collecting completed surfaces does
not start more creation. A runnable visible tile continues through capture and
submission before speculative initialization, and already-ready visible work
wins ties with new visible creation. Work is selected again from current demand
after each unit; a blocked tile does not prevent eligible siblings progressing.
This does not cancel initialization already in flight or guarantee that an abrupt jump cannot
expose an unfinished tile.

WebGPU also uses that ordering for cold first drawing. Missing vector/image
drawing commands can be recorded in small steps, retaining ancestor clipping,
mask-source, and opacity state without uploading or presenting a partial tile.
Completed commands are replayed and submitted one physical tile at a time;
remaining work is reconsidered against the current installed plan. Ordinary
application changes still update their drawings, including animations while a
tile is cold. This does not defer component mounting or lifecycle handlers.

The shared runtime gives these preparation passes a cooperative 4ms CPU
allowance: it stops starting more work based on elapsed time and recent cost,
while allowing one eligible unit to make progress. The synchronous portion of
one surface initialization, a primitive capture, or a tile submission is atomic
and can overrun the allowance. Async initialization retains its existing slot
and completion safety checks, then wakes presentation rather than recursively
starting speculative initialization. If commands cannot fit in
the bounded cache, an atomic direct draw of the selected tile preserves
correctness instead of retrying forever. Shader-effect subtrees also use that
atomic tile fallback: their capture boundary cannot be replayed as independent
cached leaves. Ordinary scene updates, browser image
decoding, and asynchronously awaited GPU/device work are not covered by this allowance. Preparation
reselects the next eligible batch after each submission, using the remaining
allowance rather than stopping after one batch per layer. A blocked layer waits
until the next pass while eligible siblings can continue; this does not relax
the settled-scene checks or submit incomplete tiles.
Within ordinary rendering, WebGPU services newly visible surfaces after current
geometry, lighting, and removal setup, before unrelated drawing. Ordinary dirty
drawing then completes visible logical layers before warm/offscreen layers,
including their inherited clips, masks, and opacity. Newly completed visible
layers can prepare their cold tiles immediately, without waiting for unrelated
ordinary drawing. This split preserves order within each layer; unclippable or
unresolved render paths retain the full-tree fallback. Failed removals or flushes
keep their retry flags and block settled preparation on that layer.
Unmounted nodes retire their pending drawing/replay requests without discarding
the GPU removal request. A late invalidation of an unmounted node cannot force
unrelated drawing back to full-tree traversal; remounting schedules fresh drawing.
Any unused preparation allowance services remaining visible and warm work
afterward. All these opportunities share one allowance and one guaranteed first
unit—not a fresh allowance at each boundary. Ordinary drawing itself is not
budgeted or deferred across application ticks by this policy.
Asynchronous completions can request one additional coalesced opportunity between
ordinary frames. Preparation never advances the application clock. This first-draw policy is
currently enabled for WebGPU, not Vello CPU or native chassis; the implementation and
opt-in are in the shared Rust runtime.
If an addition fails, survivors remain usable; another attempt waits for a layout
change. New tiles still incur surface allocation and first-draw costs. Sibling
tiles using the same GPU context share immutable image textures by image identity,
version, and pixel dimensions. A tile can reuse an image already held by a sibling
without uploading its pixels again; transforms, clipping, opacity, and draw bindings
remain tile-local. WebGPU canvases within one mounted app share a device and its
pipeline/immutable-image caches across logical layers, including when rows go cold
and return. This avoids repeating device and pipeline initialization during scrolling.
Native GPU sharing remains within each logical layer. These behaviors apply in debug
and release, not the Vello CPU fallback or native image controls. Separate apps or GPU
contexts do not share textures.

For browser diagnosis, add `?pax_surface_trace=1` to a debug or release URL.
Invalid scroller geometry (negative extents, non-finite measurements, or a
singular transform) makes only that scroller and its descendants unavailable
to surface preparation. Unrelated branches continue rendering. A valid later
observation automatically restores the branch; stale coverage is not retained
as a substitute for valid geometry. Tracing reports `preparation-branch-rejected`
and `preparation-branch-recovered`, with the host ID and rejection reason.
This branch isolation is shared scheduler policy, currently exercised by Web.
It does not make malformed application layout valid. In particular, pausing an
embedded example's animation must not skip its initial layout or resize handling.

The app mount exposes a local `paxSurfaceTrace` array (at most 2048 events) for
visible demand (including denied tiles), admission, installed plans, refreshes,
initialization queue/start/completion or rejection, retirement, lease release,
and first submission. Installed-plan events include assigned, retiring, and
available canvas counts; waiting for a lease is distinct from impossible visible
demand. Canvas `data-surface-*` attributes expose generation, content
revision, and timestamps. A submitted revision means a frame was submitted for
that tile—not that every asynchronous image has loaded. Image events separately
report requests/coalescing, decoding and CPU publication, missing decoded pixels,
GPU uploads/cache hits, changed image drawings, and their subsequent surface
submission (`image-content-submit`). The snapshot includes cumulative image
counts and bounded waiting/recorded-not-submitted state. Submission is a CPU-side
presentation event, not proof of GPU completion or a capture of every displayed
pixel; these diagnostics do not establish completeness of arbitrary application
content. Image GPU events apply to WGPU, not the Vello CPU fallback.
The snapshot also includes cumulative surface-completion wake-up outcomes:
queued, coalesced with an existing request, or deferred because the extra pass
was already used. `presentation-completion-wait` measures the delay from the
first such deferral to the next service opportunity and reports whether scene
state blocked that opportunity. It does not measure a particular tile's latency
or establish that additional passes would improve throughput.
Tracing is opt-in and sends no data anywhere. Profile component mounting and
image decoding separately; graphics admission cannot remove those costs. The isolated
`examples/fixtures/surface-scheduling` fixture has no external images.

Admission events include each tile's owner, local `[left, top, width, height]`,
rank, and submitted-readiness flag, plus borrowed forward pixels and the
borrowing ceiling. These distinguish allocated depth from submitted coverage;
the farthest ready tile alone does not establish a gap-free forward buffer.

Open **Surface trace snapshot** in the corner to inspect the bounded event log;
close and reopen it to refresh. `visible-unready` transitions preserve transient
gaps even after the view settles. Add `&pax_scroll_perf=1` to include frame phases
taking over 20ms, including individual `tick.*` phases. `settled-presentation`
events report started initializations, recorded drawings, submitted/deferred jobs,
cache misses, atomic fallbacks, and CPU elapsed time for preparation. The `source`
field distinguishes `ordinary-visible`, `ordinary-remaining`, and `completion`
opportunities; `dirty_layers` counts jobs deferred by layer safety checks.
`unfiltered_layers` counts early captures deferred because an unclippable or
otherwise unsupported render path needs full-tree traversal while other work is dirty.
`render.geometry`, `render.lighting`, `render.reconcile`, `render.draw`, and
`render.flush` distinguish ordinary setup/draw/submission costs from
`render.visible_preparation` and `render.remaining_preparation`.
When ordinary drawing can be split, `render.visible_draw`, `render.visible_flush`,
and `render.visible_dirty_preparation` measure the early dirty-layer group;
`render.draw` and `render.flush` then describe the remaining group. The
`render.partition_*` phases identify an unclippable or unresolved path that
required unsplit traversal. Pending nodes on clean, cold layers do not disable
the split; they remain pending outside the visible group. The
`ordinary-visible` report aggregates both early preparation opportunities; its
`deferred` count describes the latest opportunity rather than summing queue sizes.
Earlier CPU-side
submission does not prove earlier browser presentation if a long synchronous
task still blocks the host.
`occlusion-work` includes cumulative native-compositing counters and a `delta`
since the preceding report. Rebuild reason/node and up to eight aggregated
structural-invalidation call sites are reported only when that interval contains
a rebuild. The call-site records include a representative node and count; they
are bounded diagnostics, not an exhaustive call log. Do not interpret cumulative
record counts or a historical rebuild as work performed in the current frame.
`records_updated`, `masks_evaluated`, and `mask_candidates` count incremental
work only; a structural rebuild can be expensive even when those deltas are zero.
Runtime timings distinguish `tick.property_import`, `tick.property_cleanup`,
and `tick.property_effects` from `tick.imperative_update`, `tick.tick_handlers`,
and `tick.pre_render_handlers`. Import/cleanup/effect phases can occur several
times within one tick; sum their samples for a per-frame total. Handler timings
exclude the following settlement pass. These are CPU phase boundaries, not GPU
completion or compositor-presentation timestamps.

To isolate browser Frame clipping during nested-scroll testing, add
`&pax_frame_clip=box`. This experimental web path uses sized CSS boxes with
`overflow: clip` and rounded corners for explicitly identified rectangular
Frames, preserving descendant world coordinates with an inverse inner transform.
It does not create another scroll container. Arbitrary Mask paths, singular or
invalid transforms, and unsupported browsers retain the SVG path. Omit the flag
or use `pax_frame_clip=svg` for the existing default. Compare on the same build,
viewport, and input mode; physical trackpad and cross-browser behavior still
need validation before making box clipping the default.
`scroll-command` records engine-originated browser position writes, including
their axis and whether the host was active; ordinary native observations should
not produce these writes.
The inspector's **Probe scroller** and **Probe duration** controls
can move one vertical viewport in 200, 400, or 1,000ms in either direction. Probe
events record actual callback intervals and detailed frame phases; this is
programmatic scroll input, not a physical touch/momentum test or a capture of
every compositor-presented frame. Continuous input can still expose preparation
delays that this probe misses, because its input stops while the main thread is
busy. Native scrolling does not. With both diagnostic flags enabled,
`scroll-sample` records observed offsets and `scroll-phase` records phase timings
through a 250ms tail after observed movement. These measurements start when the
main thread samples movement, not when the compositor first displays it.
`scroll-reversal` identifies per-axis direction changes between observed offsets;
subpixel changes of at most 0.1 local units are ignored. Surface lifecycle events
include their current demand coverage, owner, and installed replay priority, so
work started after a reversal can be distinguished from earlier in-flight work.
These diagnostics do not synthesize scroll intent or measure GPU preemption.
`native-message-batch` breaks synchronous message processing down by message
type: count, total milliseconds, and slowest call. It records during probes and
observed scrolling, or when the batch exceeds 20ms. This includes patch parsing
and handler work, including any forced layout inside a handler; it excludes
deferred browser layout/paint and asynchronous image decoding. Comparing these
counts and times helps distinguish many cheap frame updates from a few expensive
mask or text updates. Timing itself adds overhead, so also test without tracing.

With both flags enabled, `wheel-input` separates event timestamp-to-handler-entry
delay (`deliveryDelayMs`) from synchronous Pax dispatch (`handlerMs`), and records
trusted/cancelable input, deltas, cancellation requests, and observed default
prevention. Incompatible timestamps report `null`, not zero. The inspector's
`wheel-input-summary` keeps the last 128 samples independently of tile-log churn.
Delivery delay includes browser dispatch/queueing; neither timing measures when
the compositor first moved pixels or identifies a physical gesture boundary.
Use actual wheel/trackpad input for this test: the programmatic probe does not
exercise a cancellable wheel listener.

For an explicit diagnostic comparison only, add `&pax_wheel_passive=1` alongside
both flags. This makes the global wheel listener passive while still dispatching
Pax handlers. **Wheel cancellation is disabled in this mode**: do not use it to
validate custom wheel controls or scroll-blocking behavior, or as a deployment
setting. Without both tracing flags, it has no effect. The normal path retains
synchronous cancellation and performs no per-wheel timing. Compare the same
content, warmed state, and input sequence; passive handlers may still be delivered
late while the browser scrolls independently, so handler delay alone cannot prove
or disprove smoother presentation.

Settled screenshots do not establish that every intermediate frame was complete.
The preparation allowance does not cover ordinary rendering and is not a hard
frame-time guarantee. Pixel admission alone does not cap frame time or eliminate
cold-jump gaps.

Readiness covers the surface revision, not application-level
draw-ahead: a graph generated only for the current viewport can still expose
empty regions during native scrolling, even on an already-submitted canvas.

The image index holds weak references: after the last tile drops its binding,
Pax retains no unused image texture for future scrolling (in-flight GPU work may
still reference it). Returning later can require another upload. This does not
change decoded-image ownership or defer application initialization.

Removed surfaces are released. Native logical-layer contexts are released when
their last tile and pending creation release them; the browser retains its shared
device/pipeline context until the mounted app is disposed. Unused image textures
are still released when their last binding disappears. The retention region does
not grow. These GPU optimizations apply in debug and release builds. Vello CPU
also preserves matching canvases and their replay history when its tile set
changes, but does not use the GPU drawing-program or texture caches. Neither
backend defers component or image initialization in application code.

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
pax-cli run --path examples/fixtures/rounded-scroller-tiles --target web
pax-cli run --path examples/fixtures/scroll-matrix --target web
pax-cli run --path examples/fixtures/scroll-garden --target web
```

Run one at a time. `rounded-scroller-tiles` focuses on rounded viewports and
mixed content; `scroll-matrix` exercises nesting, transforms, and controls;
`scroll-garden` explores scroll-driven scenes. Their source lives under
`examples/` and remains the place to follow the complete applications.

### A cinematic catalog

`examples/paxflix` combines a large hero with 21 horizontal movie shelves
inside one vertical Scroller. Its 210 cards reuse 35 local images through stable
thumbnail paths; a selected film opens a full-resolution root-level detail panel.
Each shelf's viewport spans the screen, with gutter space inside its scrolling
content. At zero scroll the first card aligns with the heading; scrolling lets
the cards cross the fixed layout gutters and clip only at the viewport edges.
Card titles and metadata sit over a dense lower image gradient that follows
the selected dark or light theme.
By default, each card observes a fixed wrapper through `@viewport_proximity_change` and
slides and fades its title and metadata over a 480 ms timeline at the first
visible sample; the artwork stays still and opaque. The shelf staggers nearby
arrivals by 80 ms, with no more than 240 ms of added delay. There is no exit
animation. `@viewport_proximity_exit` rearms
the entrance only after a full departure from proximity, so small reversals
at a viewport edge do not restart it. This animation does not defer image
initialization or unmount the card.
The shelves remain mounted while the panel opens, preserving their scroll
positions. Short landscape windows use a side-by-side detail layout.
Its previous/next controls animate the bound scroll position with `ease_to`;
wheel or touch input cancels that animation so direct scrolling takes over.

Run it with `pax-cli run --path examples/paxflix --target web`. The example's
README documents the asset inventory and a repeatable culling/interaction check.
All 210 card trees mount with each Paxflix instance. The prior provisional
scroll-dependent shelf gate has been removed: GPU preparation must not silently
schedule application lifecycle work. Card entrance animations still use visible
proximity samples, and GPU tiles/bindings can be prepared or retired independently
of those mounted cards. Hosts can set `disable_card_entrances=true` on `Paxflix`
to show captions at their final position and opacity without mounting the
per-card proximity observers or per-shelf entrance scheduling subscriptions.
The standalone default remains animated; image readiness is independent of this
option. Toggling it changes only the animation helpers, not card lifetime.

The website uses this quiet mode and mounts both instances (420 cards) with the
page, sharing image resources in the same web app. Its calculator also mounts
with the page; visibility still pauses recurring offscreen animation/work, not
initial layout or mounting. Removing these application-level approach gates
moves work to startup rather than eliminating it. This is not list virtualization
or runtime lazy evaluation.

## Read more

- [Layout and Responsiveness](layout-responsiveness.md): parent frames,
  measurement, and responsive structure.
- [Properties](state-properties.md): bindings and derived state.
- [Event Handling](event-handling-rust.md): coordinate conversion and gestures.
- [Animation and Motion](animation-motion.md): playheads, tracks, and motion choices.
- [Compositing and Effects](compositing-effects.md): clips, masks, and native surfaces.
- [Scroller API](api/pax-std/core/scroller.md) and
  [Carousel API](api/pax-std/layout/carousel.md): property reference.
