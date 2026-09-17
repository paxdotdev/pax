<a id="animation--motion"></a>

# Animation and Motion
<!-- summary: Timelines, shared playheads, Rust easing, lifecycle transitions, and animated layout changes. -->
<!-- tags: animation, motion, timelines, easing, transitions -->

Motion can help someone follow a change: a new note arrives, a panel settles
into place, or a list closes the space left by a removed item. Pax lets you
describe that motion alongside the interface, with Rust controlling the
state and actions that start it.

This chapter builds on [Properties](state-properties.md),
[Events](event-handling-rust.md), and [Layout](layout-responsiveness.md).
Start with a property timeline, then give several tracks a shared playhead.
Later sections cover enter/exit transitions and moving siblings in a Stacker.

For a hands-on preview, try [Transition Grid](#try-it-transition-grid):
insert, remove, and reorder tiles while comparing their motion policies.

<a id="timelines-and-easing"></a>

## Timelines

A timeline is a sequence of property values at positions in time. Pax
samples between those values as its playhead moves. Here, a small marker
brightens and fades over a repeating 1.2-second cycle:

```pax
<Ellipse x=24px y=24px width=20px height=20px fill=rgb(56, 100, 78)
    opacity=@timeline {
        duration: 1.2s,
        loop: true,
        0%: 0.3, InOutQuad,
        50%: 1.0, InOutQuad,
        100%: 0.3,
    }
/>
```

This can go in a component's template with `use pax_kit::*;` in its Rust file.
The inline timeline supplies the Ellipse's `opacity`. Each keyframe gives a
marker, a value, and optionally an easing curve for the segment that
**leaves that keyframe**. The first segment brightens from 0.3 to 1; the
second fades back to 0.3.

### Markers, duration, and loops

Write an explicit `duration` so the timeline's scale is clear:

| Duration | Clock |
| --- | --- |
| `300ms` or `0.3s` | Elapsed milliseconds from the chassis's monotonic clock |
| `18` or `18f` | Runtime frame count |

Use seconds or milliseconds when an interaction should take a particular
amount of time across devices. A frame-based animation lasts for that many
runtime ticks; its wall-clock duration depends on how those ticks arrive.
Pax uses a nominal 60 frames per second when converting mixed authoring
units, which is a conversion rule rather than a rendering-rate promise.

Markers can likewise be percentages, frame positions, or time values:
`50%`, `12`, `12f`, `150ms`, or `0.15s`. Percent markers make it easy to
change the overall duration while preserving the rhythm. For time-based
tracks, prefer percentages or matching time units; a bare `12` marker still
means frame 12.

Ordinary timelines loop by default. Use `loop: false` to clamp the playhead
to the range and keep the final value after the end. For a seamless loop,
give the first and last keyframes matching values, as above. Frame loops
include both frame zero and the final frame.

Without an explicit `playhead`, ordinary timelines sample the application's
`$frames` or `$millis` clock. Inserting an element later does not give that
ordinary timeline a fresh local clock. Use an [enter transition](#structural-transitions)
for motion that should start when the element appears, or supply a playhead
you control.

### Easing

Easing changes how a value moves between two keyframes. A missing curve
uses Linear.

| Curve | Character |
| --- | --- |
| `Linear` | Constant progress through the segment |
| `Hold` | Keep the first value until the segment ends |
| `InQuad` | Begin slowly, then accelerate |
| `OutQuad` | Begin quickly, then settle |
| `InOutQuad` | Accelerate and then settle |
| `InBack`, `OutBack`, `InOutBack` | Anticipation, overshoot, or both |

OutQuad is a useful starting point for a small arriving surface; InOutQuad
can soften a movement between two resting positions. Back curves can exceed
the interval between the endpoints, so leave room for the overshoot and
avoid applying it indiscriminately to bounded values such as opacity.

Interpolation also depends on the property type. Numeric values, sizes,
rotations, and colors have interpolation behavior; arbitrary data does not
necessarily have a meaningful in-between value. Path command lists do not
currently interpolate their coordinates automatically. To change a shape,
animate numeric parameters and derive its geometry from them, as in the
`mouse-animation` example. For a reveal, animate a Path's `draw_end`; see
[Drawing](drawing-styling.md#reveal-a-stroke).

## Share a playhead

A named timeline can coordinate several properties on elements selected by
ID or class. A shared playhead lets a button, a slider, or application logic
drive the whole sequence together.

Here is a complete small playback example. In `src/lib.rs`:

```rust
use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub playhead: Property<f64>,
}

impl Example {
    pub fn replay(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playhead.cancel_transitions();
        self.playhead.set(0.0);
        self.playhead.ease_to(
            100.0,
            Duration::Milliseconds(900.into()),
            EasingCurve::Linear,
        );
    }

    pub fn pause(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playhead.cancel_transitions();
    }
}
```

In `src/lib.pax`:

```pax
<Button x=24px y=24px width=110px height=40px label="Replay"
    @button_click=self.replay />
<Button x=148px y=24px width=110px height=40px label="Pause"
    @button_click=self.pause />
<Slider x=24px y=84px width=280px height=32px
    min=0.0 max=100.0 step=1.0 value=bind:self.playhead />

<Rectangle id=marker y=156px width=40px height=40px
    corner_radius=8 fill=rgb(56, 100, 78) />
<Text id=caption x=24px y=224px width=280px height=40px
    text="Ready for another page"
    style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />

@settings {
    #marker { x: 24px }
}

@timeline reveal {
    duration: 100,
    playhead: {self.playhead},
    loop: false,
    #marker {
        x: {
            0%: {$base}, OutQuad,
            100%: {$base + 200px},
        },
        rotate: {
            0%: -12deg, OutQuad,
            100%: 0deg,
        },
    },
    #caption {
        opacity: {
            0%: 0,
            40%: 0, OutQuad,
            100%: 1,
        },
    },
}
```

Replay advances `playhead` from 0 to 100 over 900 milliseconds. The template
maps that one value to the marker's position and rotation and the caption's
opacity. Pause stops the Rust-side easing at its current value. Press Pause
before dragging the slider to scrub without an active animation writing
to the same property.

The timeline's `duration: 100` establishes a 0–100 frame-position range.
Because `playhead` is supplied explicitly, it acts here as a convenient
sampling scale; the Rust easing determines how long playback takes.

A numeric playhead uses the track's clock units: frames for a frame-based
duration and **milliseconds** for a time-based duration, including one
written in seconds. A duration-valued playhead such as `{(self.seconds)s}`
carries its units through conversion. A numeric playhead is not automatically
normalized to 0–1.

Tracks inherit the enclosing timeline's duration, playhead, and loop setting
unless overridden on the track. Multiple named timelines can bind to the
same property when separate groups need the same progress. Naming the
timeline organizes its tracks; playback comes from its clock or playhead,
rather than an implicit Rust method named after it.

## Relative values and property ownership

`$base` means the value underneath this setting or timeline layer. In the
playback example, `#marker { x: 24px }` supplies the base position, so the
track moves from 24px to 224px. Changing that base moves the whole animation
without repeating the layout value in every keyframe.

It does not mean the previous animation frame or the parent's property.
Keyframe expressions remain reactive, so changing a dependency can change
the sampled motion. See [PAXEL's base values](data-binding-expressions.md#base)
for the general model.

Keep one clear owner for each animated value. For ordinary selector
timelines, an inline assignment to that same property takes precedence:
adding `x=24px` directly to the marker above would hide the timeline's
`x` track. Put the base in a settings rule, as shown, or assign a property
timeline inline. Lifecycle transitions have their own overlay behavior;
do not generalize this ordinary-settings rule to `@in` and `@out`.

Also avoid animating a state property while a handler continually sets it,
or trying to ease a derived value whose formula keeps recomputing. Animate
an owned source property and let its dependents follow.

## Imperative easing

Rust can animate a property directly with `ease_to`. It replaces that
property's pending transition queue and begins from its current eased value.
`ease_to_later` appends another segment. For a component with a
`Property<f64>` named `strength`, this handler rises, holds, and settles:

```rust
pub fn pulse(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
    self.strength.ease_to(
        1.0, Duration::Milliseconds(180.into()), EasingCurve::OutQuad,
    );
    self.strength.ease_to_later(
        1.0, Duration::Milliseconds(120.into()), EasingCurve::Linear,
    );
    self.strength.ease_to_later(
        0.2, Duration::Milliseconds(300.into()), EasingCurve::InOutQuad,
    );
}
```

Bind `strength` to the visual property you want to animate. Triggering the
handler again replaces the unfinished pulse; it does not accumulate an
ever-longer queue.

Passing a plain number to either easing method means frames.
`Duration::Frames(18.into())` is explicit frame timing;
`Duration::Milliseconds(300.into())` and `Duration::Seconds(0.3.into())`
express elapsed time.

`cancel_transitions()` stops the active segment, clears the queue, and
leaves the current eased value in place. Call it before `set(...)` when an
immediate edit should take ownership. A plain `set` does not cancel a
previously queued animation.

Rust also accepts `EasingCurve::Custom` with a function. That is a Rust API;
arbitrary custom easing closures are not part of template timeline syntax.
See the [animation API](api/pax-runtime-api/animation.md) and
[property API](api/pax-runtime-api/properties.md) for the full method and
value-type contracts.

<a id="structural-transitions"></a>
<a id="in-transitions"></a>
<a id="out-transitions"></a>

## Enter and exit transitions

Use `@in` and `@out` for motion tied to an instance entering or leaving the
mounted tree. Their clocks start locally at the transition, and playback
is finite. Newly entering content can animate while old content is still
finishing its exit.

For a reusable note surface, declare this component in `src/note_card.rs`,
then add `pub mod note_card;` and `use note_card::NoteCard;` to `src/lib.rs`:

```rust
use pax_kit::*;

#[pax]
#[file("note_card.pax")]
pub struct NoteCard {
    pub title: Property<String>,
}
```

In `src/note_card.pax`:

```pax
<Text x=16px y=16px width={100% - 32px} height={100% - 32px}
    text={self.title}
    style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />
<Rectangle width=100% height=100% corner_radius=12
    fill=rgb(237, 241, 226) />

@settings {
    @in: enter
    @out: exit
}

@timeline enter {
    duration: 300ms,
    self {
        opacity: { 0%: 0, OutQuad, 100%: {$base}, },
        y: { 0%: {$base + 16px}, OutQuad, 100%: {$base}, },
    },
}

@timeline exit {
    duration: 220ms,
    self {
        opacity: { 0%: {$base}, InQuad, 100%: 0, },
        y: { 0%: {$base}, InQuad, 100%: {$base - 12px}, },
    },
}
```

Here `self` targets the NoteCard instance itself, including its position
within the calling template. The named timelines can also use `#id` or
`.class` selectors for elements inside NoteCard's template.

Add a `Property<bool>` named `show_note` to a parent and toggle it from an
event handler. This conditional placement gives the card something to
enter and leave:

```pax
if self.show_note {
    <NoteCard x=24px y=96px width=280px height=84px title="Moss and rain" />
}
```

### Retention and interrupted motion

Removing the card from the conditional starts its exit. Pax retains the
mounted instance until its exit finishes, then unmounts it. The application
state has already changed; the retained visual gives the change time to
read. Cleanup tied to unmount happens at actual unmount, not at the first
request to leave.

The runtime enforces a five-second exit timeout to avoid retaining a node
indefinitely. An exit that exceeds that limit can be truncated with a warning.
Keep lifecycle exits short; use ordinary playback for a longer presentation.

<a id="interrupted-in--out"></a>

Toggle the same conditional back before the exit completes and Pax can
rescue its still-mounted instance. The default `interruption: Takeover`
starts the destination transition from the currently sampled property
value. The destination's remaining keyframes, duration, and easing still
apply. This preserves value continuity; it does not promise the velocity
continuity of a physical spring.

Set `interruption: Restart` in the destination timeline when it should
start from its authored first value instead. The option belongs beside
`duration` in either a named or inline lifecycle timeline. It affects
direct enter/exit reversals on the **same mounted instance**, without
changing `$base` or ordinary timeline playback.

Conditional and route branches can reuse retained instances during a
reversal. In repeated lists, stable keys make that identity explicit:
reordering a retained item keeps its instance, and removing its key can
start an exit. A newly allocated, unrelated instance has no earlier motion
to take over. See [Lists and identity](components-composition.md#keyed-lists)
and [Routing](routing.md).

### Element-level transitions

For a local effect, put property tracks directly in an element's inline
transition:

```pax
<Group x=24px y=24px width=280px height=84px
    @in=@timeline {
        duration: 300ms,
        opacity: { 0%: 0, OutQuad, 100%: 1, },
    }
    @out=@timeline {
        duration: 220ms,
        opacity: { 0%: 1, InQuad, 100%: 0, },
    }
>
    <Text x=16px y=16px width=248px height=52px text="A brief note"
        style={font: "Arial", font_size: 18px, fill: rgb(36, 54, 47)} />
    <Rectangle width=100% height=100% corner_radius=12
        fill=rgb(237, 241, 226) />
</Group>
```

Element bindings also accept names, such as `@in=panel_enter`. That
timeline belongs to the containing component: `self` addresses the element
that carries the binding, while selectors can reach other elements in the
same containing template. A reusable component's own settings-level
transition, like NoteCard's, targets its own template.

## Container-owned motion

A child's enter/exit transition controls its visual motion. Stacker
separately controls the space allocated to children and how that layout
changes. There are two independent choices:

| Setting | Choices |
| --- | --- |
| `exit_mode` | `Flow` keeps exiting children in normal layout until exit completes. `Ghost` holds their previous frames as overlays while active children lay out without them. |
| `reflow_transition.kind` | `Snap` moves immediately to the new layout. `Ease` animates between layout frames. |

The defaults are Flow and Snap. To let surviving notes move into an
exiting note's space immediately, choose Ghost with Ease. For a parent
whose `notes` property contains items with stable `id` and `title` fields:

```pax
<Stacker x=24px y=24px width=280px height=320px gutter=12px
    exit_mode=ContainerExitMode::Ghost
    reflow_transition={
        kind: ContainerReflowTransitionKind::Ease
        frames: 18
        curve: ContainerReflowCurve::OutQuad
        name: ""
    }>
    for note in self.notes key note.id {
        <NoteCard title={note.title} />
    }
</Stacker>
```

Stacker reflow currently uses `frames`, not a `duration` field. Its
`ContainerReflowCurve` choices mirror the built-in easing names above.
`ContainerReflowTransitionKind::Named` is reserved and is not implemented
as a named-timeline lookup.

### Try it: Transition Grid

Choose **Remove** to watch a tile leave. Switch from
**Flow** to **Ghost** and remove another: compare when the remaining tiles
begin moving into its space. **Snap** and **Ease** change how those remaining
tiles reach their new positions.

Next, tap a tile to change its accent color and increment its counter, then choose
**Reverse**. The loop uses `key cell.id`, so each tile keeps its identity as
its position changes. A fifth tap replaces that tile with a new one at the
end of the list. **Insert** adds a fresh tile at the beginning.
The tile panel scrolls independently, keeping the policy controls in reach
as the list grows.

The source tabs show the container policies in `lib.pax`, the child's enter
and exit timelines in `cell_button.pax`, and the Rust handlers that update
the collection. **Restart** reloads the whole example; **Open standalone**
gives it a separate page.

<pax-example
  path="transition-grid"
  title="Transition Grid"
  height="820"
  files="src/lib.pax,src/cell_button.pax,src/lib.rs,src/cell_button.rs,src/hint_pill.pax,src/hint_pill.rs">
</pax-example>

Use keys and exercise rapid changes as well as settled states. A ghost may
overlap its moving siblings while exiting, so consider clipping and visual
order in the surrounding composition. Container reflow controls layout
frames; animate a nested surface when you want a separate scale or flourish
without making it responsible for the container's placement.

## Paths, interaction, and motion choices

A playhead can come from more than a clock. A slider, pointer position,
scroll position, or Rust simulation can supply the value that several
tracks sample. Keep the mapping explicit and bounded: translate the input
into the timeline's position range, then let the tracks own the visuals.

The `marionette` example uses shared and per-part playheads;
`pax-logo` exposes progress for coordinated vector motion; and
`mouse-animation` derives a moving shape from a parametric path.
`timeline-playground` explores longer sequences and mixed duration units.
These are deeper source references after the small examples here.

For handwriting or a drawn-line reveal, animate `draw_start` and
`draw_end` on Path or Handwriter. Drawing owns the
[path geometry and reveal limits](drawing-styling.md#paths-and-svg);
[Scrolling](scrolling-viewports.md) owns scroll positions and viewports.
Animation supplies the changing value.

Choose motion that preserves the interface's meaning when paused or
skipped. Keep long decorative sequences user-driven, and provide an
application setting that can select a stable final state or a shorter
transition when needed. Pax does not currently expose a unified
reduced-motion preference in its public platform API; do not assume the
examples automatically adapt to the operating system's preference.

The core timeline and property systems are shared by web, macOS, iOS, and
iPadOS. The animated property still has its own renderer and native-control
limits, and frame scheduling varies by target. Verify the finished
interaction on the targets you ship rather than inferring visual parity
from a successful web run.

## Read more

Continue with [Compositing and Effects](compositing-effects.md) for clipping,
masking, and mixed native/rendered surfaces.
[Layout](layout-responsiveness.md) and the [Stacker API](api/pax-std/layout/stacker.md)
cover ordinary container sizing. [Events](event-handling-rust.md) explains the
handlers and lifecycle that drive state changes, while
[Accessibility and Native Controls](accessibility-native-controls.md) covers
keyboard and control behavior.
