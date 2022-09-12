# Pax

Pax is a Rust-based language for interactive graphics & GUIs. As of September 2022, Pax is in [alpha preview](https://docs.pax-lang.org/status-sept-2022.html).

Pax works as a companion language to Rust. Here's a simple example:

```rust
#[pax(                              //rust
    <Text>"Hello, world!"</Text>    //pax
)]                                  //rust
pub struct HelloWorld {}            //rust
```

In addition to static content like the example above, Pax supports [high-performance](https://docs.pax-lang.org/intro-goals.html) 2D drawing, expressions, animations, composable responsive layouts, and form controls for GUIs.

The Pax compiler outputs platform-specific application executables, for example .apps for macOS or .wasm-powered webpages for browsers. Today, Pax supports only Web and macOS, though it is planned to extend to: iOS, Android, Windows, and Linux.

**Read more in [The Pax Docs](https://docs.pax-lang.org/)**

## Getting Started

Refer to the [guide for getting started](https://docs.pax-lang.org/start-creating-a-project.html) in the Pax docs.

## Current status & support

**Pax is pre-release** — Read [the lastest status](https://docs.pax-lang.org/status-sept-2022.html).

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
| 2D rendering and UIs [2]                | ✅ <br/>Canvas | ⏲ <br/>CoreGraphics | ⏲ <br/>Cairo      | ✅ <br/>CoreGraphics | ⏲ <br/>Direct2D             | ⏲ <br/>Cairo |
| 3D rendering and UIs                    | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |
| Vector graphics APIs                    | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| 2D layouts                              | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Animation APIs                          | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Native text rendering                   | ✅ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ✅ <br/>SwiftUI      | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native form elements                    | ⏲ <br/>DOM    | ⏲ <br/>UIKit        | ⏲ <br/>android:\* | ⏲ <br/>SwiftUI      | ⏲ <br/>System.Windows.Forms | ⏲ <br/>GTK   |
| Native event handling (e.g. Click, Tap) | ✅             | ⏲                   | ⏲                 | ✅                   | ⏲                           | ⏲            |
| Rust as host language                   | ✅ <br/>WASM   | ⏲ <br/>LLVM         | ⏲ <br/>LLVM       | ✅ <br/>LLVM         | ⏲ <br/>LLVM                 | ⏲ <br/>LLVM  |
| JS/TypeScript as host language          | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |
| C++/Python/etc. as host languages       | ⏲             | ⏲                   | ⏲                 | ⏲                   | ⏲                           | ⏲            |

| Legend:             |
|---------------------|
| ✅ Supported         |
| ⏲ Not yet supported |


## Development

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


