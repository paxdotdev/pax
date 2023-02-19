# Pax  

Pax is a user interface language and 2D layout engine.  Use Pax to develop cross-platform apps with Rust.

## Example

Writing Pax is intended to feel familiar and the language borrows many ideas from [prior art](https://docs.pax.rs/intro-goals-prior-art.htmll#prior-art--inspiration).

Following is a simple Pax component called `IncrementMe`:

```rust
//File: increment-me.rs

use pax::*;
use pax_std::{Text};
use pax_std::forms::{Button, ArgsButtonSubmit};
use pax_std::layout::{Stacker};

/// The `pax_component` macro loads the file `increment-me.pax` as a template declaration 
/// and defines a reusable component called `IncrementMe`.  You could also declare
/// the Pax definition inline with `#[pax_component_inline(<Stacker>/*...*/</Stacker>)] 
#[pax_component("increment-me.pax")] 
pub struct IncrementMe {
  pub num_clicks: Property<i64>
}
impl IncrementMe {
  pub async fn increment(&self, args: ArgsButtonSubmit) {
    let old_num_clicks = self.num_clicks.get();
    self.num_clicks.set(old_num_clicks + 1);
  }
}
```
```rust
//File: increment-me.pax

<Stacker cells=2>
  <Text>{"I have been clicked " + self.num_clicks + " times."}</Text>
  <Button @submit=self.increment>"Increment me!"</Button>
</Stacker>
```


Any Pax component like the example above may be included inside other Pax components, or may be mounted as the root of a stand-alone app.

See a more thorough, [running example](https://docs.pax.rs/intro-example.html).

## Features

 - **Fast** — low-level native rendering targeting 120FPS animations
 - **Accessible** — supports native screen readers for text & GUI elements, as well as SEO on the Web
 - **Lightweight** — under 100kB baseline for WebAssembly binary 
 - **Declarative** UI language makes it easy to reason about complex scenes and GUIs
 - **Expressive** — includes free-form drawing and animation toolkit alongside GUI form elements and layouts
 - **Reusable component system** includes an optional standard library of GUI components
 - **Cross-platform** — a single Pax definition compiles through Rust into 1. a completely native Mac app (via LLVM, zero Web tech) and 2. a Web app (via WebAssembly). Support for more platforms is planned, at least: Linux, Windows, iOS, and Android.


## Docs
Read more in [The Pax Docs](https://docs.pax.rs/)


## Getting Started
**Pax is being developed in the open in an unstable _alpha preview._** You cannot yet, without pain, create an app.  If you want to collaborate on library development at this stage, [reach out on Discord](https://discord.gg/P6vTntC6fr).

Once Pax reaches `Alpha` — soon — this section will be updated to include simple instructions for getting started.


## Current status & support

Pax is in **alpha-preview** and is not yet viable for building apps — read [the latest status](https://docs.pax.rs/status-sept-2022.html).


#### Support matrix:

|                                         | Web browsers  | Native iOS          | Native Android    | Native macOS        | Native Windows              | Native Linux |
|-----------------------------------------|---------------|---------------------|-------------------|---------------------|-----------------------------|--------------|
| Development harness & chassis           | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| 2D rendering and UIs                    | ✅ <br/>Canvas | ⏲ <br/>CoreGraphics | ⏲ <br/>Cairo      | ✅ <br/>CoreGraphics | ⏲ <br/>Direct2D             | ⏲ <br/>Cairo |
| 3D rendering and UIs                    | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |
| Vector graphics APIs                    | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| 2D layouts                              | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Animation APIs                          | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Native text rendering                   | ✅ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ✅ <br/>SwiftUI      | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native form elements                    | ⏲ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ⏲ <br/>SwiftUI      | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native event handling (e.g. Click, Tap) | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Rust host language                      | ✅ <br/>WASM   | ⏲ <br/>LLVM         | ⏲ <br/>LLVM       | ✅ <br/>LLVM         | ⏲ <br/>LLVM                 | ⏲ <br/>LLVM  |
| JS/TypeScript host language             | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |

| Legend:             |
|---------------------|
| ✅ Supported         |
| ⏲ Not yet supported |


## License

© 2023 PaxCorp Inc.  [contact@pax.rs].

This project is licensed under either of:
- [MIT license](LICENSE-MIT)
- [Apache 2.0 License](LICENSE-APACHE)

at your option.

## Library Development

### Environment setup

Use `rustc` 1.65.0 via `rustup`


### Environment setup, building for Web

- Install `wasm-opt` via `binaryen`:
   ```shell
   brew install binaryen
   ```

- Install 'wasm-pack' via:
   ```shell
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh 
   ```

- Install `node`: https://nodejs.org/en/download/

- Install `yarn`:
   ```shell
  # if necessary: sudo chown -R yourusername /usr/local/lib/node_modules 
  npm i --global yarn
   ```

### Environment setup, building for macOS

- Install xcode, command line utils

### Running Development Environment

First, refer to [the latest project status](https://docs.pax.rs/status-sept-2022.html)

The current leading edge of development is in the compiler —
```
cd pax-compiler
./run-example.sh
```

To run `pax-example`'s parser binary and print its output to stdout:
```
cd pax-example
cargo build --bin=parser --features=parser
```

To run a full, compiled Pax example you must check out an older branch and run the demo there.  
```
git checkout jabberwocky-demo
./run.sh # or ./run-web.sh
```

To initialize the docs submodule, for super-grep powers:

```
git submodule update --init --recursive
```

As needed, review the [git submodule docs.](https://git-scm.com/docs/gitsubmodules)


### Architectural Reference

<img src="runtime-arch.png" />
<div style="text-align: center; font-style: italic;">Runtime dependency & codegen graph</div>
<br /><br /><br />
<img src="compiler-sequence.png" />
<div style="text-align: center; font-style: italic;">
Pax compiler sequence diagram | <a href="https://www.github.com/pax-lang/pax/blob/master/pax-compiler/">Compiler source</a>
</div>
