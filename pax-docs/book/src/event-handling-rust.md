<a id="event-handling--rust-logic"></a>

# Events and Rust
<!-- summary: Connect interactions to Rust handlers, work with event data and lifecycle, and understand event delivery and local coordinates. -->
<!-- tags: events, rust, lifecycle, input -->

A template describes how an interface responds to its state. An event handler
gives the user a way to change that state: advance a task, edit a title, choose
a destination, or move something across the screen.

This chapter continues the Field notes panel from
[Properties](state-properties.md#reading-and-updating-state). We will look
closely at its button handler, add a title editor and a keyboard shortcut,
then explore lifecycle and custom interactions. The examples use the same
`Notes` component and its `title` and `progress` properties; the complete
starting declaration and template are in Properties. Keep `use pax_kit::*;`
in the Rust file.

Space Game combines keyboard input with a running simulation. Use `w`, `a`,
`s`, and `d` to move and Space to fire. In an embed, first click the game area
so keyboard input reaches its document. After game-over, choose **Play Again**
below the score to start a new round.
Its source combines key state with
tick-driven updates; the frame-based movement is specific to this example.

<pax-example
  path="space-game"
  title="Space Game"
  height="560"
  files="src/lib.pax,src/lib.rs,src/animation.rs">
</pax-example>

## Connect an action to Rust

The panel's button names the method to call when it is activated:

```pax
<Button x=24px y=76px width=160px height=28px
    label="Advance" @button_click=self.advance
/>
```

The corresponding method lives in `impl Notes`:

```rust
impl Notes {
    pub fn advance(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let next = (self.progress.get() + 0.25).min(1.0);
        self.progress.set(next);
    }
}
```

The binding `@button_click=self.advance` connects the Button's activation
event to a public Rust method. The method receives three things:

- `&mut self` gives it mutable access to the `Notes` instance that owns this
  template.
- `&NodeContext` describes the node handling the event and provides runtime
  information and operations, such as its bounds and navigation.
- `Event<ButtonClick>` carries the event. This particular payload has no
  fields; the activation itself is enough to advance progress.

The leading underscores in `_ctx` and `_event` are Rust's convention for
arguments that this method does not use. Other handlers can name them `ctx`
and `event` and read their contents.

The handler calculates a new value and writes it to the property. The label
and progress bar already read that property, so their bindings pick up the
change. See [Properties](state-properties.md) for the state operations and
[PAXEL](data-binding-expressions.md) for the formulas that consume them.

### Buttons and custom activation

Use `@button_click` with the native `Button` control. For custom content, such
as a card or drawing, `@click` and `@tap` provide general activation handlers.
Both receive `Event<Click>`, which includes position, button, and modifier
information through `event.mouse`.

Either binding on its own receives mouse clicks and single-touch taps. If
both are present on the same node, mouse input selects `@click` and touch
input selects `@tap`. This lets a custom surface share an action across input
types, while still allowing separate actions where needed.

A touch activation occurs at release, after `TouchEnd`, when the gesture
qualifies as a tap. For immediate pressed feedback or dragging, use the
lower-level mouse and touch events described under
[Events and coordinates](#events-and-coordinates).

## Use event data

An event's type tells you what data the handler receives. These are common
starting points:

| Binding | Handler argument | Useful data |
| --- | --- | --- |
| `@button_click` | `Event<ButtonClick>` | Activation of a native Button |
| `@click`, `@tap` | `Event<Click>` | `event.mouse.x`, `.y`, `.button`, `.modifiers` |
| `@mouse_move` | `Event<MouseMove>` | Pointer position through `event.mouse` |
| `@touch_start`, `@touch_move`, `@touch_end`, `@touch_cancel` | `Event<TouchStart>`, `Event<TouchMove>`, `Event<TouchEnd>`, `Event<TouchCancel>` | `event.touches`, including identifiers and positions |
| `@key_down`, `@key_up` | `Event<KeyDown>`, `Event<KeyUp>` | `event.keyboard.key`, `.modifiers`, `.is_repeat` |
| `@textbox_input`, `@textbox_change` | `Event<TextboxInput>`, `Event<TextboxChange>` | `event.text` |
| `@checkbox_change` | `Event<CheckboxChange>` | `event.checked` |
| `@slider_change` | `Event<SliderChange>` | `event.value` |

`Event<T>` exposes its payload fields directly, so `event.text` is a
convenient way to read `event.args.text`. The
[event API reference](api/pax-runtime-api/events.md) lists the complete
payloads, including wheel, drop, and other input events. Sensor availability
and native control behavior also depend on the target.

To edit the panel's title, add this Textbox after the panel's outer `Group`
in `lib.pax`:

```pax
<Textbox x=24px y=260px width=320px height=32px
    text={self.title} @textbox_input=self.rename
/>
```

Add its handler to the Rust file:

```rust
impl Notes {
    pub fn rename(&mut self, _ctx: &NodeContext, event: Event<TextboxInput>) {
        self.title.set(event.text.clone());
    }
}
```

As the user types, the handler copies the event's text into `title`, and the
panel's heading follows. The template supplies the current title to the
Textbox; the handler makes the application's response to edits explicit.

On web, `@textbox_input` follows the browser's `input` event, while
`@textbox_change` follows its `change` event, used for committed edits. Choose
input for live feedback; choose change when the action should wait for a
commit. Exact commit timing belongs to the platform control, so test it on
the targets you ship. Forms, two-way bindings, and focus are covered in
[Native Controls](accessibility-native-controls.md).

## Choose the binding's scope

An inline binding belongs to the element where it is written. In the Advance
example, the Button is the receiving node, and `self` is the containing
`Notes` component. Consequently, `ctx.bounds_self` describes the Button's
bounds, even though `self.progress` is a field on Notes.

A binding in the component's `@settings` block runs at component scope. For
example, to handle button activations that reach Notes, move the binding off
the Button and into the template's settings:

```pax
@settings {
    @button_click: advance
}
```

Here both `self` and the receiving context refer to Notes. Activations from
its buttons can reach this handler through event propagation. If you try this
alternative, remove `@button_click=self.advance` from the Button: keeping both
bindings calls `advance` at both places and increments twice.

The same distinction applies when you use a child component. A handler in
the child's own settings acts on that child; an inline binding on the child's
invocation acts on the containing component. See
[Components and Composition](components-composition.md) for the component
boundary and [Event delivery](#event-delivery) for propagation.

## Do application work

Handlers are ordinary Rust methods. They can validate input, call helpers,
update several properties, and invoke application services. Keep reusable
work in a helper when more than one interaction needs it.

For example, let the right arrow key advance the panel as well as the button.
Replace the original `impl Notes` containing `advance` with:

```rust
impl Notes {
    fn advance_progress(&mut self) {
        let next = (self.progress.get() + 0.25).min(1.0);
        self.progress.set(next);
    }

    pub fn advance(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.advance_progress();
    }

    pub fn key_down(&mut self, _ctx: &NodeContext, event: Event<KeyDown>) {
        if event.keyboard.key == "ArrowRight" && !event.keyboard.is_repeat {
            self.advance_progress();
            event.prevent_default();
        }
    }
}
```

Keep `rename` and any other methods in their existing `impl` blocks. Add the
keyboard binding to the template's `@settings` block, or create that block if
there is none:

```pax
@settings {
    @key_down: key_down
}
```

The `is_repeat` check limits this action to the initial key press. On web,
try it while focus is outside the Textbox and other native controls. The
keyboard dispatch and default-prevention boundaries are explained below.

Pax calls event handlers synchronously. Keep their work short enough for
the interface to remain responsive. For network requests or lengthy work,
use an integration appropriate to your target, then publish the result into
application state on the runtime's thread. `Property` handles belong to a
thread-local reactive graph; they are not a cross-thread messaging API.
Declaring a handler `async fn` does not give Pax an executor to run it.

A loading interface can expose a loading flag, a result, and an error as
properties. Its template can then describe those states while the application
layer manages the request. A complete asynchronous integration is beyond this
chapter. For other common actions, read [Routing](routing.md) for navigation
and [Motion](animation-motion.md) for easing and timeline control.

## Work with component lifecycle

Some work follows the lifetime of a component rather than an input event.
Pax recognizes public lifecycle methods on its Rust implementation:

| Method | When it runs | Typical use |
| --- | --- | --- |
| `on_mount` | When the component mounts | Install computations or subscriptions that need established bindings or context |
| `on_tick` | On runtime ticks while the component is active | Advance an application simulation or other time-driven logic |
| `on_pre_render` | After tick handlers, in the pre-render phase | Synchronize work that must follow that tick's updates |
| `on_unmount` | When the component is removed from the mounted tree | Release application-owned registrations or finish mounted work |

Lifecycle methods take `&mut self` and `&NodeContext`, without an `Event`
argument. For a small way to observe the panel's lifetime, add:

```rust
impl Notes {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        log::info!("Notes mounted");
    }

    pub fn on_unmount(&mut self, _ctx: &NodeContext) {
        log::info!("Notes unmounted");
    }
}
```

On web, open the app with `?pax_log=info` to see these messages in the browser
console. The unmount message appears when Pax removes the component from the
tree, for example through a template conditional. It is not a guarantee of
cleanup on browser or process termination.

If Notes already has an `on_mount`, add the new work to that method. The
[computed-property example](state-properties.md#computed-properties) uses
mount to establish a derived label. Ordinary initial values can live in
`Default`; mount is useful for setup that needs the mounted context or
property connections. Do not assume that all children have finished mounting
or measuring when the parent's mount handler runs.

The short names `mount`, `tick`, `pre_render`, and `unmount` are also recognized.
When both forms exist, the `on_*` name is selected. To use a different method
name, bind it explicitly, for example `@mount: initialize` in `@settings`,
with a public `initialize(&mut self, ctx: &NodeContext)` method. An explicit
binding replaces automatic selection for that lifecycle event.

`on_tick` is driven by the runtime, with no fixed frame-rate guarantee. Use
elapsed time for behavior that should progress at a stable real-time rate;
`ctx.elapsed_frames` and `ctx.elapsed_millis` expose the runtime's clocks.
For ordinary visual transitions, start with the facilities in
[Motion](animation-motion.md).

Subscriptions registered through `NodeContext::subscribe` are automatically
cleared when their node unmounts. Other resources owned by your application
still need the cleanup appropriate to that integration. See
[Subscriptions and effects](state-properties.md#subscriptions-and-effects)
for callback scheduling and lifetime details.

## Dispatch custom events

A reusable control can report an application-level action such as “advance,”
“save,” or “dismiss.” Its caller chooses how to respond. Use
`NodeContext::dispatch_event` to connect that named action to a handler on the
caller.

Consider an Advance control whose internal Button should ask Notes to update
progress. Add this declaration to `lib.rs`, keeping the existing
`use pax_kit::*;` import:

```rust
#[pax]
#[inlined(
    <Button width=100% height=100% label="Advance"
        @button_click=self.activate
    />
)]
pub struct AdvanceButton {}

impl AdvanceButton {
    pub fn activate(&mut self, ctx: &NodeContext, _event: Event<ButtonClick>) {
        ctx.dispatch_event("advance")
            .expect("AdvanceButton requires an @advance binding");
    }
}
```

`#[inlined(...)]` associates this short template with `AdvanceButton`. The
native Button calls `activate` on that component. Its handler then dispatches
the custom name `"advance"`. The name is a string literal; the method accepts
an `&'static str`. Choose a name for the action your component offers, keeping
it distinct from built-in input event names.

Replace the panel's original Button in `lib.pax` with:

```pax
<AdvanceButton x=24px y=76px width=160px height=28px
    @advance=self.advance_from_control
/>
```

Add the receiver to Notes:

```rust
impl Notes {
    pub fn advance_from_control(&mut self, _ctx: &NodeContext) {
        self.progress.set((self.progress.get() + 0.25).min(1.0));
    }
}
```

The `@advance` binding on the component invocation registers that receiver.
Pressing the Button calls `AdvanceButton::activate`, which queues `advance`;
Pax then calls `Notes::advance_from_control`. The two handlers belong to
different component instances. The control needs no access to Notes' fields,
and the caller can reuse the control with another response elsewhere.

### Context and receiver

The context passed to `dispatch_event` identifies the emitting component.
Here `ctx` is the internal Button's context, whose containing component is
`AdvanceButton`. Pax looks for `advance` on that component's invocation. In
the receiver, `self` is Notes while the context describes the AdvanceButton
invocation, following the inline-binding scope described above.

This mechanism connects a component to its caller. It does not search the
tree for a matching name or broadcast the action to every ancestor. Use the
context supplied to the emitting handler and connect the action explicitly
at the call site. A root component has no outer caller to receive such an
action.

Bind custom event names inline, as `@advance=self.advance_from_control`.
Custom names are not currently supported in an `@settings` handler
declaration. That restriction is separate from the built-in bindings, such
as `@button_click`, that settings blocks do support.

### Delivery and data

`dispatch_event` returns `Result<(), String>`. It checks that the named
receiver is registered before adding the event to the runtime's queue.
`Ok(())` means the event was queued; the receiving handler has not run when
the call returns. Delivery happens in the custom-event phase at the end of
a runtime tick. Avoid reading state immediately after dispatch under the
assumption that the receiver has already updated it.

The example uses `expect` because the control requires an `@advance` binding.
If the action is optional, handle the returned error according to that
component's contract. A missing receiver does not create a new event binding.

Custom callbacks have two arguments, `&mut self` and `&NodeContext`, with no
`Event<T>` payload. This API sends a name only: it has no argument for custom
data, no return value from the receiver, and no input event to cancel. Use
properties or shared state for data the receiver needs, and remember that a
queued receiver observes that state at delivery time. If several queued
actions each need distinct data, the application must preserve those values
explicitly rather than overwrite a single shared field.

The original Button event still follows its ordinary delivery rules.
Dispatching `advance` neither cancels that input nor stops its propagation.
If the same application action is also bound to a bubbling `@button_click`,
both paths can run it. Use the custom action as the control's outward
interface and keep the native activation handler inside the control.

For split-file components, value inputs, `bind:`, and scoped stores, continue
with [Components and Composition](components-composition.md#component-actions).

## Event delivery

The remaining sections are useful when interactions span several elements,
share shortcuts, or depend on pointer geometry.

### Targeting and propagation

Pointer input begins with hit testing: Pax finds the topmost eligible node
under the pointer. Remember that earlier siblings in a Pax template are
drawn above later siblings; see
[Templates](template-language.md#element-order). Native controls
can also send events directly to their corresponding node, as a Button does
for `@button_click`.

Common pointer and control events then travel from the receiving node through
its template parents. This is event bubbling. Each visited node runs its
applicable handlers with its own context, which is why an inline handler and
a component-level handler can both observe one action.

The route follows template ancestry. Projection and component composition
can make that ancestry different from the visual container arrangement. Do
not infer the entire event path from which rectangles overlap on screen.

Not every event family bubbles. For example, `@mouse_over` and `@mouse_out`
dispatch locally, while keyboard events use the global delivery described
below. The click/tap choice described earlier is made at each node visited
during activation propagation.

### Default actions

`event.prevent_default()` requests that the chassis suppress the platform's
default response. The arrow-key handler uses it to request that the browser
not perform its normal arrow-key action for that handled press.

Default prevention leaves Pax's handler delivery intact: other handlers and
template parents still receive the event. The current `Event` API has no
`stop_propagation` method.

Whether the request can suppress an action depends on the chassis and event.
On web, it also depends on the browser listener's cancellation rules; the
current touch-start, touch-end, and touch-cancel listeners are passive. Do not
rely on default prevention as a portable way to take ownership of a gesture
or prevent a native control from changing its value.

### Keyboard delivery

Keyboard events currently dispatch across the mounted Pax tree, so multiple
components with keyboard handlers may receive the same key. If a shortcut
belongs to one active view or mode, check that application state in the
handler before acting on it.

On web, the chassis forwards these global keyboard events only while the
document body is the active element. When a Textbox or another native DOM
element has focus, its keyboard input stays with that control. A component's
`@key_down` binding does not by itself give that component keyboard focus.
Read [Native Controls](accessibility-native-controls.md) for the control-specific
focus and accessibility model.

## Events and coordinates

Mouse and touch positions use Pax's window coordinate space. An interaction
inside a translated, scaled, rotated, or scrolling node needs local
coordinates. Use `ctx.local_point` to convert through the node handling the
event.

For example, add a pad below the title editor:

```pax
<Rectangle x=24px y=320px width=320px height=100px
    fill=rgb(220, 216, 206) corner_radius=12
    @mouse_move=self.track_mouse
/>
```

Import `Point2` alongside the existing `pax_kit` import and add this handler:

```rust
use pax_kit::math::Point2;

impl Notes {
    pub fn track_mouse(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        let local = ctx.local_point(Point2::new(event.mouse.x, event.mouse.y));
        self.progress.set(local.x.clamp(0.0, 1.0));
    }
}
```

Moving across the pad maps its left edge to zero progress and its right edge
to one. `local_point` returns normalized coordinates: `(0, 0)` is the node's
local origin and `(1, 1)` is its opposite corner. To work in local pixel-like
units, read `(width, height)` from `ctx.bounds_self.get()` and multiply
`local.x * width` and `local.y * height`. These are layout units, not a promise
of physical display pixels.

The conversion includes the node's transform and ancestor Scroller
presentation offsets. Convert inside the handler using its current context;
subtracting only the element's `x` and `y` misses rotation, scale, and moving
scroll content. Read more about [Layout](layout-responsiveness.md) and
[Scrolling](scrolling-viewports.md).

### Touch sequences and capture

For a custom touch interaction, use `@touch_start`, `@touch_move`,
`@touch_end`, and `@touch_cancel`. Each payload contains touches with a
position, movement deltas, and an identifier. Retain the identifier your
interaction accepts so subsequent events can be matched to it.

The primary touch captures the topmost hit node at `TouchStart`. While that
capture remains valid, subsequent move, end, and cancel events target it even
when the finger leaves its bounds; the usual template-parent delivery still
applies. This is useful for keeping a drag attached to its original control.
It does not establish equivalent mouse capture or a general multi-touch
gesture recognizer.

Clear pressed or dragging state in both end and cancel handlers. `TouchEnd`
finishes a normally released sequence. `TouchCancel` handles an interrupted
sequence, such as one aborted by the platform. A cancelled sequence does not
synthesize a click or tap. Use the activation event for the final action and
the lower-level events for transient feedback when those responsibilities
need to stay separate.

### Touches inside a Scroller

On iOS and iPadOS, the native Scroller's touch observation lets custom child
content receive the start immediately and continue receiving moves while
scrolling. Normal release produces `TouchEnd`; recognition of a scroll
disqualifies tap activation without requiring cancellation of that lower-level
stream. This allows a child to light up on contact, follow the touch, and
clear its feedback when the finger lifts.

Because local conversion includes the moving content transform, a contact
that moves with the scroll stays approximately fixed on its original child
in that direction. Native controls and platform gestures can impose further
input behavior, so test the particular control and target together. On web,
tap qualification also uses a movement tolerance; a moved touch sequence need
not produce activation on release.

## Read further


Continue with [Components and Composition](components-composition.md) for
reusable component interfaces, shared state, and custom event contracts.
