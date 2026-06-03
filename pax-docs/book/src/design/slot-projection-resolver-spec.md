# Slot Projection Resolver (Draft)

Authoring Date: 2026-05-31

<!-- summary: Draft semantics for explicit and remainder slot projections. -->
<!-- tags: slots, projection, runtime, templates -->

## Problem

Pax components can project caller-provided children with `slot(index)`, but a
component that wants to render "some specific children, then the rest" has had
to mirror child counts or invent a wrapper API. That makes simple composition
patterns awkward and pushes declarative template structure into Rust props.

The resolver should support this common shape directly:

```pax
<Group>
    slot(0)
    slot(1)
    slot(2)
    <Group>
        slot()
    </Group>
</Group>
```

## Goals

- Keep the authoring surface small: `slot(expr)` for explicit projection and
  `slot()` for the remaining children.
- Treat `slot()` as a declarative remainder projection, not as a persistent
  mutable cursor.
- Recompute projection from current inputs whenever projected children, active
  slot sites, or slot index expressions change.
- Let `if` and `for` create active slot sites naturally through existing
  template control flow.
- Warn when an explicit slot site has no rendered home because its requested
  child is missing or was already consumed.
- Do not warn when `slot()` receives no children.

## Non-goals

- No slot-specific mini-language such as `slot(3..)` or `slot(*)`.
- No grammar-wide open-ended range changes for this feature.
- No author-controlled multi-home projection. A projected child has one
  rendered home in a single resolution.
- No first-pass attempt to expose the computed resolution to user code.

## Authoring Surface

`slot(expr)` projects exactly one child by index. The expression is evaluated as
the slot site's current index. If the index is negative, out of range, or already
consumed by an earlier slot site, the slot renders no children and emits a
warning.

`slot()` projects all children not consumed by earlier active slot sites in the
same containing component. Empty remainder is valid and silent.

Both forms are scoped to the component whose template owns the slot site. Slot
sites inside nested component boundaries belong to that nested component's own
resolver.

## Resolution Semantics

For each component instance, the runtime resolves slot projection from:

1. the current ordered projected child list,
2. the current active slot sites in the component's encapsulated tree, and
3. each explicit slot site's current index expression.

The resolver walks active slot sites in rendered child-tree order. At each site:

- `slot(n)` consumes child `n` if it exists and has not already been consumed.
- `slot()` consumes every unconsumed child in original projected-child order.

Resolution is recomputed from scratch when its inputs change. There is no retained
"drain position" across frames; remainder is purely a function of earlier active
slot sites in the current resolution.

### Examples

Given projected children `[A, B, C, D]`:

| Template projection sites | Rendered result |
| --- | --- |
| `slot(0)`, `slot()` | first site gets `[A]`; remainder gets `[B, C, D]` |
| `slot(2)`, `slot()` | first site gets `[C]`; remainder gets `[A, B, D]` |
| `slot(8)`, `slot()` | explicit site gets `[]` with a warning; remainder gets `[A, B, C, D]` |
| `slot(1)`, `slot(1)`, `slot()` | first explicit gets `[B]`; duplicate gets `[]` with a warning; remainder gets `[A, C, D]` |
| `slot()`, `slot(0)` | remainder gets `[A, B, C, D]`; later explicit gets `[]` with a warning |
| inactive `if { slot(0) }`, `slot()` | remainder gets `[A, B, C, D]` |
| active `if { slot(0) }`, `slot()` | explicit gets `[A]`; remainder gets `[B, C, D]` |

## Reactivity

Dynamic explicit indices participate in the reactive graph:

```pax
slot(self.selected)
slot()
```

When `self.selected` changes, the component re-resolves projection. The remainder
slot receives the children not consumed by the new selected index.

Control-flow sites behave the same way. If an `if` branch adds or removes a
prior slot site, or a `for` loop changes the order or number of explicit slot
sites, the same component re-resolves from the active tree.

## Diagnostics

Required diagnostics:

- warn for negative `slot(expr)` indices,
- warn for out-of-range `slot(expr)` indices,
- warn when an explicit slot asks for a child already consumed by an earlier
  slot projection site.

Silent cases:

- `slot()` with zero remaining children,
- `slot()` when the component received no projected children.

Recommended authoring rule: place `slot()` after any explicit `slot(expr)` sites
that should reserve children. A later explicit site can render empty because an
earlier remainder has already consumed that child.

## Example Project

`examples/src/slot-projection-resolver` visualizes projection resolution with bright
ROYGBIV gem-tone tiles on a dark background. Each panel uses `Stacker` buckets
to show where the projected children land:

- static first-three-then-remainder projection,
- dynamic `slot(self.selected)` followed by `slot()`,
- out-of-range explicit projection,
- duplicate explicit indices,
- conditional slot sites,
- repeated slot sites from a `for` loop.

Run the example from the project directory:

```sh
./pax run --target=web
```

Use the controls across the top to change the projection inputs and watch the
same projected child list re-deal into each panel.

Current layout note: a `slot()` site can project multiple children, but a
`Stacker` still treats the `Slot` site itself as its direct child. The example
keeps each remainder bucket compact while visualizing the resolver behavior; a
layout-transparent "explode projected children into Stacker cells" behavior is
a separate container-layout feature, not part of this resolver spec.

## Implementation Notes

The first implementation compiles `slot()` as a `Slot` control-flow node with an
`is_remainder` runtime property. Explicit slot sites keep using the existing
`slot_index_expression` manifest field.

The runtime stores a component-local invalidation property so slot children can
recompute when projected children, active component child structure, or explicit
slot indices change.
