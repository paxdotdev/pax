# What is Pax?
<!-- summary: Pax's authoring model, runtime, targets, and current maturity. -->
<!-- tags: foundations, mental-model, rust, paxel, targets -->

Pax is a language-first GUI framework for Rust. A Pax application combines
declarative `.pax` templates, reactive properties and PAXEL expressions, Rust
application logic, and a portable runtime. The same interface model currently
targets web browsers, macOS, iOS, and iPadOS.

Pax is for builders who want the structure of an application framework without
giving up direct control over layout, vector drawing, composition, and motion.
It exists to widen the expressive range of software while keeping the source
legible enough to reason about.

If you already know you want to try it, go directly to
[Getting Started](getting-started.md). That chapter stands on its own; you do
not need to read the rest of the documentation first.

## A language built for interfaces

A Pax component usually has two source layers: a Rust type and a `.pax`
template. The template declares the component's interface tree, layout,
styling, bindings, control flow, event routing, responsive choices, and motion.
The Rust type exposes application state and owns handlers, data access,
platform integration, and other imperative work.

Inside a template, PAXEL expressions derive values from reactive properties.
They can combine values, choose between alternatives, perform unit arithmetic,
and feed the result into content, layout, styling, or animation. PAXEL is
side-effect-free: it describes relationships between values rather than a
sequence of commands.

Templates remain declarative and inspectable,
while Rust remains available for everything that genuinely changes the world:
responding to an event, loading data, writing a file, or updating application
state. In practice, most components are understood by reading the template and
Rust file together.

## Try it: Living Quilt

Living Quilt is the project created by `pax-cli create`. Move the pointer to
shift its light, click or tap the quilt to send a wave of color through the
tiles, and click the Pax card to replay its entrance. The scene combines
responsive components, Rust-driven motion, lighting, and feathered alpha masks.
Its color reveal requires the GPU renderer.

<pax-example path="living-quilt" title="Living Quilt" height="640" files="src/lib.pax,src/lib.rs,src/quilt_scene.pax,src/logo_card.pax,src/wave.rs"></pax-example>

The source tabs show how those parts fit together. You do not need to understand
the whole scene to start editing it; [Getting Started](getting-started.md#make-a-first-edit)
walks through a small change in the generated project.

## The Pax authoring loop

The central loop has four beats:

```text
 .pax template ──renders──> interface ──event──> Rust handler
       ▲                                           │
       │                                           │ sets
       └── PAXEL derives visible values ◀── reactive properties
```

1. A `.pax` template declares the interface and binds an event.
2. PAXEL expressions derive visible values from reactive properties.
3. An interaction invokes a Rust handler.
4. The handler updates a property, and Pax updates the dependent interface
   work.

When the property changes, Pax invalidates the values that depend on it and
reevaluates them when needed. This is the sense in which Pax is
spreadsheet-like: change an input, and the formulas downstream of it become
the new interface. Rust can also construct computed properties and
subscriptions directly when an application needs a relationship or effect
beyond the template.

The later chapters separate these responsibilities in more detail:
[Template Language & Structure](template-language.md) covers the declarative
tree, [Data Binding & Expressions](data-binding-expressions.md) covers PAXEL,
[State & Properties](state-properties.md) covers the reactive graph, and
[Event Handling & Rust Logic](event-handling-rust.md) covers the imperative
side of the loop.

## One interface model, multiple native targets

Pax uses a shared compiler and runtime, then connects them to each platform
through a target-specific chassis. At a high level, the compiler prepares the
program, the runtime expands its reactive scene, and the chassis integrates
rendering, input, native elements, and platform services.

[How Pax Runs](how-pax-runs.md) follows this process from source to the screen
and explains how reactive updates and rendering work fit together.

The current application targets are:

- web browsers, through WebAssembly;
- macOS;
- iOS; and
- iPadOS.

Web development is supported from macOS, Debian/Ubuntu Linux,
and Windows; Apple targets require a suitable macOS and Xcode environment.
[Getting Started](getting-started.md) has the current workstation setup.

A shared interface model also does not mean that every platform follows an
identical rendering path. Web builds select WebGPU where browser policy and
support permit it, with a Piet/CPU renderer where that path is required. Apple
targets use their native chassis. Backend-specific capabilities must therefore
state their target and fallback behavior rather than presenting one backend as
universal.

Pax also integrates native elements where platform behavior matters. Native
text, form controls, and scrolling can share a scene and coordinate space with
rendered shapes, images, paths, masks, and motion. The runtime manages the
boundary between those surfaces so application code can work with one
component tree while still respecting target-specific behavior.

## Why choose Pax?

The language is designed around interface work. Structure, reactive formulas,
units, event bindings, conditional settings, and declarative motion live close
to the elements they affect. Rust is not squeezed into the template language;
it remains the application layer and the escape hatch for unrestricted logic.

Pax also puts ordinary application architecture and a higher creative ceiling
in the same system. Components, state, routing, events, and responsive layout
can inhabit the same scene as vector primitives, paths, gradients, masks,
clipping, native-element composition, and animation. Builders do not have to
switch to a separate presentation model as an interface becomes more visual.

Finally, the portable runtime is intended for shipping rather than previewing
alone. It carries the reactive model across the current web and Apple targets
while retaining native integration and target-aware rendering. Because the
source and running scene are structured, the same model also supports tools
for hot reload, inspection, screenshots, and repeatable event-driven checks
for both human and agent-assisted workflows.

## Current scope and maturity

Pax is ready for builders: it is coherent and runnable enough to evaluate,
learn, and build real interfaces with today. It remains pre-1.0, so APIs can
evolve, some areas are still limited, and feature or backend maturity can vary
by target. “Ready for builders” is a transition in project maturity, not a
claim that every application is production-ready without its own evaluation.

Rust is the current application language. Native text and controls, selection
and editing, and image alternatives provide accessibility foundations, but
broader work such as reading order, tab order, annotations, and comprehensive
audits is still in progress.

The framework, language, compiler, runtime, CLI, hot reload, source mapping,
inspection, screenshots, and event-driving infrastructure ship as open source
in the [Pax repository](https://github.com/paxdotdev/pax). No companion product
is required to build, run, inspect, or ship a Pax application. Any future
companion would add to that open-source workflow rather than unlock it.

## Start building

[Install the CLI, create a project, and run it](getting-started.md).

If you prefer to explore before installing, browse the
[repository examples](https://github.com/paxdotdev/pax/tree/dev/examples/src)
and return to the authoring loop above when you want to understand how their
templates and Rust code fit together.
