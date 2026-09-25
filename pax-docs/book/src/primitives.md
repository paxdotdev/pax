# Primitives
<!-- summary: Extend Pax with Rust-backed runtime nodes: when to use a primitive, how to register one, and the rendering and lifecycle responsibilities it carries. -->
<!-- tags: primitives, components, runtime, rendering, native, extension -->

A component gives a reusable interface its own template, properties, and
actions. A primitive is a more advanced alternative: it implements a template
element directly in Rust through Pax's runtime interfaces. This gives its
author control over lower-level behavior such as drawing, child expansion,
and integration with a platform surface.

You already use primitives when you place elements such as `Ellipse`,
`Group`, and `Text` in a template. Their standard-library implementations
connect those tags to the renderer and runtime. You can use the same extension
point to build a new kind of element in your own project.

## Components and primitives

From the caller's perspective, both can appear as template tags and expose
`Property<T>` inputs. Their implementations differ:

| Kind | Implementation |
| --- | --- |
| Template-backed component | A `#[pax]` type with `#[file(...)]` or `#[inlined(...)]`; its template composes other nodes. |
| Primitive | A `#[pax]` type with `#[primitive("...")]`; a Rust `InstanceNode` implements its runtime behavior, with no attached `.pax` template. |
| Data type | A `#[pax]` type without a template or primitive declaration; it describes values such as the [`NoteEntry` record](components-composition.md#ranges-and-collections). |

For application UI, begin with [components and composition](components-composition.md).
A reusable illustration can combine `Path` and the existing drawing elements;
a new route presentation can use [a custom route branch](routing.md#custom-route-branches).
A primitive becomes useful when the capability needs runtime hooks those
elements do not expose: a specialized drawing operation, a new container
behavior, or a platform integration.

That additional control comes with responsibility for rendering, lifecycle,
and target-specific behavior. Choose it for the capability you need; changing
a component into a primitive does not automatically improve performance.

This chapter assumes familiarity with Rust, [properties](state-properties.md),
and [component composition](components-composition.md). It starts with a
minimal registration scaffold, then explains the obligations of a working
primitive and points to implementations in the engine and standard library.

## Authoring primitives

### Declare and connect a runtime node

The extension point is available in userland. The compiler generates a
factory that calls your implementation through the import path in
`#[primitive(...)]`; adding a node does not require an enum entry in
`pax-std`.

There are two Rust types to understand:

- The `#[pax]` type declares the properties visible to authors.
- Its `InstanceNode` implementation supplies construction and runtime hooks.
  A shared `BaseInstance` carries the compiler-provided configuration.

The following scaffold establishes that connection. It deliberately has no
drawing or platform side effects yet. Create `src/runtime_marker.rs`:

```rust
use pax_kit::*;
use pax_kit::pax_engine::api::Layer;
use pax_kit::pax_engine::pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs,
};
use std::rc::Rc;

#[pax]
#[primitive("crate::runtime_marker::RuntimeMarkerInstance")]
pub struct RuntimeMarker {
    pub value: Property<f64>,
}

pub struct RuntimeMarkerInstance {
    base: BaseInstance,
}

impl InstanceNode for RuntimeMarkerInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(args, InstanceFlags {
                invisible_to_slot: false,
                invisible_to_raycasting: true,
                layer: Layer::DontCare,
                is_component: false,
                is_slot: false,
            }),
        })
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("RuntimeMarker").finish()
    }
}
```

The three methods shown are required by `InstanceNode`. The flags describe a
non-drawing node that does not intercept pointer hit testing. A drawing
primitive would choose `Layer::Canvas` and provide rendering behavior; a
native surface has a different layer and platform-message lifecycle.

In `lib.rs`, add the module and expose its author-facing type:

```rust
pub mod runtime_marker;
pub use runtime_marker::RuntimeMarker;
```

It can now be instantiated in a template:

```pax
<RuntimeMarker value=0.5 />
```

It occupies a runtime node but paints nothing. Use this as a registration
scaffold, then implement the specific hooks your capability needs. The
`crate::...` path above refers to this application's Rust module. A primitive
distributed in a library needs a path accessible from the consuming
application's generated code; the standard library uses paths such as
`pax_std::drawing::ellipse::EllipseInstance`.

### Implement behavior at the right level

An `InstanceNode` can serve several expanded occurrences, for example when
a template node is repeated. Each occurrence has an `ExpandedNode` with its
own properties, geometry, children, and lifecycle state. Keep per-occurrence
state there, or in state captured by an effect attached to that occurrence.
Putting a mutable per-item value directly on the shared instance can make
repeated items interfere with one another.

Runtime hooks receive the expanded node. Access its typed properties with
`expanded_node.with_properties_unwrapped(|properties: &mut RuntimeMarker| {
... })`, and use `transform_and_bounds` for its resolved geometry. This is
lower-level access than a component handler's `NodeContext`; keep ordinary
application actions in [event handlers](event-handling-rust.md).

The main implementation responsibilities depend on the capability:

- **Reactive work:** bind an effect in `handle_mount` to the properties and
  geometry it reads. Standard drawing primitives use `changed_listener` to
  mark their retained drawing dirty. Reserve `update` for work that cannot
  be driven by those dependencies; it requires opting in through
  `requires_non_reactive_update`.
- **Canvas drawing:** implement `render` through `RenderContext`. Follow the
  retained-node begin/end protocol, use the correct surface-local transform,
  register the node's opacity scopes, and invalidate drawing when its inputs
  change. `begin_bounded_canvas_node` supplies `paint_opacity`: use that value
  for drawing so a backend that composes subtrees does not apply ancestor
  opacity twice. The current `EllipseInstance` demonstrates
  `begin_bounded_canvas_node` and clearing the dirty flag only after
  `end_node` succeeds. Unbounded geometry needs suitable coverage bounds.
- **Coverage and interaction:** keep coverage paths, coverage opacity, and
  hit testing consistent with what the node draws. These hooks affect masks,
  native/canvas ordering, and pointer selection. The default hit test uses
  layout bounds; unusual shapes may need `ray_cast_test`.
- **Children and cleanup:** a container must manage child expansion and scope;
  overriding mount behavior also means preserving the needed child setup.
  Release subscriptions, resources, and native surfaces on unmount. Respect
  the [received-versus-projected child model](components-composition.md#slots) for container content.

These runtime interfaces evolve with the engine. Work from the implementation
in the Pax version your project uses and test the capability on each backend
you intend to support. Exercise repeated instances, reactive updates, resize,
transforms, opacity, clipping, and removal—not just the first rendered frame.
Check both debug and release builds.

### Native integration and engine changes

A userland primitive can use the existing rendering and runtime interfaces.
A completely new native widget may also need message types, platform-side
creation/update/deletion handlers, event serialization, and accessibility
behavior. Setting `Layer::Native` alone does not create that support.

That work can span `pax-message`, the web interface, and the Swift interfaces
for macOS, iOS, and iPadOS. New renderer operations or template/control-flow
syntax similarly need engine or compiler support. Keep the supported-target
boundary explicit.

When adding data that must cross the compiler/runtime boundary, update the
release representation and its round-trip tests too. A primitive's normal
property declaration participates in generated descriptors; adding a new
manifest field or value form entails a broader audit. See
[How Pax Runs](how-pax-runs.md) for the execution model and the
[maintainer appendix](architecture-runtime-cartridge.md) for cartridge details.

### Source examples to study

Read these alongside the code in your checkout:

- [`Ellipse` / `EllipseInstance`](https://github.com/paxproject/pax/blob/dev/pax-std/src/drawing/ellipse.rs)
  — a bounded canvas primitive: properties, dirty tracking, geometry,
  material-aware drawing, and coverage.
- [`Group` / `GroupInstance`](https://github.com/paxproject/pax/blob/dev/pax-std/src/core/group.rs)
  — child expansion, layout measurement, and conditional native-surface
  integration.
- [`Text` / `TextInstance`](https://github.com/paxproject/pax/blob/dev/pax-std/src/core/text.rs)
  — native create/update/delete messages and text measurement. Follow the
  message to the target interface for the platform half.
- [`InstanceNode`, `BaseInstance`, and `InstantiationArgs`](https://github.com/paxproject/pax/blob/dev/pax-runtime/src/rendering.rs)
  — the runtime contract and default hook implementations.
- [`ComponentInstance`](https://github.com/paxproject/pax/blob/dev/pax-runtime/src/component.rs)
  — the engine implementation behind ordinary template-backed components,
  including scope and slot projection.

The [runtime API reference](api/internal/pax-runtime/rendering.md#instancenode)
is useful for orientation. The source links follow the repository's development
branch; use the corresponding files at your dependency's release when
implementing a primitive.

## Read more

[Components and Composition](components-composition.md) covers reusable
template-backed UI, slots, and shared state. [Drawing and Styling](drawing-styling.md)
introduces the existing drawing elements, and [How Pax Runs](how-pax-runs.md)
explains the runtime and rendering model. For the lower-level interface,
continue with the [runtime API reference](api/internal/pax-runtime/rendering.md#instancenode)
and the source examples above.
