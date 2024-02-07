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

**Currently Alpha** and unstable, under active full-time development.  Today, Pax compiles and runs for iOS, macOS, and Web.  The Web target is our leading edge of development.  Pax’s standard library includes components for drawing, layouts, and form controls.

[Take Pax for a spin!](https://docs.pax.dev/start-creating-a-project.html)

[Join our Discord](https://discord.com/invite/Eq8KWAUc6b) to chat with our team.  

We do not yet recommend building any production workloads with Pax. Targeting Beta in Q3 2024.


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
