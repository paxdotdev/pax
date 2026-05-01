# Runtime Child Ontology

Authoring Date: 2026-04-26

<!-- summary: Internal terminology for child families, container payload, and exit retention. -->
<!-- tags: architecture, runtime, containers -->

This note is for maintainers working on the Pax runtime, control flow, slots, and layout containers.

The runtime carries several child-related concepts. The useful way to reason about them is to keep the axes narrow:

1. Semantic role
2. Lifecycle slice

## Semantic Role

### Received children

`received_children` are the semantic payload a node received from its caller.

- For `Group`, this is usually the authored children nested inside the `Group`.
- For `Stacker`, `Grid`, and other slot-driven containers, this is the payload passed into the container by its caller.

This is the canonical content view for container logic.

### Encapsulated children

Encapsulated children are the node's private implementation structure.

- For a component, this is the expanded tree from its own `.pax` template.
- For a primitive, this is any helper structure it assembles internally.

Encapsulated children are usually an implementation detail. Containers should generally not plan layout from them.

### Projected children

Projected children are an engine transport concept, not the primary semantic abstraction.

Projection is how the runtime threads received payload through encapsulated structure so `slot(...)` can render it. In other words:

- callers think in terms of received children
- the engine may transport them as projected children
- `Slot` consumes projected children inside encapsulated structure

Most container logic should read `received_children`, not `projected_children`.

## Lifecycle Slice

Received payload is further split by lifecycle:

### Active received children

`NodeContext::received_children` exposes the active payload set a container should normally measure and arrange.

### Retained received children

`NodeContext::retained_received_children` exposes payload nodes that have logically left the active set but are still mounted so `@out` transitions can finish.

These are useful for:

- ghost exits
- overlay rendering during exit
- container-specific exit choreography

They are not part of the active payload set.

## Render Tree vs Semantic Payload

The render tree has its own concrete child lists, including mounted children and retained exits. That is a different concern.

Use this rule:

- if you are deciding what content a container semantically owns, use `received_children`
- if you are deciding what still exists on-screen while an exit is in flight, also consult `retained_received_children`
- if you are wiring `slot(...)`, use `projected_children`

## Provenance: Owned vs Projected

The runtime still needs a provenance selector to decide how a node's semantic payload is sourced.

That is what `ReceivedChildrenSource` does:

- `Owned`: the node's active child tree already is its received payload
- `Projected`: the node receives payload from its caller, and the runtime transports that payload through projection

This is an engine detail used to normalize the semantic API. Consumers should usually care about the normalized result, not the provenance selector itself.

## Geometry Seam

Container-specific layout should not need to invent wrapper nodes just to position received payload.

The runtime exposes a geometry seam through `ContainerFrame`:

- a container computes parent-local frames for received children
- the runtime composes that frame before the child's own layout properties

Relevant API and internal entry points:

- [`NodeContext`](api/internal/pax-runtime/api.md)
- `pax-runtime::container` in source
- [`ExpandedNode`](api/internal/pax-runtime/engine.md)
- [`layout`](api/internal/pax-runtime/layout.md)

This is the seam used by `Stacker` for container-owned reflow and exit handling.

## Practical Rules

When working on containers, slots, or transition retention:

1. Prefer `received_children` for semantic payload.
2. Treat `projected_children` as transport, not content.
3. Use `retained_received_children` for exit-retained payload, not as active content.
4. Keep encapsulated implementation structure out of container layout decisions unless the primitive is explicitly introspecting its own private tree.
