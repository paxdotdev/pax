# Pax 

Design and build user interfaces that run anywhere.

Pax is two things that work together: (1) a vector design tool and (2) a cross-platform user interface engine.

(1) **Pax Designer** is a vector design tool that reads & writes user interface definitions as code.

(2) **Pax Engine** is a user interface engine: a cross-platform, fast, and lightweight toolkit for building native apps & websites.

All built in Rust 🦀🦀

## Get Started

Follow the [Get Started](https://docs.pax.dev/get-started/) instructions in the docs.

## Features

* **Integrated visual builder (Pax Designer)** — a vector design tool and reads & writes code with every visual operation
* **Cross-platform** Build WASM apps or native macOS / iOS apps (macOS and iOS targets are in Alpha; Web target is in Beta; Windows, Linux, and Android are planned.)
* **Responsive layout engine** (top-down,) including % and px as first-class citizens
* **Standard library of reusable components** like form controls, layouts, and drawing primitives
* **Robust text**, including accessibility / screen-reader support and SEO support for web builds
* **Animation engine**: every property of every element is animatable at up to 240FPS, giving an extremely high ceiling for creative expression — intended for microinteractions, data visualization, interactive cartoons, games, simulations, or whatever else you can imagine.
* **Expression language**: every property can be bound to spreadsheet-inspired expressions; this makes dynamic logic accessible to low-coders, offers a highly expressive & succinct substrate for LLM generation, and is part of our solution   
* **Lightweight footprint**, targeting 100KB baseline WASM network footprint (current status: 2-3x above target, with room to improve)


## Examples

You can try out Pax Designer on your workstation by following the [“Get Started”]((https://docs.pax.dev/get-started/)) directions.  
This will run Pax Designer and allow you to make changes to the template starter project visually and via code.

For a robust real-world project built in Pax, see [Pax Designer's source code](https://github.com/paxdotdev/pax/tree/dev/pax-designer), which is 100% Pax.

## Docs

Read the docs at [https://docs.pax.dev/](https://docs.pax.dev) or contribute to [the docs repo on GitHub](https://github.com/paxdotdev/docs).


## Project status

**Current status: Beta**

This milestone includes complete open source releases of the systems comprising Pax, including Pax Engine, Pax Designer, and the Pax Standard Library.

You can build a real-world app with Pax today — see [pax-designer](https://github.com/paxengine/pax/tree/dev/pax-designer) for an example that’s already shipping.

**Expect some rough edges with Beta:**

1. Missing vector design tool features — ([jump on our Discord](https://discord.com/invite/Eq8KWAUc6b) to share ideas & requests!)
2. Bugs — we appreciate any reports you can file as [Github Issues](https://github.com/paxdotdev/pax/issues).
3. Breaking changes — we do our best to avoid breaking changes, and are held accountable through maintaining a significant Pax app ([Pax Designer](https://github.com/paxdotdev/pax/tree/dev/pax-designer)).  That said, breaking changes are subject to occur any time before 1.0.
4. Web target is leading edge — macos and ios build targets are maintained for architectural soundness, but are several features behind the web target, e.g. occlusion and clipping.  We expect to continue prioritizing Web target development for the near term.  For mobile / desktop targets at this milestone, we recommend wrapping Pax Web with a webview e.g. Tauri.


## Current priorities

 - **Hosted version of Pax Designer** — so anyone can use Pax Designer in the browser without any terminals or code.  This will also be the chassis for Pax Pro, our commercial collaboration service that makes it easy for non-developers to contribute visual changes to GitHub.

 - **Pax JavaScript** — bindings to JavaScript so you can write Pax with JavaScript instead of Rust

 - **Responses to feedback** & general functional & ergonomic improvements

Our task tracker is private (Linear) but we are open to ideas for alternate solutions that can solve both productivity and visibility.

We collaborate publicly on the [#contribution](https://discord.com/invite/Eq8KWAUc6b) channel of our community Discord — feel free to [drop in and chat.](https://discord.com/invite/Eq8KWAUc6b)


## Contribution

Pax is open source and we welcome contributions.  See [CONTRIBUTING.md](CONTRIBUTING.md)


## Why?

Pax aims to make software creation more creative and more accessible to humanity. Learn more about Pax and our goals in [our docs](https://docs.pax.dev/).

To achieve these goals, Pax is designed for ["designability"](https://docs.pax.dev/reference/designability/) — an ongoing bilateral bridge between visual vector design and user interface definitions as code.

Pax also unlocks a new way to interact with AI — a visual builder that an LLM navigates natively, because language is the backbone of every visual operation.  Pax lets you design AND code with an LLM, and it can design and code in response.  We believe this is a glimpse of the future of building user interfaces and we're working hard to bring it to the world.


## License

© 2024 PaxCorp Inc.  [contact@pax.dev]

This project is licensed under either of:
- [MIT license](LICENSE-MIT)
- [Apache 2.0 License](LICENSE-APACHE)

at your option.


