# PAXEL and reactive values
<!-- summary: Derive interface values from state with PAXEL formulas, units, helpers, and reactive bindings. -->
<!-- tags: paxel, bindings, expressions, reactivity -->

A label might show a name, a bar might show progress, and a color might signal
that something is ready. PAXEL connects those visible values to the state they
describe. It is Pax's expression language, written inside `{...}` in templates.

Think of each expression as a spreadsheet formula: it describes how to obtain
a value from its inputs. Rust supplies application state and changes it in
response to events; the formulas let the interface follow along.

This chapter builds on the panel from [Templates](template-language.md).
It covers the values a template can derive. [State and Properties](state-properties.md)
explains how to manage those values in Rust, including computed properties and
subscriptions.

<a id="bindings"></a>
## Reactive bindings

Start with a literal, then replace it with a property read:

```pax
<Text text="Field notes" />
<Text text={self.title} />
```

The first element always displays the same words. The second reads `title`
from the component that owns the template. That component exposes the value
through a Rust `Property<T>` field. Here is a complete declaration with a title
and a progress value:

```rust
use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Notes {
    pub title: Property<String>,
    pub progress: Property<f64>,
}

impl Notes {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.title.set("Field notes".to_string());
        self.progress.set(0.25);
    }
}
```

Pax calls `on_mount` when the component mounts. These initial values give the
template something to display. In `lib.pax`, the same progress value can drive
several properties:

```pax
<Group x=24px y=24px width=320px height=220px>
    <Text x=24px y=24px width={100% - 48px} height=36px
        text={self.title}
        style={font_size: 24px, fill: rgb(32, 40, 48)}
    />
    <Slider x=24px y=76px width={100% - 48px} height=24px
        value=bind:progress min=0.0 max=1.0 step=0.25
    />
    <Text x=24px y=116px width={100% - 48px} height=28px
        text={"Progress: " + (self.progress * 100) + "%"}
        style={font_size: 16px, fill: rgb(72, 80, 88)}
    />
    <Group x=24px y=160px width={100% - 48px} height=12px>
        <Rectangle width={(self.progress * 100)%} height=100%
            fill={self.progress >= 0.5 ? rgb(55, 120, 90) : rgb(75, 100, 160)}
            corner_radius=6
        />
        <Rectangle width=100% height=100%
            fill=rgb(220, 216, 206) corner_radius=6
        />
    </Group>
    <Rectangle width=100% height=100%
        fill=rgb(246, 241, 230) corner_radius=12
    />
</Group>
```

Dragging the slider writes to `progress`. The label reads it to form a string;
the foreground rectangle reads it to choose a percentage width and a color.
At `0.25`, the bar fills a quarter of its container. At `0.5`, it fills half
and turns green.

Each formula reading `progress` is a derived value with a dependency on that
property. A write marks those derived values for reevaluation. Pax computes
them when they are next needed and caches their results between changes.
There is no separate redraw call in this example.

### Reading versus writing

An expression such as `text={self.title}` reads state. A supporting control can
also write back through `bind:`:

```pax
<Textbox text=bind:title />
```

Typing updates the same `title` property that the heading reads. `bind:` names
a property to share with the control; it does not accept a formula for a
derived result. Use `{...}` for labels, dimensions, colors, and other calculated
values. See [Accessibility and Native Controls](accessibility-native-controls.md) for control
behavior, and [Events and Rust](event-handling-rust.md) for updates that need
application logic.

In this panel, `self.title` and `title` read the same property. This chapter
uses the explicit `self` form when referring to component state.

## Literals

The receiving property supplies the expected type. A `Text` needs a string for
`text`; a `Rectangle` needs a fill for `fill` and a size for `width`. Pax converts
compatible values to that type. A value with the wrong shape or type is an
error.

| Kind | Examples |
| --- | --- |
| Number | `24`, `0.25` |
| Boolean | `true`, `false` |
| String | `"Field notes"` |
| Size | `24px`, `50%` |
| Rotation | `15deg`, `0.5rad` |
| Time | `250ms`, `1s`, `5f` |
| Color | `BLUE`, `rgb(32, 40, 48)`, `rgba(32, 40, 48, 128)` |
| List | `[120px, None, 30%]` |
| Contextual object | `{font_size: 16px, fill: BLUE}` |
| Empty option | `None` |

Some literals have a property-specific meaning. For example, the entries in
a Stacker's `sizes` list describe its cells, while entries in a rectangle's
`corner_radius` list describe its corners. The receiving type defines those
positions; see the [shape reference](#shape-value-reference) below.

### Units in formulas

Units remain useful inside expressions:

```pax
<Rectangle width={100% - 48px} />
<Rectangle width={(self.progress * 100)%} />
```

The first width leaves 48 pixels out of the available parent width. The second
turns a numeric fraction into a percentage. Layout determines what a dimension
is relative to; [Layout and Responsiveness](layout-responsiveness.md) develops
that relationship in more detail.

Write a unit immediately after its number or parenthesized expression:
`24px` and `(self.progress * 100)%`. A space between the value and the unit
breaks that form. `%` means percent; the modulo operator is `%%`.

Time suffixes follow the same pattern: `(100 + self.delay)ms` produces a
duration if `delay` is numeric. [Animation and Motion](animation-motion.md)
covers durations, frame counts, and clocks.

## Choosing values

A ternary selects a value using a boolean condition:

```pax
<Text text={self.progress >= 1.0 ? "Ready" : "In progress"} />
```

The form is `condition ? when_true : when_false`. Pax evaluates the chosen
branch. Both alternatives should produce values compatible with the receiving
property. A ternary changes a value on an existing element; use
[conditional content](components-composition.md#conditional-content) when the condition should determine
which elements exist.

For optional data, `??` supplies a fallback. If the component declares
`maybe_title: Property<Option<String>>`, its template can use:

```pax
<Text text={self.maybe_title ?? "Untitled"} />
```

An option contains either `Some(value)` or `None`. `??` unwraps a present
value, and evaluates its right side only when the left side is `None`. A
non-option value passes through unchanged: `false`, zero, and an empty string
all remain valid values. The fallback does not catch an error in the expression
on its left.

When assigning to an `Option<T>` property, an ordinary compatible value can be
lifted into the option automatically. Write `None` for the empty case; there is
no need to wrap a present template literal in `Some(...)`.

## Structured values

Use `.` to read a field and square brackets to read a zero-based list entry.
For example, if `items` is a nonempty `Property<Vec<String>>`, `self.items[0]`
reads its first string. The index must be within the list's bounds. Update
application data through the owning Rust property so its dependent values can
be invalidated.

A list can itself be derived. Given `first_size: Property<Size>`, this Stacker
uses it for the first cell and leaves the second flexible:

```pax
<Stacker sizes={[self.first_size, None]}>
    <Text text="First cell" />
    <Text text="Flexible cell" />
</Stacker>
```

The outer braces make the whole list an expression. Use this form when list
entries read state; a literal list such as `[120px, None]` is convenient for
fixed values.

Contextual objects can hold individual bindings. With
`heading_size: Property<Size>`, a text style can keep its color fixed while
reading the font size:

```pax
<Text text="Field notes" style={
    font_size: {self.heading_size}
    fill: rgb(32, 40, 48)
} />
```

Here the outer braces describe the style's fields, and the braces around
`self.heading_size` mark that field's expression. An expression can also
produce a whole object; the receiving property still determines which fields
and types are meaningful.

### Loops

A range such as `0..3` includes 0, 1, and 2. Its upper bound is exclusive.
Ranges often provide the source for a template `for`:

```pax
for i in 0..3 {
    <Text y={(i * 24)px} text={"Item " + i} />
}
```

The template repeats the element; PAXEL supplies its values. Continue with
[Data-driven components](components-composition.md#data-driven-components) for iteration over data, keys, and item identity.

## Function calls

Built-in functions can express a calculation more clearly than a longer
formula. `Math::min` and `Math::max` choose between two values; `Math::len`
returns a list's length. For example, a numeric `desired_width` can become a
width with an 80-pixel minimum:

```pax
<Rectangle width={(Math::max(80, self.desired_width))px} />
```

Function calls use a registered `Type::function(...)` name. Enum constructors
also use path syntax, such as `StackerDirection::Horizontal`. PAXEL's call
syntax reaches the functions and constructors exposed to it; arbitrary Rust
methods are not automatically available.

### A small Rust helper

To give the panel a reusable progress label, add `#[has_helpers]` alongside the
attributes on the existing `Notes` declaration. Then add a separate helper
implementation:

```rust
#[helpers]
impl Notes {
    pub fn progress_label(progress: f64) -> String {
        format!("{:.0}% complete", progress * 100.0)
    }
}
```

Call it with the changing value as an explicit argument:

```pax
<Text text={Notes::progress_label(self.progress)} />
```

`#[helpers]` exposes public associated functions in that implementation; they
have no `self` receiver. `#[has_helpers]` tells the Pax type declaration to use
that helper implementation. Keep lifecycle methods such as `on_mount` in the
ordinary `impl` block shown earlier.

The formula can track `self.progress` because it appears in the arguments.
Hidden reads of external state inside a helper cannot supply that connection.
Helpers should return a value without changing application state, reading
files, or performing other side effects. Rust event handlers and subscriptions
provide those capabilities. See [State and Properties](state-properties.md)
when a derived value needs to be shared beyond one template.

## Built-in globals

Built-in values use a `$` prefix. They describe the running app and its
environment:

| Value | What it provides |
| --- | --- |
| `$viewport` | Scene width and height in logical pixels; `major`, `minor`, `aspect`, `landscape`, `portrait`, and `square`. |
| `$target` | Boolean platform and operating-system facts, such as `web`, `native`, `macos`, `ios`, `iphone`, and `ipad`. |
| `$frames`, `$millis` | Runtime frame count and elapsed time in milliseconds. |
| `$gyro`, `$accel` | Device orientation and acceleration readings, where supplied by the target and permitted by the user. |

Viewport orientation and shape fields also have short aliases, including
`$landscape`, `$portrait`, `$square`, `$major`, `$minor`, and `$aspect`:

```pax
<Group width={$landscape ? 50% : 100%} />
<Rectangle width={($viewport.width / 2)px} />
```

The viewport dimensions are numbers. Attach a unit when using them to produce
a size. A parent-relative percentage is often more useful for nested content.

Each `$target` field likewise has an alias: `$web` means `$target.web`, for
example. Platform and operating-system facts can overlap: an app in a browser
on macOS can have both `$web` and `$macos` set. The remaining fields are
`android`, `windows`, `linux`, `mobile`, and `desktop`. These describe runtime
facts, including browser hosts; they do not extend the supported build targets
beyond web, macOS, iOS, and iPadOS.

The sensor values expose `x`, `y`, and `z`. Orientation is in degrees and
acceleration is in meters per second squared; web acceleration includes gravity
when that reading is available. Mobile WebKit requires a user gesture before
sensors can stream. A web app can request access by invoking
`window.paxRequestDeviceSensorPermissions()` from an app-owned JavaScript
button handler. This is a web integration call, separate from PAXEL. See
[Accessibility and Native Controls](accessibility-native-controls.md) for input behavior and
[Animation and Motion](animation-motion.md) for using clocks.

## `$base`

`$base` refers to the earlier value of the same property while Pax resolves
its settings. It lets a later assignment build on what an earlier one supplied:

```pax
<Rectangle class="card" width={$base + 24px} height=80px />

@settings {
    .card {
        width: 240px
    }
}
```

The class supplies a width of 240 pixels. The inline expression adds 24, so
this rectangle is 264 pixels wide. If the class width changes, the inline
formula continues to build on it.

`$base` is contextual to the property being resolved. It does not read a parent
element's property or the previous animation frame. See
[Templates](template-language.md#imported-settings) for ordinary settings
layers and [Animation and Motion](animation-motion.md) for timeline-relative
values.

<a id="common-gotchas"></a>
## Operators

This is a syntax lookup, grouped by purpose. Parentheses make the intended
grouping explicit in formulas that mix different operations.

| Purpose | Operators | Example |
| --- | --- | --- |
| Fallback | `??` | `maybe_title ?? "Untitled"` |
| Choice | `? :` | `selected ? BLUE : GRAY` |
| Boolean | `!`, `&&`, `\|\|` | `enabled && !disabled` |
| Equality | `==`, `!=` | `mode != "hidden"` |
| Comparison | `<`, `<=`, `>`, `>=` | `progress >= 0.5` |
| Arithmetic | `+`, `-`, `*`, `/`, `%%`, `^` | `(index + 1) * 24` |
| Range | `..` | `0..count` |
| Access | `.`, `[...]` | `user.name`, `items[0]` |
| Grouping and units | `(...)` with optional suffix | `(width + 12)px` |

`+` also joins strings and displayable values, as in the panel's progress
label. `^` is exponentiation. The unit suffixes are `px`, `%`, `deg`, `rad`,
`ms`, `s`, and `f`.

## Shape value reference

The following forms are useful when assigning structured values to drawing
properties. Their syntax and visual use now live together in
[Drawing and Styling](drawing-styling.md).

### Corner radii

See [Corner radii](drawing-styling.md#corner-radii) for the one-to-four-value
list, its per-corner mapping, and the named-object longhand.

### Gradients

See [Gradients](drawing-styling.md#gradients) for `@gradient` stops, `[x, y]`
points, alpha, linear defaults, and current radial/backend limits.

### Timeline-relative values

See [Relative values and property ownership](animation-motion.md#relative-values-and-property-ownership)
for using `$base` in timeline keyframes, setting a base layout value, and
understanding when an inline assignment overrides an ordinary timeline track.

## Read more

[State and Properties](state-properties.md) is the next step for the Rust side
of reactivity. [Events and Rust](event-handling-rust.md) connects user actions
to state changes. For a larger template, continue with
[Components and Composition](components-composition.md), including conditional
content, lists, and keyed identity.
