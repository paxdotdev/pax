# Architecture: Runtime & Cartridge
<!-- summary: Historical maintainer design notes on the runtime/cartridge boundary; includes proposed directions. -->
<!-- tags: architecture, runtime, cartridge, build, release, debug -->

> **Historical design notes.** This appendix records a direction for Pax's
> runtime/cartridge boundary, including proposed packaging and future logic
> runtimes. Its envelope and sidecar descriptions are not a specification of
> today's release artifact. For the current builder-facing execution model,
> rendering behavior, and build guidance, read [How Pax Runs](how-pax-runs.md).
> Rust is the current application language. State that must survive a remount
> needs an owner outside the remounted component; `Property` alone does not
> make state durable.

Pax borrows its core mental model from the NES.

The console is always present. It knows how to power up, read input, render
frames, play audio, and expose hardware capabilities. The cartridge is what you
plug in to produce one specific experience.

Pax uses the same split:

- the **runtime kernel** is the always-present engine
- the **cartridge** is the program you mount into that engine

That split is useful for two reasons:

1. it keeps the runtime boundary clear
2. it gives Pax a path toward multiple logic runtimes, not just Rust

## The Runtime Kernel

The runtime kernel is the reusable part of Pax. It is responsible for:

- the reactive property graph
- PAXEL expression evaluation
- layout and rendering
- compositing, masking, and clipping
- event delivery
- native element projection
- platform integration for web, desktop, and mobile

You can think of the kernel as the "console." It can exist before any program is
mounted, and it can later mount a cartridge.

That matters for:

- normal application startup
- design tools
- hot reload
- future online sandboxes
- future embedded use cases where a host wants to attach a program later

## The Cartridge

The cartridge is the packaged running experience that gets mounted into the
kernel.

Today, the cartridge is no longer thought of as "generated Rust glue" first.
Instead, it is better understood as an envelope containing the parts needed to
run one Pax program.

At a high level, a cartridge contains:

- a **program representation**
- one or more **logic modules**
- **assets**
- optional **debug sidecars**

### Program Representation

The program representation is the declarative part of the app:

- component tree
- template nodes
- property bindings
- expressions
- timelines
- event bindings
- asset references

In debug-oriented flows, Pax still keeps a rich authoring representation around
because it is useful for designtime edits and inspection.

In release-oriented flows, the goal is a smaller, more portable representation
that keeps only what the runtime needs to execute the program.

### Logic Modules

The logic module is the executable half of the cartridge.

Today that usually means Rust application logic: event handlers, component code,
state transitions, and helper functions.

Architecturally, Pax now treats this as a separate concept from the declarative
program representation. That matters because it creates a cleaner path to
support additional logic runtimes in the future, such as JS or TS, while
remaining compatible with the same runtime kernel and the same standard library
components.

### Assets

Assets are the external resources the program needs:

- images
- fonts
- videos
- static files

The runtime kernel does not hardcode application assets. The cartridge carries
or references them.

### Debug Sidecars

Debug builds may carry extra sidecars and metadata used for:

- source mapping
- symbolic inspection
- live edits
- reload handoff
- designtime tooling

Those are intentionally not part of the minimal release payload.

## Program Representation vs. Logic Module

This boundary is worth making explicit.

The declarative program representation says things like:

- "instantiate this component"
- "bind this property to this expression"
- "attach this timeline to this field"
- "invoke this handler when this event fires"

The logic module says what the handler actually does.

That separation helps Pax in two directions at once:

- smaller release artifacts, because less behavior needs to be expanded into
  generated code
- cleaner multi-runtime support, because the declarative program can stay
  portable while the logic module can vary by host language

## Native Elements Are Projections

Pax treats native elements such as browser controls or platform widgets as
projections of engine state, not as the source of truth.

The authoritative state lives in the property graph inside the runtime kernel.
Native elements are updated by messages from the engine and report user input
back through interrupts.

That design is important for:

- deterministic rerendering
- hot reload
- future reload handoff
- consistent behavior across platforms

If durable UI state needs to survive reload or remount, it should be reflected
into `Property` state or another explicit durable state holder, not left hidden
inside the native control itself.

## Debug and Release Builds

Debug and release builds share the same semantics, but they optimize for
different goals.

### Debug Builds

Debug builds optimize for editability and observability.

They keep richer metadata and support tooling such as:

- designtime workflows
- source-aware diagnostics
- live program updates
- reload handoff
- full symbolic structure for inspection

Because of that, debug builds are intentionally much larger.

### Release Builds

Release builds optimize for shipping.

They aim to:

- strip metadata that the runtime does not need to execute the app
- reduce generated behavioral glue
- favor compact program representation
- keep the runtime/kernel boundary clean
- minimize transfer size, especially on web targets

Release builds intentionally do **not** expose the same live-edit and hot-swap
surface as debug builds.

## Footprint Differences

The difference in footprint between debug and release can be large, especially
for web builds.

The earlier, unversioned example-size snapshot has been removed because it
does not describe the current build. Measure the application and release you
intend to ship, including its assets. [Debug and release](how-pax-runs.md#debug-and-release)
describes the current build modes and measurement boundaries.

## Why This Architecture Matters

This runtime/cartridge split gives Pax a cleaner long-term direction.

For application authors, it means:

- the same declarative program model can target multiple platforms
- release builds can get smaller without changing authoring semantics
- debug builds can remain powerful without forcing that cost into production

For Pax itself, it means the engine is moving toward:

- a mountable runtime kernel
- a smaller and more portable program representation
- clearer host-side descriptor boundaries
- future support for additional logic languages

In short: the kernel is the console, the cartridge is the program, and the
boundary between them is now becoming a first-class part of Pax rather than an
implementation detail hidden inside generated code.
