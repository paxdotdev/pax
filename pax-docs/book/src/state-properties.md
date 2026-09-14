<a id="state--properties"></a>

# State and Properties
<!-- summary: Initialize and update reactive Rust state, share property handles, and derive values with computed properties and subscriptions. -->
<!-- tags: state, properties, reactivity, dag -->

A progress value can feed a label, a bar, and a color. In
[PAXEL](data-binding-expressions.md), we described those relationships with
template formulas. Here we turn to the Rust side: where the value lives, how
an action changes it, and how to build relationships between properties in
Rust.

Pax represents reactive state with `Property<T>`, where `T` is the value's
Rust type. A property can hold a value your application writes, or compute a
value from other properties. Both participate in the same reactive graph as
template expressions.

This chapter continues the small Field notes panel from
[Templates](template-language.md). It includes the component declaration and
template needed to try the first example in a project from
[Getting Started](getting-started.md).

<a id="properties-and-computed-values"></a>
## State in a component

The panel has a title and a numeric progress value. Declare them as fields on
the Rust component that owns its template:

```rust
use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Notes {
    pub title: Property<String>,
    pub progress: Property<f64>,
}
```

Rust accesses these values through methods such as `self.progress.get()`.
The template reads the value directly with `{self.progress}`. PAXEL connects
that read to the property so subsequent writes can update the interface.

Use a property for state that other reactive values need to observe. Ordinary
Rust data is useful for calculations and data structures, but changing a
plain field alone does not notify the graph. Fields on a `#[pax]` type also
participate in generated serialization and value-conversion code; they must
meet that generated code's type requirements.

## Initial values

By default, `#[pax]` derives Rust's `Default` trait for the component.
`Property<T>::default()` creates a property containing `T::default()`: the
title starts as an empty string and progress starts at zero.

To give this panel its initial content, add `#[custom(Default)]` alongside
the existing attributes on `Notes`, then supply the implementation:

```rust
impl Default for Notes {
    fn default() -> Self {
        Self {
            title: Property::new("Field notes".to_string()),
            progress: Property::new(0.25),
        }
    }
}
```

`#[custom(Default)]` tells the macro to use your implementation in place of
its generated one. Each call creates properties for that component instance.
`Property::new` also works outside a component when you need a local reactive
value.

The preceding PAXEL example set these same values in `on_mount`. With the
custom default above, remove those initialization writes. Mount remains a
useful place for setup that needs `NodeContext` or the component's established
property connections, as the computed-property example below will show.
See [Events and Rust](event-handling-rust.md) for lifecycle behavior and
[Components and Composition](components-composition.md) for values supplied
by a parent component.

## Reading and updating state

Give the panel a button that advances progress by a quarter, up to one. Add
this handler to `Notes`:

```rust
impl Notes {
    pub fn advance(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let next = (self.progress.get() + 0.25).min(1.0);
        self.progress.set(next);
    }
}
```

In `lib.pax`, bind the button to the handler and read progress in the label
and bar:

```pax
<Group x=24px y=24px width=320px height=220px>
    <Text x=24px y=24px width={100% - 48px} height=36px
        text={self.title}
        style={font_size: 24px, fill: rgb(32, 40, 48)}
    />
    <Button x=24px y=76px width=160px height=28px
        label="Advance" @button_click=self.advance
    />
    <Text x=24px y=116px width={100% - 48px} height=28px
        text={"Progress: " + (self.progress * 100) + "%"}
        style={font_size: 16px, fill: rgb(72, 80, 88)}
    />
    <Group x=24px y=160px width={100% - 48px} height=12px>
        <Rectangle width={(self.progress * 100)%} height=100%
            fill=rgb(55, 120, 90) corner_radius=6
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

The handler reads the current value, calculates the next one, and writes it
with `set`. The label and width then follow the new progress. Event wiring is
covered in [Events and Rust](event-handling-rust.md); the state operation
itself is the same whether an update begins with a button, a data-loading
result, or another application action.

### Collections and value snapshots

`get()` returns a clone of the stored value. For a `Vec<String>`, you can edit
that copy and write it back, or use `update` to perform both steps:

```rust
let entries = Property::new(vec!["Sketch the header".to_string()]);

let mut draft = entries.get();
draft.push("Choose a palette".to_string());
assert_eq!(entries.get().len(), 1);

entries.set(draft);
assert_eq!(entries.get().len(), 2);

entries.update(|items| items.push("Add an interaction".to_string()));
assert_eq!(entries.get().len(), 3);
```

The first `push` changes the local vector. `set` publishes that vector through
the property. `update` is a convenient get–mutate–set operation; it also clones
the value before calling your closure. Use the same pattern for a property
holding a struct whose fields you want to change together.

The access methods have these roles:

| Method | Behavior |
| --- | --- |
| `get()` | Bring a dirty property up to date, then clone and return its value. |
| `read(f)` | Bring it up to date, then lend its value to `f` without cloning it. |
| `set(value)` | Store a value and invalidate downstream dependents. |
| `set_if_neq(value)` | Set only when the value differs from the current one; return whether a write occurred. |
| `update(f)` | Clone the current value, let `f` mutate that copy, then store it and invalidate dependents. |

`set` and `update` invalidate dependents even if the resulting value is equal
to the previous one. Use `set_if_neq` when unchanged writes are common and
equality is inexpensive; it requires `T: PartialEq`. The button above could
use it to avoid another invalidation when progress is already at one.

For an inspection that does not need a copy, `entries.read(|items| items.len())`
returns the count without cloning the vector. Keep a `read` closure focused
on its borrowed value. Reading or writing the same property again inside
that closure can panic, including when another property evaluation leads
back to it.

## Property handles

Cloning a `Property<T>` gives you another handle to the same graph node. A
write through either handle changes the value both observe:

```rust
let progress = Property::new(0.25_f64);
let progress_handle = progress.clone();
let earlier_value = progress.get();

progress_handle.set(0.5);
assert_eq!(progress.get(), 0.5);
assert_eq!(earlier_value, 0.25);
```

This is useful when a closure needs access to a property after the method
that created it returns. Move a cloned handle into the closure; later reads
still obtain the property's current value.

The depth of a value copy follows `T`'s own `Clone` implementation. If a value
contains other property handles or shared pointers, cloning that value can
still share the objects inside it. In particular, cloning an entire component
does not produce independent copies of its property values.

To change an existing component field's value, call `set` or `update`. An
assignment such as `self.progress = Property::new(0.5)` gives the field a new
handle; any templates or closures holding the previous handle keep their
old connection. The next section introduces `replace_with` for changing how
an existing field is computed while keeping its graph identity.

Shared handles are also useful across components. Continue with
[Components and Composition](components-composition.md) for ownership,
parent/child bindings, and scoped state. Property handles belong to the
runtime's thread-local graph; use an appropriate application messaging
boundary when work runs on another thread.

## Computed properties

A computed property derives its value from other properties. A PAXEL formula
already does this for a template. Rust's `Property::computed` is useful when
the derived value should be available to Rust code as well, or when you want
to assemble the dependency relationship in Rust.

For example, the panel can expose its formatted progress label as a property.
Add `pub progress_label: Property<String>` to `Notes` and
`progress_label: Property::default()` to its custom default. Then add this
mount method:

```rust
impl Notes {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let progress = self.progress.clone();
        let dependencies = [progress.untyped()];

        self.progress_label.replace_with(Property::computed(
            move || format!("{:.0}% complete", progress.get() * 100.0),
            &dependencies,
        ));
    }
}
```

Replace the panel's progress text element with:

```pax
<Text x=24px y=116px width={100% - 48px} height=28px
    text={self.progress_label}
    style={font_size: 16px, fill: rgb(72, 80, 88)}
/>
```

The initial label reads “25% complete.” Pressing Advance changes the source
property and the label becomes “50% complete.” The bar still reads progress
through its PAXEL formula.

`Property::computed` takes an evaluator and an explicit dependency list.
`.untyped()` provides a handle suitable for that list, allowing properties
with different value types to be dependencies of one computation. Include
every input whose changes should trigger reevaluation. A `.get()` inside a
Rust evaluator does not automatically register a dependency.

`replace_with` installs the new evaluator and dependencies on the existing
`progress_label` field. The field keeps the outgoing connections already
established by its template and any other consumers. For a new local
property, you can use the handle returned by `Property::computed` directly.

Change derived state by updating its source inputs. Writing to a computed
property with `set` changes its stored value but leaves the evaluator in
place; a later dependency change can compute the value again. Keep the
dependency graph acyclic, and avoid an evaluator that reads the very field
it is replacing, directly or through another computed property.

Computed evaluators should be deterministic and free of side effects. Their
execution follows demand for a value. Use a subscription for work that must
respond to a change independently of a value being read.

<div class="docs-example-placeholder">
<p><strong>Interactive example planned:</strong> advance the Field notes panel and follow the Rust state write into the label and bar.</p>
<!-- Production brief:
- Keep the source property, handler, computed field, and template visible.
- Show the initial value and successive changes without introducing a starter.
- Distinguish a shared property handle from a copied value.
Treatments: the existing panel with coordinated source highlights; a short
checklist whose add action demonstrates collection updates; or two side-by-side
readouts sharing a handle, with a separate frozen snapshot. Prefer the panel.
Keep source and behavior tied to the docs version. -->
</div>

## Subscriptions and effects

A subscription runs a callback in response to reactive dependencies. Use it
for a bounded side effect, such as notifying an application service that a
selection changed. A pure derived value is usually simpler as a computed
property or PAXEL formula.

For a small observable example, log the panel's progress. Rename `_ctx` to
`ctx` in the mount method above and add the following after its computed
property setup:

```rust
let progress = self.progress.clone();
let dependencies = [progress.untyped()];

ctx.subscribe(&dependencies, move || {
    log::info!("Progress observed: {}", progress.get());
});
```

On the web, append `?pax_log=info` to the running app's URL and reload to see
these messages in the browser console. Informational logs are hidden at the
default logging level.

The callback is scheduled once after registration to observe the initial
state. It is then scheduled when a dependency dirties. `subscribe` does not
call it immediately inside the registration call, and several writes before
the reactive work is drained can produce one callback observing the latest
value. A subscription is therefore useful for observing current state; it
does not provide a record of every intermediate write.

`NodeContext` retains subscriptions for its node and clears them when the
node unmounts. `ctx.clear_subscriptions()` removes all subscriptions registered
on that node. `subscribe` returns no individual cancellation handle.

Callbacks run synchronously as Pax drains reactive work during runtime
updates. They may set another property and enqueue further work. Keep them
short, move blocking I/O outside the callback, and avoid effects that
continually dirty one another. For actions that should happen specifically
because a user pressed a button, put the action in the event handler; see
[Events and Rust](event-handling-rust.md).

<a id="the-property-graph"></a>
## How updates travel

The property graph records which values depend on which inputs. The panel
now contains relationships like these:

```text
progress ----> bar width (PAXEL)
    |
    +--------> progress_label (Rust) ----> text (PAXEL)
```

Setting progress marks its ordinary computed dependents *dirty*: their
cached values may be out of date. This invalidation travels through those
dependents before their new values are calculated.

A later `get` or `read`, including a read needed to update the interface,
recomputes the required dirty values. As each evaluator reads its inputs,
those inputs are brought up to date too. Repeated reads of a clean computed
property use its cached value. Multiple writes can coalesce before a read,
and an ordinary computed value with no observers need not run just because
an input changed.

This is eager invalidation with on-demand evaluation. Subscriptions are
explicit observers: the runtime schedules them when they become dirty.
Propagation cutoffs, described next, also schedule work so they can decide
whether a change should travel farther through the graph.

<div class="docs-media-placeholder">
<p><strong>Diagram planned:</strong> trace a progress write through invalidation and then through the reads that update the panel.</p>
<!-- Production brief:
- Distinguish graph edges from the temporal order of writes and evaluations.
- Show one input branching into a template formula and a Rust computed value.
- Show cached reads and multiple writes coalescing before evaluation.
Treatments: an annotated three-step dependency graph; a before/after graph of
replace_with preserving consumers; or a pointer-to-bucket cutoff diagram with
several input positions sharing one accepted output. Prefer the three-step
graph; avoid suggesting that every write triggers a full render. -->
</div>

## Propagation cutoffs

Cutoffs are an advanced tool for a graph whose frequently changing inputs
produce comparatively stable outputs. The ordinary properties above are
enough for the panel; consider a cutoff when measurement points to expensive
downstream work that often produces the same result.

Ordinary invalidation travels through a computed property before Pax knows
whether its output has changed. `Property::computed_with_cutoff` creates a
boundary: when an input changes, Pax evaluates this property before deciding
whether to invalidate its downstream dependents.

For example, a pointer position might select one bucket for every 20 units:

```rust
let pointer_x = Property::new(0.0_f64);
let pointer_x_for_bucket = pointer_x.clone();

let bucket = Property::computed_with_cutoff(
    move || (pointer_x_for_bucket.get() / 20.0).floor() as i64,
    &[pointer_x.untyped()],
    |last_accepted, candidate| last_accepted == candidate,
);
```

Moving from 2 to 3 leaves the bucket at zero. Moving to 21 changes it to one.
The predicate compares the last accepted output with the new candidate:

| Return value | Result |
| --- | --- |
| `true` | Discard the candidate, retain the last accepted value, and stop invalidation at the cutoff. |
| `false` | Accept the candidate and invalidate downstream dependents. |

The first evaluation is always accepted because there is no prior accepted
value to compare. After a candidate is suppressed, the next comparison still
uses the last accepted value. Equality is common; approximate or domain-specific
predicates are also possible, with that same retention behavior.

Cutoffs settle as part of the runtime's synchronous reactive work, before
queued effects. A direct `get` or `read` of a dirty cutoff can settle it
earlier. The cutoff still evaluates its own function; the saved work is the
invalidation and reevaluation beyond it when the output is equivalent.

Use a cutoff where inputs change frequently, the result often remains
equivalent, and downstream work is materially more expensive than evaluating
the cutoff and its predicate. Keep both callbacks inexpensive and free of
side effects. `computed_with_cutoff_and_name` adds a diagnostic name; the
other named property constructors can also help identify a large graph's
values during diagnosis.

<a id="performance-guidelines"></a>
## Choosing a pattern

| Need | Start with |
| --- | --- |
| A value derived for a template | A PAXEL formula; the compiler supplies its dependency edges. |
| Application state changed by an action | A property written by the Rust handler. |
| Derived reactive state used from Rust | `Property::computed`, with every dependency listed explicitly. |
| A short side effect observing state | `NodeContext::subscribe`, with its node-scoped lifetime. |
| Fewer unchanged writes | `set_if_neq`, when equality is inexpensive. |
| A stable output between a busy input and expensive consumers | A measured use of `computed_with_cutoff`. |

For the complete method surface and value-type requirements, see
[the properties API](api/pax-runtime-api/properties.md). Continue with
[Events and Rust](event-handling-rust.md) to connect more kinds of actions to
state, [Components and Composition](components-composition.md) to organize
shared state, or [Animation and Motion](animation-motion.md) to change values
over time.
