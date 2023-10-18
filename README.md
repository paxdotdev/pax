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
//File: lib.rs
use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::*;
use pax_std::types::*;
use pax_std::types::text::*;
use pax_std::components::Stacker;

/// Defines the Pax component `IncrementMe`, with template & settings specified in `increment-me.pax`.
#[derive(Pax)]
#[main]
#[file("increment-me.pax")]
pub struct IncrementMe {
    pub num_clicks: Property<u32>,
    pub message: Property<String>,
}

impl IncrementMe {
    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
        self.num_clicks.set(0);
        self.message.set("0 clicks".to_string());
    }
    pub fn increment(&mut self, ctx: RuntimeContext, args: ArgsClick){
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
        self.message.set(format!("{} clicks", self.num_clicks.get()));
    }

} 
```
```rust
//increment-me.pax
<Text text={self.message} class=centered id=text class=centered />
<Rectangle class=centered class=small @click=self.increment
    fill={Fill::Solid(Color::rgba(0.0,0.0,0.0,1.0))} 
    corner_radii={RectangleCornerRadii::radii(10.0,10.0,10.0,10.0)}
/>

@handlers{
     did_mount:handle_did_mount
}

@settings {
     .centered {
        x: 50%
        y: 50%
        anchor_x: 50%
        anchor_y: 50%
    } 
    .small {
        width: 120px
        height: 120px
    }
    #text {
        style: {
                font: {Font::system("Times New Roman", FontStyle::Normal, FontWeight::Bold)},
                font_size: 32px,
                fill: {Color::rgba(1.0, 1.0, 1.0, 1.0)},
                align_vertical: TextAlignVertical::Center,
                align_horizontal: TextAlignHorizontal::Center,
                align_multiline: TextAlignHorizontal::Center
        }
    }
}
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

### Setup, macOS workstation

 - Install `rustc` 1.70.0 via `rustup`
 - Install the Pax CLI: `cargo install pax-cli`
 - Follow instructions to build for [WebAssembly](#to-build-pax-projects-for-webassembly) or [macOS](#to-build-pax-projects-as-native-macos-apps) below
 - Create a new project `pax-cli new my-first-project`

### Setup, Linux (Debian / Ubuntu) workstation

 - Install `rustc` 1.70.0 via `rustup`
 - Install development dependencies: `apt install pkg-config libssl-dev`
 - Install the Pax CLI: `cargo install pax-cli`
 - Follow instructions to build for [WebAssembly](#to-build-pax-projects-for-webassembly) below
 - Create a new project `pax-cli new my-first-project`

### Setup, Windows workstation

 - Install `rustc` via installer
 - Install the Pax CLI: `cargo install pax-cli`
 - Follow instructions to build for [WebAssembly](#to-build-pax-projects-for-webassembly) below
 - Create a new project `pax-cli new my-first-project`

### To build Pax projects for WebAssembly

- Install 'wasm-pack' via:
   ```shell
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh 
   ```
  For Windows, follow instructions to use installer [here.](https://rustwasm.github.io/wasm-pack/installer/)

- Install `node` v20 LTS, recommended via [`nvm`](https://github.com/nvm-sh/nvm#installing-and-updating)
  ```shell
  # For macOS / Linux:  first install nvm
  curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.3/install.sh | bash
  # After restarting terminal:
  nvm install 20
  nvm use 20 --default
  ```
  For Windows, install [`nvm-windows`](https://github.com/coreybutler/nvm-windows) and install Node v20 LTS.

### To build Pax projects as native macOS apps

- Building macOS apps requires running a Mac with macOS.  This is a constraint enforced technically and legally by Apple.
- Install xcode `>=15.0` and Xcode command line utils: `xcode-select --install`
- SDK Version `macosx13.3`, Xcode version `>=15.0`
- Current Minimum Deployment `13.0`
- Install all necessary build architectures for Rust, so that binaries can be built for both Intel and Apple Silicon macs
  ```
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  ```

### To build Pax projects as native iOS apps

- Follow instructions for building native macOS apps, above
- Install all necessary build architectures for Rust, so that binaries can be built for iOS and simulator targets:
  ```
  rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
  ```

#### Support matrix:

|                                         | Web browsers  | Native iOS          | Native Android    | Native macOS        | Native Windows              | Native Linux |
|-----------------------------------------|---------------|---------------------|-------------------|---------------------|-----------------------------|--------------|
| Development harness & chassis           | ✅             | ✅                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| 2D rendering and UIs                    | ✅ <br/>Canvas | ✅ <br/>CoreGraphics | ⏲ <br/>Cairo      | ✅ <br/>CoreGraphics | ⏲ <br/>Direct2D             | ⏲ <br/>Cairo |
| 3D rendering and UIs                    | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |
| Vector graphics APIs                    | ✅             | ✅                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| 2D layouts                              | ✅             | ✅                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Animation APIs                          | ✅             | ✅                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Native text rendering                   | ✅ <br/>DOM    | ✅ <br/>SwiftUI      | ⏲ <br/>android:\* | ✅ <br/>SwiftUI      | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native form elements                    | ⏲ <br/>DOM    | ⏲ <br/>SwiftUI      | ⏲ <br/>android:\* | ⏲ <br/>SwiftUI      | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native event handling (e.g. Click, Tap) | ✅             | ✅                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Rust host language                      | ✅ <br/>WASM   | ✅ <br/>LLVM         | ⏲ <br/>LLVM       | ✅ <br/>LLVM         | ⏲ <br/>LLVM                 | ⏲ <br/>LLVM  |
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


### Running Monorepo Development Environment

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
