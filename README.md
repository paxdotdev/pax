# Pax  

Pax is a user interface engine for native apps & websites.

Write application logic in Rust while declaring content & behavior in Pax's JSX-inspired user interface description language.

Pax compiles into native desktop/mobile apps and WebAssembly-driven web apps. It's very fast and very lightweight.

## What's in this repo?
 - The Pax compiler and an implementation of the Pax user interface description language
 - Native renderers + runtimes for iOS, macOS, and browsers/WebAssembly
 - Language bindings for supported programming languages, currently only Rust
 - Pax's CLI for compiling and managing projects 
 - Example projects

## Status

Developed in the open since 2022

**Currently Alpha** and unstable, under active full-time development.  Today, Pax compiles and runs for iOS, macOS, and Web.  The Web target is our leading edge of development.  Pax’s standard library includes components for drawing, layouts, and form controls.

[Take Pax for a spin!](https://docs.pax.dev/start-creating-a-project.html)

[Join our Discord](https://discord.com/invite/Eq8KWAUc6b) to chat with our team.  

We do not yet recommend building any production workloads with Pax. Targeting Beta in Q3 2024.


## Examples

See a **simple example** in the [introduction page](https://docs.pax.dev/) of our docs, or see a **more complex & running example** with the [pax.dev](https://www.pax.dev/) website, built in Pax.  Refer also to our [website's Pax source code](https://www.github.com/paxproject/www.pax.dev).

You can also run the [examples](https://github.com/paxproject/pax/tree/master/examples/src) in this monorepo:

1. [Install Rust toolchain](https://www.rust-lang.org/tools/install)
2. Clone this repo: `git clone git@github.com:paxproject/pax.git && cd pax`
3. Run an example: `cargo run --example fireworks`.  See the available examples [here](https://github.com/paxproject/pax/tree/master/examples/src).

## License

© 2024 PaxCorp Inc.  [contact@pax.dev].

This project is licensed under either of:
- [MIT license](LICENSE-MIT)
- [Apache 2.0 License](LICENSE-APACHE)

at your option.

---
# Pax’s Priorities

### Made for Shipping

**Native builds**: Pax compiles to pure machine-code native binaries for each target platform.  Pax today includes native runtimes for macOS desktop, iOS mobile, and browsers via WebAssembly

**Tiny footprint**: <100KB network footprint for WASM builds, enabled by our [unique native compositor](https://docs.pax.dev/reference-native-rendering.html). For native builds we target a <1MB binary footprint.

**Top-tier performance:** 240fps on supporting hardware, achieved in early builds, will be an ongoing priority. GPU rendering [has been achieved](https://github.com/paxproject/pax/pull/76) but requires work for production-readiness (namely: WASM footprint reduction.)

**Accessibility:** Pax supports screen readers on each implemented platform, as well as search engine optimization (SEO) in browsers.  This is enabled by compositing _native elements_ — for example, DOM nodes for the web and SwiftUI elements for macOS/iOS — with virtual canvas drawings, for specific primitives like text and form controls.

### Learnable & Powerful

**Inspired by JSX** — anyone who has used React should find Pax familiar.  The syntax, from the XML base flavor through to curly-brace-wrapped expressions, was designed to echo React.  Nonetheless, by compiling into machine code Pax maintains the power, low overhead, and extensibility of systems programming and the Rust ecosystem.

**Modular & composable** — All Pax components are built around Rust structs and exposed through Rust’s module system, for example across Rust crates.  Pax’s standard library is exposed as components and everything is swappable and extensible.

**Creatively expressive** — Animations, mix & match UI elements with design elements, free-form positioning melded with responsive layouts

**Multiple programming languages** (future) — today we support only Rust for application logic, but plan to extend this to at least TypeScript/Javascript and likely C++, Go, Python, and .NET.

### Designable
Pax’s 100% declarative _user interface description language_ is readable & writable by machines as well as humans.   The language is designed to encode the union of: `anything you might express in a Photoshop file` and `anything you might build in React.`  This includes vector elements, responsive layouts, form controls, and custom, composable components.

Pax the language is just data — declarative markup and expressions.  Turing-complete logic is handled by the accompanying source code (e.g. Rust), where Rust functions can be bound to Pax events, e.g. `<Button @submit=some_rust_function />`, and Pax expressions can refer to Rust struct data like `<Rectangle x={self.dynamic_x_value} />`

Further, Pax’s layout engine renders in “design tool coordinates,” the same coordinate space as a tool like Photoshop or Figma.  You can imagine a visual tool statically opening a Pax codebase, performing visual edits, and persisting those edits as source code — all while offering a Figma-like (or Flash-like) creative experience.

We call this principle “designability.”[1]

Our team is building a commercial, collaborative visual designer for Pax, so people who don’t write code can build software hand-in-hand with those who do. [JOIN THE EARLY ACCESS LIST](https://jeaattwds6e.typeform.com/to/aDG3OH7k)


---

[1] Interestingly, Pax’s prioritization of  machine-editibility appears to cross-pollinate with large language models (LLMs), in addition to visual tooling.  While our experiments are early, Pax is a promising substrate for collaborating on UIs between humans and LLMs.