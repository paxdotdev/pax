# Pax

**A language-first GUI framework for Rust.**

Pax combines declarative UI authoring with Rust application logic and a
portable, performance-oriented runtime. Build native macOS, iOS, and iPadOS
apps—or target web browsers through WebAssembly.

[Get started with the Pax CLI](https://docs.pax.dev/getting-started/) ·
[Read the docs](https://docs.pax.dev/) ·
[Explore examples](examples/src) ·
[Join Discord](https://discord.com/invite/Eq8KWAUc6b)

## Get started

**(1) Set up your workstation**

Pax projects are Rust projects. Follow the [workstation setup instructions](https://docs.pax.dev/getting-started/) in the docs.

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
        fill=INDIGO
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

When the handler updates `count`, Pax reactively reevaluates the expressions
that depend on it and updates the running interface. Pax propagates the change
through its reactive dependency graph, invalidating the affected layout and
rendering work—spreadsheet-like reactivity carried through the rendering
pipeline.

Browse the [repository examples](examples/src), or open a live example on the
[Pax website](https://www.pax.dev/) and choose **View Source**.

## Build your imagination

Pax combines the building blocks of an application—components, reactive state,
routing, events, and responsive layout—with direct control over vectors,
paths, gradients, masks, clipping, occlusion, and animation.

Any property can be animated. Animations can be driven imperatively with a
tweening API, or declaratively with timelines in `.pax` templates. Timelines
can also be bound to fire any time elements enter, leave, or reflow, or can be
triggered imperatively in Rust.

Pax aims to offer a high creative ceiling for artists with a powerful and
accessible technical toolkit.
Pax is designed to empower art, to help make computing more human.

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

## Platforms and development loop

| Area | Current support |
| --- | --- |
| Application targets | Web browsers via WebAssembly, macOS, iOS, and iPadOS |
| Development workstations | macOS, Debian/Ubuntu Linux, and Windows for the web install/create/run workflow; Apple targets require macOS and Xcode |
| Template hot reload | `.pax` changes reload in debug sessions on web, macOS, iOS, and iPadOS |
| Rust logic hot reload | Supported on web and macOS; iOS and iPadOS require a rebuild and relaunch after Rust changes |

The reactive runtime propagates changes through dependent properties, gates
layout and rendering work on dirty state, and culls content outside the
viewport. Release cartridges omit debug and authoring metadata.

`pax-cli dev` can capture screenshots, inspect the expanded scene tree, query
selectors and hit targets, read web-session logs, and apply source-aware
template updates against a running debug session. The framework and today's
developer tooling—including hot reload, source mapping, inspection,
screenshots, and event-driving—ship open source in this repository. No
companion application is required to evaluate or use Pax.

The same structured, live, inspectable loop supports both human and AI-assisted
authoring.

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
