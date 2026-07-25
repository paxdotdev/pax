# Legacy `pax-designer` Retrospective

Status: historical prior art; **not a current Pax example or API reference**.

Final pre-removal snapshot: `6093c29d4`

<!-- summary: Retrieval guidance and durable lessons from the removed pax-designer prototype. -->
<!-- tags: pax-designer, legacy, designtime, authoring, retrieval -->

## Retrieval guidance

The legacy `pax-designer` was the largest integrated Pax-authored application of
its era. Its final snapshot contains 36 Pax templates, 86 Rust files, roughly
17,700 lines of source, and about 38 Pax components. That makes it useful
evidence that Pax can support a dense, multi-component tool UI.

It should not be retrieved as a model for writing current Pax applications.
The implementation predates major improvements to layout, autosizing, input,
scrolling, routing, animation, and the developer-tool protocol. It was excluded
from ordinary workspace validation and depended directly on engine,
designtime, manifest-ORM, and dedicated designer-build internals.

Use the archived source to answer historical or architectural questions. Use
the current book, API documentation, and compiling examples for syntax and
implementation guidance.

## What the prototype proved

- A substantial tool can itself be authored from cooperating Pax components,
  rather than requiring a separate UI technology.
- A tool shell can surround an embedded user program with panels, a scene tree,
  settings editors, overlays, a console, toolbars, and contextual controls.
- Explicit actions and transactions are useful boundaries for selection,
  pointer tools, tree mutation, undo/redo, and source-authoring operations.
- Typed coordinate spaces help organize stage, viewport, selection, and
  transform math in an editor-like application.
- Pax templates and Rust logic can divide responsibilities across a large
  component graph while retaining reactive presentation.

These are architectural lessons, not endorsements of the original APIs or
implementation techniques.

## What should not be copied

| Legacy pattern | Current guidance |
| --- | --- |
| Direct `pax-engine` and `pax-std` integration in userland | Start from `pax-kit` and current generated project structure. |
| A dedicated designer compiler/runtime mode and second manifest | Build authoring tools on generic designtime protocols and ordinary Pax programs. |
| Manual fixed-pixel panel grids and duplicated layout arithmetic | Prefer current autosizing, responsive units, Stackers, and layout primitives. |
| Large amounts of explicit `Property::computed`, dependency arrays, subscriptions, and global `RefCell` state | Use declarative bindings and contemporary component-local state first; introduce programmatic graph plumbing only where required. |
| UI workarounds for missing z-order, pointer capture, focus, or Scroller behavior | Verify current runtime and chassis behavior before reproducing an old workaround. |
| Manifest-ORM types and engine node handles as ordinary application architecture | Treat them as designtime/editor infrastructure, not general userland patterns. |
| Source copied from a crate outside normal workspace validation | Prefer examples and tests that compile in the current workspace. |

The source also predates current timelines, routing patterns, runtime themes,
scoped lighting, and other newer language/runtime capabilities. Its silence on
those features is historical, not prescriptive.

## Current replacements

For current implementation patterns, begin with:

- `pax-docs/book/src/getting-started.md` and the adjacent language, binding,
  event, layout, routing, animation, and scrolling chapters
- `examples/src/router-playground` for a cohesive multi-component application,
  responsive navigation, routing, and transitions
- `examples/src/auto-sized-containers` for contemporary intrinsic layout
- `examples/src/scroll-garden` for scrolling and composed animated content
- `examples/src/timeline-playground` and `examples/src/transition-grid` for
  declarative motion
- `examples/src/runtime-settings-themes` for reusable settings and themes
- `examples/src/example-host` for embedding and hosting examples
- `pax-designtime/src/messages.rs`, `pax-runtime/src/designtime_support.rs`, and
  `pax-compiler/src/design_server` for the active generic developer-tool seam
- `pax-docs/book/src/design/designtime-host-programs-spec.md` for future
  Pax-authored host-tool direction

No single current example completely replaces the designer's integrated scale.
The most valuable future additions would be a modern application-shell example
and a compiling canvas/editor example covering selection, transforms, dragging,
and undo without reviving a special product mode.

## Inspecting the archive

The source remains available through normal Git history; no history rewrite or
object purge accompanied its removal. For example:

```sh
git ls-tree -r --name-only 6093c29d4 -- pax-designer
git show 6093c29d4:pax-designer/src/lib.pax
git show 6093c29d4:pax-designer/src/model/action/mod.rs
```

Keeping the implementation out of HEAD prevents stale code from outweighing
current examples in retrieval, while this record preserves where to find the
original evidence and how to interpret it.
