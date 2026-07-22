# State & Properties
<!-- summary: Reactive component state, computed properties, and the property graph. -->
<!-- tags: state, properties, reactivity, dag -->

Pax stores component state in `Property<T>`. A property is a typed handle to a
node in Pax's reactive property graph. Template expressions, component fields,
computed Rust values, animations, and runtime effects all use this graph.

Use properties for values that can change while a component is mounted. Use
ordinary Rust fields for immutable configuration or implementation state that
does not participate in rendering or reactive computation.

## Property Handles

A component exposes reactive state by declaring `Property<T>` fields:

```rust
use pax_kit::*;

#[pax]
#[main]
#[file("counter.pax")]
pub struct Counter {
    pub count: Property<i64>,
}

impl Counter {
    pub fn increment(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.count.set(self.count.get() + 1);
    }
}
```

The corresponding template can read `count` directly. Pax tracks the template
expressions that depend on it and updates them after `count` is set.

```pax
<Text text={"Count: " + count} />
<Button label="Increment" @button_click=self.increment />
```

`Property::new` creates a standalone literal property:

```rust
let count = Property::new(0_i64);
```

Cloning a property clones its handle, not its current value. All clones refer to
the same graph node:

```rust
let count = Property::new(0_i64);
let count_handle = count.clone();

count_handle.set(3);
assert_eq!(count.get(), 3);
```

The main access methods are:

| Method | Behavior |
| --- | --- |
| `get()` | Updates the property if it is dirty, then clones and returns its value. |
| `read(f)` | Updates the property, then lends its value to `f` without cloning it. |
| `set(value)` | Stores a value and invalidates its downstream dependents. |
| `set_if_neq(value)` | Sets only when `value` differs from the stored value. |
| `update(f)` | Mutates a cloned value, stores it again, and invalidates dependents. |

`set` and `update` invalidate dependents even when the resulting value is equal
to the previous value. When unchanged writes are common and `T: PartialEq`, use
`set_if_neq` to avoid unnecessary graph work.

## Computed Properties

`Property::computed` creates a property whose value is derived from other
properties. The evaluator defines the computation, and the dependency list
defines the graph edges that invalidate it.

```rust
pub struct Dimensions {
    pub width: Property<f64>,
    pub height: Property<f64>,
    pub area: Property<f64>,
}

impl Dimensions {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let width = self.width.clone();
        let height = self.height.clone();
        let dependencies = [width.untyped(), height.untyped()];

        self.area.replace_with(Property::computed(
            move || width.get() * height.get(),
            &dependencies,
        ));
    }
}
```

The dependency list is explicit. Every property read by the evaluator that
should cause reevaluation must appear in that list. Reading a property inside
the evaluator does not add a dependency automatically.

Component fields already have a graph identity by the time `on_mount` runs.
`replace_with` replaces a field's literal value with a computed evaluator while
preserving the downstream connections already made by templates and other
properties. Use `Property::computed` directly when creating a new local handle;
use `replace_with` when converting an existing component field.

Computed evaluators should be deterministic and free of side effects. Use a
subscription when a change needs to perform external work.

## The Property Graph

The property graph is a directed acyclic graph. An edge points from a property
to a computed property that depends on it. For example:

```text
width ----\
           area ----> label
height ---/
```

Setting `width` marks `area` and `label` dirty. It does not immediately run both
evaluators. A later `get` or `read` walks the dirty chain in dependency order,
recomputes the values that are needed, and caches their results. Repeated reads
of a clean computed property return the cached value.

This combination of eager invalidation and on-demand evaluation has two useful
properties:

- Multiple writes can coalesce before a computed value is read.
- Computations that are no longer observed do not need to run merely because an
  input changed.

Template expressions compile into the same graph. An expression such as
`{width * height}` behaves like a computed property whose dependencies were
derived by the compiler. Rust code uses `Property::computed` when it needs to
construct the equivalent relationship manually.

Keep the graph acyclic. In particular, `replace_with` can create a cycle if the
replacement evaluator reads the property being replaced, directly or through
another computed property.

## Propagation Cutoffs

Ordinary invalidation continues through a computed property before Pax knows
whether reevaluating it will produce a different value. This is appropriate for
most properties, but it can do unnecessary work when a frequently changing
input maps to a comparatively stable output.

`Property::computed_with_cutoff` creates a computed property that acts as an
invalidation boundary. When one of its inputs becomes dirty, Pax evaluates the
cutoff property before invalidating its downstream dependents. A predicate
compares the last accepted value with the new candidate value:

```rust
let pointer_x = Property::new(0.0_f64);
let pointer_x_for_bucket = pointer_x.clone();

let bucket = Property::computed_with_cutoff(
    move || (pointer_x_for_bucket.get() / 20.0).floor() as i64,
    &[pointer_x.untyped()],
    |last_accepted, candidate| last_accepted == candidate,
);
```

The predicate's return value controls propagation:

| Return value | Result |
| --- | --- |
| `true` | Discard the candidate, retain the last accepted value, and stop invalidation at the cutoff. |
| `false` | Accept the candidate and invalidate downstream dependents. |

The first evaluation is always accepted because there is no prior accepted
value to compare. If a later candidate is suppressed, the stored value is not
updated; the next predicate call still compares against the last accepted
value.

Cutoffs are settled as part of the runtime's synchronous reactive work. A
direct `get` or `read` of a dirty cutoff can settle it earlier. The cutoff does
not avoid evaluating its own function; it avoids invalidating and reevaluating
the graph beyond that function when its semantic output has not changed.

Use a cutoff at a boundary where all of the following are true:

- Upstream inputs change frequently.
- The computed result often remains semantically equivalent.
- Downstream invalidation is materially more expensive than evaluating the
  cutoff and its predicate.

Equality is the common predicate, but approximate or domain-specific predicates
are also possible. Such predicates deliberately retain the last accepted value,
so choose their tolerance with that behavior in mind. Keep both the evaluator
and predicate inexpensive and free of side effects.

`computed_with_cutoff_and_name` provides the same behavior with a diagnostic
name. The named variants are useful when profiling or diagnosing a large graph.

## Subscriptions and Effects

Use `NodeContext::subscribe` when a property change must perform a side effect
rather than derive another reactive value. A subscription declares its
dependencies in the same form as a computed property:

```rust
pub fn on_mount(&mut self, ctx: &NodeContext) {
    let selection = self.selection.clone();
    let selection_for_effect = selection.clone();

    ctx.subscribe(&[selection.untyped()], move || {
        log::info!("selection changed: {:?}", selection_for_effect.get());
    });
}
```

The node retains subscriptions registered through its context. They are
discarded with the node, or can be removed explicitly with
`clear_subscriptions`.

Subscriptions are drained synchronously during runtime updates. A subscription
may set another property, which can enqueue further reactive work in the same
drain. Keep callbacks short, and avoid cycles in which effects repeatedly dirty
one another. Use a computed property instead when the result is pure derived
state; it is easier to reason about and remains lazy.

## Performance Guidelines

- Prefer PAXEL expressions for template-local derived values. The compiler
  supplies their dependency edges.
- List every dependency of a Rust computed property explicitly.
- Use `read` when inspecting a large value without cloning it.
- Use `set_if_neq` when equal writes are common and equality is inexpensive.
- Add cutoffs at stable semantic boundaries, not to every computed property.
- Keep computed evaluators, cutoff predicates, and subscriptions short.
- Move blocking I/O and other long-running work outside the synchronous
  reactive graph.

See [Data Binding & Expressions](data-binding-expressions.md) for PAXEL syntax
and [the properties API](api/pax-runtime-api/properties.md) for the complete Rust
surface.
