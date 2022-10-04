# Pax  

Pax is a cross-platform rendering engine & Rust framework for interactive graphics, animations, and GUIs.

Pax extends the Rust programming language with a syntax for declarative component-based graphical content and behavior. Pax programs compile through Rust into lightweight native app executables or WebAssembly-powered Web apps with a <100kB base footprint and up to 120FPS rendering.

As of September 2022, Pax is being developed in the open, [in alpha preview](https://docs.pax-lang.org/status-sept-2022.html).


## Example

Following is a simple Pax component called `IncrementMe`:

```rust
//increment-me.rs

use pax::*;
use pax_std::{Text};
use pax_std::forms::{Button, ArgsButtonSubmit};
use pax_std::layout::{Stacker};

#[pax(
  <Stacker cells=2>
    <Text>{"I have been clicked " + self.num_clicks + " times."}</Text>
    <Button @submit=self.increment>"Increment me!"</Button>
  </Stacker>
)] 
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
Any Pax component like the example above may be included inside other Pax components, or may be mounted as the root of a stand-alone app.

See a more thorough and [running example in the docs](https://docs.pax-lang.org/intro-example.html).


## Docs
Read more in [The Pax Docs](https://docs.pax-lang.org/)


## Getting Started

Refer to the [guide for getting started](https://docs.pax-lang.org/start-creating-a-project.html) in the Pax docs.

## Current status & support

Pax is in **alpha-preview** and is not yet viable for building apps — read [the latest status](https://docs.pax-lang.org/status-sept-2022.html).

#### End-to-end:
 - Syntax design  ✅
 - Parser  ✅
 - Compiler front-end ❌ WIP
 - Compiler back-end ✅
 - Rendering engine  ✅
 - Cross-platform chassis  ✅
 - Browser runtime  ✅
 - macOS runtime  ✅


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

© 2022 The Pax Foundation [contact@pax-lang.org].

This project is licensed under either of:
- [MIT license](LICENSE-MIT)
- [Apache 2.0 License](LICENSE-APACHE)

at your option.

## Development

### Architectural Reference 

<img src="runtime-arch.png" />
<div style="text-align: center; font-style: italic;">Runtime dependency & codegen graph</div>
<br /><br /><br />
<img src="compiler-sequence.png" />
<div style="text-align: center; font-style: italic;">
Pax compiler sequence diagram | <a href="https://www.github.com/pax-lang/pax/blob/master/pax-compiler/">Compiler source</a>
</div>.  

### Optional environment setup, web chassis

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

### Optional environment setup, macOS chassis

- Install xcode, command line utils

### Running Development Environment

First, refer to [the latest project status](https://docs.pax-lang.org/status-sept-2022.html)

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
