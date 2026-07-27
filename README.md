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

Pax projects are Rust projects. After completing the
[workstation setup](https://docs.pax.dev/getting-started/), install the CLI,
create a project, and run it:

```sh
cargo install pax-cli
pax-cli create my-first-project
cd my-first-project
pax-cli run
```

`pax-cli run` targets the web by default. Use `--target macos`, `--target ios`,
or `--target ipados` to run an Apple target from a supported macOS workstation.

## A declarative language designed for interfaces

`.pax` templates give UI trees, layout, bindings, responsive behavior, and
motion a concise declarative home. PAXEL, Pax's side-effect-free reactive
expression language, derives values from application state. Rust owns state,
event handlers, platform integration, and other side effects.

## A higher creative ceiling

Pax brings components, routing, responsive layout, native text and controls,
and scrolling into the same composition model as vectors, paths, gradients,
masks, and first-class animation. Native elements and rendered content share a
transformed, clipped, and occluded scene.

## One Rust codebase for native and web

Build for web browsers, macOS, iOS, and iPadOS from the same `.pax` templates
and Rust application logic. Pax uses platform-native text, form controls, and
scrolling where native behavior matters, then composites those elements with
rendered content in one scene. Native text and controls, selection and editing,
and image alternatives provide accessibility foundations; broader reading
order, tab order, accessibility annotation, and broad audit work is ongoing.

Rendering remains target-aware: Apple builds use Metal-backed rendering, while
web builds select WebGPU where supported and fall back to CPU rendering where
necessary.

## How Pax is authored

A Pax component is a declarative template paired with Rust application logic.
Here, the PAXEL expression describes the label and rotation as a function opf `count`, while
the click handler performs the state change:

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
that depend on it and updates the running interface.

## Platforms and development loop

| Area | Current support |
| --- | --- |
| Application targets | Web browsers via WebAssembly, macOS, iOS, and iPadOS |
| Development workstations | macOS, Debian/Ubuntu Linux, and Windows for the web install/create/run workflow; Apple targets require macOS and Xcode |
| Template hot reload | `.pax` changes reload in debug sessions on web, macOS, iOS, and iPadOS |
| Rust logic hot reload | Supported on web and macOS; iOS and iPadOS require a rebuild and relaunch after Rust changes |

`pax-cli dev` can capture screenshots, inspect the expanded scene tree, query
selectors and hit targets, read web-session logs, and apply source-aware
template updates against a running debug session. The framework and today's
developer tooling—including hot reload, source mapping, inspection,
screenshots, and event-driving—ship open source in this repository. No
companion application is required to evaluate or use Pax.

## Project status

**Pax is ready for builders.** The project has been under development since 2021, and now offers
a coherent, runnable framework for Apple-platform native apps and WebAssembly web apps. 
Pax remains pre-1.0: expect evolving APIs and some platform-specific differences.

## Explore

- Run the [Increment](examples/src/increment) example for a small end-to-end
  component.
- Explore [Path Drawing](examples/src/path-drawing) for vector graphics and
  motion.
- Explore [Liquid Glass](examples/src/liquid-glass) for native Apple
  compositing.
- Browse [all examples](examples/src) or the complete
  [documentation](https://docs.pax.dev/).
- Visit [pax.dev](https://pax.dev/), [star Pax on
  GitHub](https://github.com/paxdotdev/pax), read the
  [contribution guide](CONTRIBUTING.md), or join the
  [community Discord](https://discord.com/invite/Eq8KWAUc6b).

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
