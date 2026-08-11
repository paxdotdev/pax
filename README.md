# Pax

**A language built for living interfaces.**

Pax is a language-first GUI framework for Rust, combining declarative UI
authoring with Rust application logic and a portable, high-performance runtime
for macOS, iOS, iPadOS, and the web.

[Get started with the Pax CLI](https://docs.pax.dev/getting-started/) ·
[Read the docs](https://docs.pax.dev/) ·
[Explore examples](examples/src) ·
[Join Discord](https://discord.com/invite/Eq8KWAUc6b)

## Get started

**(1) Set up your workstation**

Traditional: Follow the [Getting Started](https://docs.pax.dev/getting-started/) chapter in the docs.

AI-assisted: Prompt your coding agent naturally and include the primer URL:

> Build me a spaceship shooter game with https://pax.dev/ai



**(2) Run your first project**

```sh
cargo install pax-cli
pax-cli create my-first-project
cd my-first-project
pax-cli run
```

`pax-cli run` targets the web by default. Use `--target macos`, `--target ios`,
or `--target ipados` to run an Apple target from a supported macOS workstation.


## Source example

A Pax component has two layers. A `.pax` template declares the interface tree,
layout, styling, events, and motion; its side-effect-free PAXEL expressions
derive values from application state. Rust owns state changes, event handlers,
platform integration, and other side effects.

This component shows that loop in miniature: PAXEL derives the label and
rotation from `count`, while the click binding routes an event to Rust:

```pax
// src/lib.pax
<Group
    x=50%
    y=50%
    width=240px
    height=140px
    rotate={(count * 4)deg}
    @click=self.increment
>
    <Text id=label text={"Clicks: " + count} />
    <Rectangle
        fill=rgb(100%, 0, 0)
        corner_radii={RectangleCornerRadii::radii(12.0, 12.0, 12.0, 12.0)}
    />
</Group>

@settings {
    #label {
        width: 100%
        height: 100%
        style: {
            font_size: 28px
            fill: WHITE
            align_vertical: TextAlignVertical::Center
            align_horizontal: TextAlignHorizontal::Center
            align_multiline: TextAlignHorizontal::Center
        }
    }
}
```

```rust
// src/lib.rs
use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Counter {
    pub count: Property<usize>,
}

impl Counter {
    pub fn increment(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.count.set(self.count.get() + 1);
    }
}
```

Pax UI definitions are 100% declarative by grammatical constraint, relegating imperative work to the Rust layer.

In this example, when the handler updates `count`, Pax reactively reevaluates the expressions
that depend on it and updates the running interface. Pax propagates the change
through its reactive dependency graph, invalidating the affected layout and
rendering work, including the minimal set of necessary GPU uploads (when using a default GPU rendering backend).  This model is inspired by the spreadsheet, but instead of updating cells, Pax updates pixels.

Browse the [repository examples](examples/src), or open a live example on the
[Pax website](https://www.pax.dev/) and click the **View Source** button on any of the page sections.

## Build your imagination

Pax combines the building blocks of an application: components, reactive state,
routing, events, and responsive layout -- in the same cohesive scene and coordinate space as vector drawing,
paths, gradients, masks, clipping, occlusion, opacity, and animation.

Any property can be animated, with total creative control. Animations can be driven imperatively with a
tweening API, or declaratively with timelines in `.pax` templates. Timelines
can also be bound to fire any time elements enter, leave, or reflow from data-driven lists.

Pax aims to offer a high creative ceiling for artists with a powerful and
accessible technical toolkit. We wish to empower art, to help make computing more human.

## One Rust codebase for native and web

Build for web browsers, macOS, iOS, and iPadOS from the same `.pax` templates
and Rust application logic. Pax uses platform-native text, form controls, and
scrolling where native behavior matters, then composites those elements with
rendered content into one cohesive scene. Native text and controls, selection
and editing, and image alternatives provide accessibility foundations. Broader
accessibility work, including reading order, tab order, annotation, and audit
work, remains ongoing.

Rendering remains target-aware: Apple builds use Metal-backed rendering, while
web builds select WebGPU where supported and fall back to CPU rendering where
necessary.

## Prioritizing performance

Performance isn't an accident -- nor is it ever solved. Every new Pax feature
requires attention to its cost across targets.

Pax's reactive graph limits updates to dependent properties and dirty layout
and rendering work. On GPU-backed targets, retained resource caches, targeted
replay, and coverage-based culling reduce redundant mesh and texture uploads
and off-screen rendering. Rust gives the core runtime direct control over
allocation, memory layout, and execution without incurring garbage collector pauses in that
path.

Pax targets high performance across several axes:

- **Frame rate and animation.** In informal testing on an Apple Silicon Mac
  driving a 240 Hz 1080p display, most repository examples have run at 240 fps
  during vector animation and scrolling. Pax does not yet maintain a rigorous
  cross-hardware benchmark suite; building one would be a valuable open-source
  contribution.
- **Memory footprint.** iOS Safari's memory constraints make it a useful stress
  target, and current examples have run there without observed out-of-memory
  failures. Published memory benchmarks are still needed to quantify and
  maintain this behavior.
- **Disk and network footprint.** Representative release Wasm bundles
  currently gzip to approximately 393-503 KB. The longer-term goal is to bring
  minimal applications toward 100-200 KB by modularizing optional runtime
  components.
- **Idle energy footprint.** Pax currently keeps a display-synchronized tick
  running while an application is mounted, so an otherwise idle application
  still performs a small amount of per-frame CPU work. When nothing has dirtied
  the scene, the render path exits before scene traversal, resource uploads,
  command encoding, or GPU submission.
  [A demand-driven frame scheduling proposal](pax-docs/book/src/design/demand-driven-frame-scheduling.md)
  is one idea for how Pax could further reduce idle CPU.


## Platform Support

| Area | Current support                                                                                                                                                                                                                                                                 |
| --- |---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Application targets | Web browsers via WebAssembly, native macOS, iOS, and iPadOS                                                                                                                                                                                                                     |
| Development workstations | macOS, Debian/Ubuntu Linux, and Windows for the web install/create/run workflow; Apple targets require macOS and Xcode                                                                                                                                                          |
| Template hot reload | `.pax` changes reload in debug sessions on web, macOS, iOS, and iPadOS.  Hot reloading is only available on debug builds; release build program definitions are immutable.                                                                                                      |
| Rust logic hot reload | Supported on web and macOS; iOS and iPadOS require a rebuild and relaunch after Rust changes. The default `--hot-reload=pax` avoids automatic logic rebuilds; opt in with `pax-cli run --hot-reload=all`. |
| Future plans | Pax is designed to be platform agnostic, with a well defined interface between the shared engine and platform-specific bindings.  Support is planned for Linux/GTK, Android, and Windows, and other platforms (like tvOS, watchOS, and embedded) are within reach. Any target is open to open source contribution.              

## Development loop

Pax UI definitions are 100% declarative by grammatical constraint.  After you declare the UI in .pax, the reactive runtime propagates changes through dependent properties, updating
only the layout and rendering work that are dependent on dirty state. This model is inspired by a spreadsheet, but instead of updating cells, Pax updates pixels.

The framework and today's developer tooling -- including hot reload, source mapping, runtime inspection,
programmatic screenshots, and manual event-triggering -- all ship open source in this repository.

`pax-cli dev` can capture screenshots, inspect the expanded scene tree, query
selectors and hit targets, read web-session logs, and apply source-aware
template updates against a running debug session.

The same structured, live, inspectable loop is designed for both human and AI authors.

## Project status

**Pax is ready for builders.** The project has been under development since
2021 and now offers a coherent, runnable framework for web browsers, macOS,
iOS, and iPadOS. Pax remains pre-1.0: expect evolving APIs and some
platform-specific differences.

## License

Pax is licensed under either the [MIT license](LICENSE-MIT) or the
[Apache License 2.0](LICENSE-APACHE), at your option.

© PaxCorp Inc.

## Build with Pax

Install the CLI, create a project, and start building:

```sh
cargo install pax-cli
pax-cli create my-first-project
cd my-first-project
pax-cli run
```
