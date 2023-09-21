# Pax  

Pax is a user interface engine for native apps and websites.

This repository includes:
 - The Pax compiler and language implementation
 - Native runtimes
 - Language bindings for supported programming languages, currently only Rust
 - Pax's CLI for compiling and managing projects 
 - Example projects

## Example

Writing Pax is intended to feel familiar and the language borrows many ideas from [prior art](https://docs.pax.dev/intro-goals-prior-art.html#prior-art--inspiration).

Following is a simple Pax component called `IncrementMe`:

```rust
//File: increment-me.rs

use pax_lang::*;
use pax_std::{Text};
use pax_std::forms::{Button, ArgsButtonSubmit};
use pax_std::layout::{Stacker};

/// Defines the Pax component `IncrementMe`, with template & settings specified in `increment-me.pax`.
#[derive(Pax)]
#[file("increment-me.pax")] 
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
  <Text text={"I have been clicked " + self.num_clicks + " times."}></Text>
  <Button @submit=self.increment>"Increment me!"</Button>
</Stacker>
```

Any Pax component like the example above may be included inside other Pax components, or may be mounted as the root of a stand-alone app.

See a more thorough, [running example](https://docs.pax.dev/intro-example.html).


## Features

 - **Fast** — 240fps animations, compiles to machine code
 - **Native** — Platform-native text, UI controls, and drawing APIs.
 - **Accessible** — Supports screen readers for native text & GUI elements, as well as SEO on the Web
 - **Lightweight** — <100kB footprint baseline for WebAssembly binary. No web tech in desktop & mobile builds. 
 - **Declarative** UI language makes it easy to reason about complex scenes and GUIs, as well as build tooling that reads & writes Pax
 - **Expressive** — Free-form drawing and animation toolkit, plus GUI form elements and responsive layouts.  Blur the lines between work & play, function & art.
 - **Extensible** — UI component system built around Rust structs enables modular application building and publication of reusable components through cargo and crates.io.  Pax's standard library (`pax-std`) is a canonical example, publishing modular primitives like `<Group />`, drawing elements like `<Rectangle />`, form elements like `<Text />`, and layout elements like `<Stacker />`.


## Docs
Read more in [The Pax Docs](https://docs.pax.dev/)


## Getting Started
**Pax is being developed in the open in an unstable _alpha preview._** You cannot yet, without pain, create an app.  If you want to collaborate on library development at this stage, [reach out on Discord](https://discord.gg/P6vTntC6fr).


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

© 2023 PaxCorp Inc.  [contact@pax.dev].

This project is licensed under either of:
- [MIT license](LICENSE-MIT)
- [Apache 2.0 License](LICENSE-APACHE)

at your option.

## Library Development

### Environment setup

Use `rustc` 1.70.0 via `rustup`


### To build Pax projects for WebAssembly

- Install 'wasm-pack' via:
   ```shell
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh 
   ```

- Install `node` v20 LTS, recommended via [`nvm`](https://github.com/nvm-sh/nvm#installing-and-updating)
  ```shell
  # First install nvm
  curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.3/install.sh | bash
  # After restarting terminal:
  nvm install 20
  nvm use 20 --default
  ```

### To build Pax projects as native macOS apps

- Install xcode `>=14.3` and Xcode command line utils: `xcode-select --install`
- SDK Version `macosx13.3`, Xcode version `>=14.3`
- Current Minimum Deployment `13.0`


### Running Development Environment

First, refer to [the latest project status](https://docs.pax.dev/status-sept-2022.html)

Run `pax-example`:

```shell
# after cloning pax, from `pax/`
cd pax-example
# the `./pax` shell script emulates the `pax` CLI for Pax monorepo development
./pax run --target=macos # or --target=web 
```

To initialize the submodules, for super-grep powers:

```
git submodule update --init --recursive
```

As needed, review the [git submodule docs.](https://git-scm.com/docs/gitsubmodules)
