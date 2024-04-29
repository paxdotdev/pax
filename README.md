# Pax Engine

Build extremely fast user interfaces that run anywhere.

Write application logic in Rust (or TypeScript, coming soon) — declare your user interface in Pax's user interface description language.

Pax compiles into native desktop/mobile apps, WebAssembly-driven sites, and embeddable universal UI components.

## What's in this repo?

 - The Pax compiler and an implementation of the Pax user interface description language
 - Native renderers + runtimes for iOS, macOS, and browsers/WebAssembly
 - Layout and animation engine for responsive positioning and expressive user interactions
 - Language bindings for supported programming languages: Rust is supported today; TypeScript is coming soon.  If you want support for another host programming language, please file an issue.
 - Pax's CLI for compiling and managing projects
 - Pax's standard library of reusable UI components like `Text`, `TextBox`, and `Button`; vector drawing primitives like `Rectangle`, `Path`, and `Group`; responsive layouts via `Stacker`, clipping via `Frame`, and scrolling via `Scroller`.
 - Example projects

## Status

**Currently Alpha** and unstable, under active full-time development.  Today, Pax compiles and runs for iOS, macOS, and Web.  The Web target is our leading edge of development.  Pax’s standard library includes components for drawing, layouts, and form controls.

[Take Pax for a spin!](https://docs.pax.dev/start-creating-a-project.html)

[Join our Discord](https://discord.com/invite/Eq8KWAUc6b) to chat with our team.  

We do not yet recommend building any production workloads with Pax. Targeting Beta in Q3 2024.

Embedded universal components have been proven in concept but adapters for React, Next, Vue, SwiftUI, etc. have not been built.  If you are interested in a particular component adapter, please open an issue so that we can understand your use-case & prioritize accordingly.

## Examples

See a **simple example** in the [introduction page](https://docs.pax.dev/) of our docs.

You can also run the [examples](https://github.com/paxproject/pax/tree/master/examples/src) in this monorepo:

1. [Install Rust toolchain](https://www.rust-lang.org/tools/install)
2. Clone this repo: `git clone git@github.com:paxproject/pax.git && cd pax`
3. Run an example: `cargo run --example fireworks`.  See the available examples [here](https://github.com/paxproject/pax/tree/master/examples/src).


## Docs

Read the docs at [https://docs.pax.dev/](https://docs.pax.dev)

## License

© 2024 PaxCorp Inc.  [contact@pax.dev].

This project is licensed under either of:
- [MIT license](LICENSE-MIT)
- [Apache 2.0 License](LICENSE-APACHE)

at your option.

## Build Pax visually

[Get early access to Pax Create](https://airtable.com/appCUQtUS9g4kuQZL/pagcoNLd0e8amZB0D/form)

![image](https://github.com/paxengine/pax/assets/2100885/972fd339-868d-4718-8e07-aabc26d6945c)



